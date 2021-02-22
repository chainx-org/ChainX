#![cfg_attr(not(feature = "std"), no_std)]

pub mod types {
    use sp_std::{default::Default, prelude::Vec};

    use codec::{Decode, Encode};
    use light_bitcoin::keys::Address;

    pub type BtcAddress = Address;

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
    pub struct SystemVault<AccountId, Balance> {
        pub(crate) id: AccountId,
        pub(crate) to_be_issued_tokens: Balance,
        pub(crate) issued_tokens: Balance,
        pub(crate) to_be_redeemed_tokens: Balance,
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
    use sp_std::{collections::btree_set::BTreeSet, default::Default};

    use frame_support::pallet_prelude::*;
    use frame_support::traits::ReservableCurrency;
    use frame_system::pallet_prelude::{ensure_signed, BlockNumberFor, OriginFor};

    use super::types::*;
    use crate::assets::pallet as assets;
    use assets::BalanceOf;

    type DefaultVault<T: Config> = Vault<T::AccountId, BlockNumberFor<T>, BalanceOf<T>>;

    #[pallet::config]
    pub trait Config: frame_system::Config + assets::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(crate) trait Store)]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_finalize(_: BlockNumberFor<T>) {
            if assets::Pallet::<T>::is_bridge_running() {
                for (id, vault) in <Vaults<T>>::iter() {
                    if Self::_check_vault_liquidated(&vault) {
                        let _ = Self::liquidate_vault(&id);
                    }
                }
            }
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Register a vault.
        #[pallet::weight(0)]
        pub(crate) fn register_vault(
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
            <assets::Pallet<T>>::lock_collateral(&sender, collateral)?;
            <assets::Pallet<T>>::increase_total_collateral(collateral);
            Self::insert_btc_address(&btc_address, sender.clone());
            let vault = Vault::new(sender.clone(), btc_address);
            Self::insert_vault(&sender, vault.clone());
            Self::deposit_event(Event::VaultRegistered(vault.id, collateral));
            Ok(().into())
        }

        /// Add extra collateral for registered vault.
        #[pallet::weight(0)]
        pub(crate) fn add_extra_collateral(
            origin: OriginFor<T>,
            collateral: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {
            let sender = ensure_signed(origin)?;
            ensure!(Self::vault_exists(&sender), Error::<T>::VaultNotFound);
            <assets::Pallet<T>>::lock_collateral(&sender, collateral)?;
            <assets::Pallet<T>>::increase_total_collateral(collateral);
            Self::deposit_event(Event::ExtraCollateralAdded(sender, collateral));
            Ok(().into())
        }
    }

    /// Error during register, withdrawing collateral or adding extra collateral
    #[pallet::error]
    pub enum Error<T> {
        /// The amount in request is less than lower bound.
        InsufficientVaultCollateralAmount,
        /// Collateral is less than lower bound after extrinsic.
        InsufficientVaultCollateral,
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
        /// New vault has been registered.
        VaultRegistered(<T as frame_system::Config>::AccountId, BalanceOf<T>),
        /// Extra collateral was added to a vault.
        ExtraCollateralAdded(<T as frame_system::Config>::AccountId, BalanceOf<T>),
        /// Vault released collateral.
        CollateralReleased(<T as frame_system::Config>::AccountId, BalanceOf<T>),
    }

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

    /// Secure threshold for vault
    /// eg, 200 means 200%.
    #[pallet::storage]
    #[pallet::getter(fn secure_threshold)]
    pub(crate) type SecureThreshold<T: Config> = StorageValue<_, u16, ValueQuery>;

    /// Secure threshold for vault
    /// eg, 150 means 150%.
    #[pallet::storage]
    #[pallet::getter(fn premium_threshold)]
    pub(crate) type PremiumThreshold<T: Config> = StorageValue<_, u16, ValueQuery>;

    /// Secure threshold for vault.
    /// eg, 100 means 100%.
    #[pallet::storage]
    #[pallet::getter(fn liquidation_threshold)]
    pub(crate) type LiquidationThreshold<T: Config> = StorageValue<_, u16, ValueQuery>;

    /// Specicial `SystemVault`
    #[pallet::storage]
    #[pallet::getter(fn liquidator)]
    pub(crate) type Liquidator<T: Config> =
        StorageValue<_, SystemVault<T::AccountId, BalanceOf<T>>, ValueQuery>;

    /// Liquidator account id
    #[pallet::storage]
    #[pallet::getter(fn liquidator_id)]
    pub(crate) type LiquidatorId<T: Config> = StorageValue<_, T::AccountId, ValueQuery>;

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        pub(crate) minimium_vault_collateral: u32,
        pub(crate) secure_threshold: u16,
        pub(crate) premium_threshold: u16,
        pub(crate) liquidation_threshold: u16,
        pub(crate) liquidator_id: T::AccountId,
    }

    #[cfg(feature = "std")]
    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            Self {
                minimium_vault_collateral: Default::default(),
                secure_threshold: 180,
                premium_threshold: 250,
                liquidation_threshold: 300,
                liquidator_id: Default::default(),
            }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
        fn build(&self) {
            let pcx: BalanceOf<T> = self.minimium_vault_collateral.into();
            <MinimiumVaultCollateral<T>>::put(pcx);
            <SecureThreshold<T>>::put(self.secure_threshold);
            <PremiumThreshold<T>>::put(self.premium_threshold);
            <LiquidationThreshold<T>>::put(self.liquidation_threshold);
            <Liquidator<T>>::put(SystemVault {
                id: self.liquidator_id.clone(),
                ..Default::default()
            });
            <LiquidatorId<T>>::put(self.liquidator_id.clone());
        }
    }

    impl<T: Config> Pallet<T> {
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

        /// Liquidate vault and mark it as `Liquidated`
        ///
        /// Liquidated vault cannot be updated.
        pub(crate) fn liquidate_vault(id: &T::AccountId) -> Result<(), DispatchError> {
            <Vaults<T>>::mutate(id, |vault| match vault {
                Some(ref mut vault) => {
                    if vault.status == VaultStatus::Active {
                        vault.status = VaultStatus::Liquidated;
                        Ok(())
                    } else {
                        Err(Error::<T>::VaultInactive)
                    }
                }
                None => Err(Error::<T>::VaultNotFound),
            })?;

            let vault = Self::get_vault_by_id(id)?;
            let collateral = <assets::CurrencyOf<T>>::reserved_balance(&vault.id);
            <assets::Pallet<T>>::slash_collateral(&vault.id, &Self::liquidator_id(), collateral)?;

            <Liquidator<T>>::mutate(|liquidator| {
                liquidator.issued_tokens += vault.issued_tokens;
                liquidator.to_be_issued_tokens += vault.to_be_issued_tokens;
                liquidator.to_be_redeemed_tokens += vault.to_be_redeemed_tokens;
            });
            Ok(())
        }

        pub(crate) fn _check_vault_liquidated(vault: &DefaultVault<T>) -> bool {
            if vault.issued_tokens == 0.into() {
                return false;
            }
            let collateral = <assets::CurrencyOf<T>>::reserved_balance(&vault.id);
            <assets::Pallet<T>>::calculate_collateral_ratio(vault.issued_tokens, collateral)
                .map(|collateral_ratio| collateral_ratio < Self::liquidation_threshold())
                .unwrap_or(false)
        }
    }
}
