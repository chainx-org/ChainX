#![cfg_attr(not(feature = "std"), no_std)]

pub mod types {
    use codec::HasCompact;
    use frame_support::pallet_prelude::{Decode, Encode};

    pub type BtcAddress = Vec<u8>;

    #[derive(Encode, Decode, Clone, PartialEq)]
    #[cfg_attr(feature = "std", derive(Debug))]
    pub enum VaultStatus {
        Active,
        Liquidated,
        CommittedTheft,
    }

    impl Default for VaultStatus {
        fn default() -> Self {
            VaultStatus::Active
        }
    }

    #[derive(Encode, Decode, Default, Clone, PartialEq)]
    #[cfg_attr(feature = "std", derive(Debug))]
    pub struct Vault<AccountId, BlockNumber, XBTC> {
        // Account identifier of the Vault
        pub id: AccountId,
        // Number of XBTC tokens pending issue
        pub to_be_issued_tokens: XBTC,
        // Number of issued XBTC tokens
        pub issued_tokens: XBTC,
        // Number of XBTC tokens pending redeem
        pub to_be_redeemed_tokens: XBTC,
        // Bitcoin address of this Vault (P2PKH, P2SH, P2PKH, P2WSH)
        pub wallet: BtcAddress,
        // Block height until which this Vault is banned from being
        // used for Issue, Redeem (except during automatic liquidation) and Replace .
        pub banned_until: Option<BlockNumber>,
        /// Current status of the vault
        pub status: VaultStatus,
    }

    impl<AccountId, BlockNumber, XBTC: HasCompact + Default> Vault<AccountId, BlockNumber, XBTC> {
        pub(super) fn new(id: AccountId, address: BtcAddress) -> Self {
            Self {
                id,
                to_be_issued_tokens: Default::default(),
                issued_tokens: Default::default(),
                to_be_redeemed_tokens: Default::default(),
                wallet: address,
                banned_until: None,
                status: VaultStatus::default(),
            }
        }
    }
}

#[frame_support::pallet]
pub mod pallet {
    use super::types::*;
    #[cfg(feature = "std")]
    pub use frame_support::traits::GenesisBuild;
    use frame_support::Blake2_128Concat;
    use frame_support::{
        pallet_prelude::*,
        storage::types::{StorageMap, StorageValue, ValueQuery},
    };
    use frame_support::{
        traits::{Currency, ReservableCurrency},
        Twox64Concat,
    };
    use frame_system::pallet_prelude::{ensure_signed, BlockNumberFor, OriginFor};

    use sp_runtime::DispatchResult;

    pub type PCX<T> =
        <<T as Config>::PCX as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    pub type Token<T> =
        <<T as Config>::Token as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type PCX: Currency<Self::AccountId> + ReservableCurrency<Self::AccountId>;
        type Token: Currency<Self::AccountId> + ReservableCurrency<Self::AccountId>;
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Register a vault.
        #[pallet::weight(0)]
        fn register_vault(
            origin: OriginFor<T>,
            collateral: PCX<T>,
            btc_address: BtcAddress,
        ) -> DispatchResultWithPostInfo {
            let sender = ensure_signed(origin)?;
            ensure!(
                collateral >= Self::minimium_vault_collateral(),
                Error::<T>::InsufficientVaultCollateralAmount
            );
            ensure!(!Self::vault_exists(&sender), Error::<T>::VaultRegistered);
            ensure!(
                !Self::btc_address_exists(&btc_address),
                Error::<T>::BtcAddressOccupied
            );
            Self::lock_collateral(&sender, collateral)?;
            Self::insert_btc_address(&btc_address, sender.clone());
            let vault = Vault::new(sender.clone(), btc_address);
            Self::insert_vault(&sender, vault.clone());
            Self::deposit_event(Event::RegisterVault(vault.id, collateral));
            Ok(().into())
        }
    }

    #[pallet::error]
    pub enum Error<T> {
        InsufficientFunds,
        InsufficientVaultCollateralAmount,
        VaultRegistered,
        BtcAddressOccupied,
    }

    #[pallet::event]
    // Additional argument to specify the metadata to use for given type.
    #[pallet::metadata(BalanceOf<T> = "Balance", u32 = "Other")]
    // Generate a funciton on Pallet to deposit an event.
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        RegisterVault(<T as frame_system::Config>::AccountId, PCX<T>),
    }

    #[pallet::type_value]
    pub(super) fn zero_pcx<T: Config>() -> PCX<T> {
        0.into()
    }

    /// Total collateral.
    #[pallet::storage]
    #[pallet::getter(fn total_collateral)]
    pub(super) type TotalCollateral<T: Config> = StorageValue<_, PCX<T>, ValueQuery, zero_pcx<T>>;

    #[pallet::storage]
    pub(super) type Vaults<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        Vault<T::AccountId, T::BlockNumber, Token<T>>,
    >;

    #[pallet::storage]
    pub(super) type BtcAddresses<T: Config> = StorageMap<_, Twox64Concat, BtcAddress, T::AccountId>;

    #[pallet::storage]
    #[pallet::getter(fn minimium_vault_collateral)]
    pub(super) type MinimiumVaultCollateral<T: Config> =
        StorageValue<_, PCX<T>, ValueQuery, zero_pcx<T>>;

    #[pallet::genesis_config]
    #[derive(Default)]
    pub struct GenesisConfig {
        _minimium_vault_collateral: u32,
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig {
        fn build(&self) {
            let pcx: PCX<T> = self._minimium_vault_collateral.into();
            <MinimiumVaultCollateral<T>>::put(pcx);
        }
    }
    impl<T: Config> Pallet<T> {
        /// Lock collateral
        fn lock_collateral(sender: &T::AccountId, amount: PCX<T>) -> DispatchResult {
            T::PCX::reserve(sender, amount).map_err(|_| Error::<T>::InsufficientFunds)?;
            Ok(())
        }

        /// increase total collateral
        fn increase_total_collateral(amount: PCX<T>) {
            let new_collateral = Self::total_collateral() + amount;
            <TotalCollateral<T>>::put(new_collateral);
        }

        fn insert_vault(
            sender: &T::AccountId,
            vault: Vault<T::AccountId, T::BlockNumber, Token<T>>,
        ) {
            <Vaults<T>>::insert(sender, vault);
        }

        fn insert_btc_address(address: &BtcAddress, vault_id: T::AccountId) {
            <BtcAddresses<T>>::insert(address, vault_id);
        }

        fn vault_exists(id: &T::AccountId) -> bool {
            <Vaults<T>>::contains_key(id)
        }

        fn btc_address_exists(address: &BtcAddress) -> bool {
            <BtcAddresses<T>>::contains_key(address)
        }
    }
}
