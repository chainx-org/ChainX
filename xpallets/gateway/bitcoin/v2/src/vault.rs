#![cfg_attr(not(feature = "std"), no_std)]

pub mod types {
    use codec::{Decode, Encode};
    use sp_std::prelude::Vec;

    pub type BtcAddress = Vec<u8>;

    #[derive(Encode, Decode, Clone, PartialEq)]
    #[cfg_attr(feature = "std", derive(Debug))]
    pub enum VaultStatus {
        /// Vault is ready to serve issue and redeem request, unless it was banned.
        Active,
        /// Vault is under Liquidation
        Liquidated,
        /// Vault was committed has illegal behavior.
        CommittedTheft,
    }

    impl Default for VaultStatus {
        fn default() -> Self {
            VaultStatus::Active
        }
    }

    #[derive(Encode, Decode, Default, Clone, PartialEq)]
    #[cfg_attr(feature = "std", derive(Debug))]
    pub struct Vault<AccountId, BlockNumber, Balance> {
        /// Account identifier of the Vault
        pub id: AccountId,
        /// Number of tokens pending issue
        pub to_be_issued_tokens: Balance,
        /// Number of issued tokens
        pub issued_tokens: Balance,
        /// Number of tokens pending redeem
        pub to_be_redeemed_tokens: Balance,
        /// Bitcoin address of this Vault (P2PKH, P2SH, P2PKH, P2WSH)
        pub wallet: BtcAddress,
        /// Block height until which this Vault is banned from being
        /// used for Issue, Redeem (except during automatic liquidation) and Replace .
        pub banned_until: Option<BlockNumber>,
        /// Current status of the vault
        pub status: VaultStatus,
    }

    impl<AccountId: Default, BlockNumber: Default, Balance: Default>
        Vault<AccountId, BlockNumber, Balance>
    {
        pub(crate) fn new(id: AccountId, address: BtcAddress) -> Self {
            Self {
                id,
                wallet: address,
                ..Default::default()
            }
        }
    }
}

#[frame_support::pallet]
#[allow(dead_code)]
pub mod pallet {
    use frame_support::{
        pallet_prelude::*,
        traits::{Currency, ReservableCurrency},
    };
    use frame_system::pallet_prelude::{ensure_signed, BlockNumberFor, OriginFor};
    use sp_runtime::DispatchResult;

    use super::types::*;

    pub type BalanceOf<T> =
        <<T as Config>::PCX as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type PCX: ReservableCurrency<Self::AccountId>;
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(crate) trait Store)]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Register a vault.
        #[pallet::weight(0)]
        pub fn register_vault(
            origin: OriginFor<T>,
            collateral: BalanceOf<T>,
            btc_address: BtcAddress,
        ) -> DispatchResultWithPostInfo {
            let sender = ensure_signed(origin)?;
            ensure!(
                collateral >= Self::minimium_vault_collateral(),
                Error::<T>::InsufficientVaultCollateralAmount
            );
            ensure!(
                !Self::vault_exists(&sender),
                Error::<T>::VaultAlreadyRegistered
            );
            ensure!(
                !Self::btc_address_exists(&btc_address),
                Error::<T>::BtcAddressOccupied
            );
            Self::lock_collateral(&sender, collateral)?;
            Self::increase_total_collateral(collateral);
            Self::insert_btc_address(&btc_address, sender.clone());
            let vault = Vault::new(sender.clone(), btc_address);
            Self::insert_vault(&sender, vault.clone());
            Self::deposit_event(Event::VaultRegistered(vault.id, collateral));
            Ok(().into())
        }
    }

    /// Error during register, withdrawing collateral or adding extra collateral
    #[pallet::error]
    pub enum Error<T> {
        /// Requester doesn't has enough pcx for collateral.
        InsufficientFunds,
        /// The amount in request is less than minimium bound.
        InsufficientVaultCollateralAmount,
        /// Requester has been vault.
        VaultAlreadyRegistered,
        /// Btc address in request was occupied by another vault.
        BtcAddressOccupied,
        /// Vault does not exist.
        VaultNotFound,
        /// Vault was inactive
        VaultInactive,
    }

    /// Event during register, withdrawing collateral or adding extra collateral
    #[pallet::event]
    #[pallet::generate_deposit(pub(crate) fn deposit_event)]
    pub enum Event<T: Config> {
        /// When a new vault has been registered.
        VaultRegistered(<T as frame_system::Config>::AccountId, BalanceOf<T>),
    }

    /// Total collateral.
    #[pallet::storage]
    #[pallet::getter(fn total_collateral)]
    pub(crate) type TotalCollateral<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

    /// Mapping account to vault struct.
    #[pallet::storage]
    pub(crate) type Vaults<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        Vault<T::AccountId, T::BlockNumber, BalanceOf<T>>,
    >;

    /// Mapping btc address to vault id.
    #[pallet::storage]
    pub(crate) type BtcAddresses<T: Config> = StorageMap<_, Twox64Concat, BtcAddress, T::AccountId>;

    /// Lower bound for registering vault or withdrawing collateral.
    #[pallet::storage]
    #[pallet::getter(fn minimium_vault_collateral)]
    pub(crate) type MinimiumVaultCollateral<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

    #[pallet::genesis_config]
    #[derive(Default)]
    pub struct GenesisConfig {
        pub(crate) minimium_vault_collateral: u32,
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig {
        fn build(&self) {
            let pcx: BalanceOf<T> = self.minimium_vault_collateral.into();
            <MinimiumVaultCollateral<T>>::put(pcx);
        }
    }

    impl<T: Config> Pallet<T> {
        /// Lock collateral
        #[inline]
        pub fn lock_collateral(sender: &T::AccountId, amount: BalanceOf<T>) -> DispatchResult {
            T::PCX::reserve(sender, amount).map_err(|_| Error::<T>::InsufficientFunds)?;
            Ok(())
        }

        /// increase total collateral
        #[inline]
        pub fn increase_total_collateral(amount: BalanceOf<T>) {
            <TotalCollateral<T>>::mutate(|c| *c += amount);
        }

        #[inline]
        pub fn insert_vault(
            sender: &T::AccountId,
            vault: Vault<T::AccountId, T::BlockNumber, BalanceOf<T>>,
        ) {
            <Vaults<T>>::insert(sender, vault);
        }

        #[inline]
        pub fn insert_btc_address(address: &BtcAddress, vault_id: T::AccountId) {
            <BtcAddresses<T>>::insert(address, vault_id);
        }

        #[inline]
        pub fn vault_exists(id: &T::AccountId) -> bool {
            <Vaults<T>>::contains_key(id)
        }

        #[inline]
        pub fn btc_address_exists(address: &BtcAddress) -> bool {
            <BtcAddresses<T>>::contains_key(address)
        }

        pub fn get_vault_by_id(
            id: &T::AccountId,
        ) -> Result<Vault<T::AccountId, T::BlockNumber, BalanceOf<T>>, DispatchError> {
            match <Vaults<T>>::get(id) {
                Some(vault) => Ok(vault),
                None => Err(Error::<T>::VaultNotFound.into()),
            }
        }

        pub fn get_active_vault_by_id(
            id: &T::AccountId,
        ) -> Result<Vault<T::AccountId, T::BlockNumber, BalanceOf<T>>, DispatchError> {
            let vault = Self::get_vault_by_id(id)?;
            if vault.status == VaultStatus::Active {
                Ok(vault)
            } else {
                Err(Error::<T>::VaultInactive.into())
            }
        }
    }
}
