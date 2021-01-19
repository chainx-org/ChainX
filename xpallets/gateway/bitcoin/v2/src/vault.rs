#![cfg_attr(not(feature = "std"), no_std)]

#[frame_support::pallet]
pub mod vault {
    use codec::HasCompact;
    use frame_support::traits::{Currency, GenesisBuild, ReservableCurrency};
    use frame_support::Blake2_128Concat;
    use frame_support::{
        pallet_prelude::*,
        storage::types::{StorageMap, StorageValue, ValueQuery},
    };
    use frame_system::pallet_prelude::{ensure_signed, BlockNumberFor, OriginFor};

    use sp_runtime::DispatchResult;
    use v1::BtcAddress;

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
        fn new(id: AccountId, address: BtcAddress) -> Self {
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
    pub type PCX<T> =
        <<T as Config>::PCX as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    pub type Token<T> =
        <<T as Config>::Token as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type PCX: Currency<Self::AccountId> + ReservableCurrency<Self::AccountId>;
        type Token: Currency<Self::AccountId> + ReservableCurrency<Self::AccountId>;
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
            Self::lock_collateral(&sender, collateral)?;
            let vault = Vault::new(sender.clone(), btc_address);
            Self::insert_vault(&sender, vault.clone());
            Self::deposit_event(vault.id, collateral);
            //TODO(wangyafei)
            Ok(().into())
        }
    }

    #[pallet::error]
    pub enum Error<T> {
        InsufficientFunds,
        InsufficientVaultCollateralAmount,
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
    #[pallet::getter(fn vaults)]
    pub(super) type Vaults<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        Vault<T::AccountId, T::BlockNumber, Token<T>>,
    >;

    #[pallet::storage]
    #[pallet::getter(fn minimium_vault_collateral)]
    pub(super) type MinimiumVaultCollateral<T: Config> =
        StorageValue<_, PCX<T>, ValueQuery, zero_pcx<T>>;

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        _minimium_vault_collateral: PCX<T>,
    }

    /// Default value for GenesisConfig
    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            Self {
                _minimium_vault_collateral: 0.into(),
            }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
        fn build(&self) {
            <MinimiumVaultCollateral<T>>::put(self._minimium_vault_collateral);
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
    }
}
