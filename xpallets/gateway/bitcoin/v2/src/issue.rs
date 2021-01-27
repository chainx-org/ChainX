#![cfg_attr(not(feature = "std"), no_std)]

pub mod types {
    use sp_std::vec::Vec;

    use codec::{Decode, Encode};

    #[cfg(feature = "std")]
    use frame_support::{Deserialize, Serialize};

    pub(crate) type BtcAddress = Vec<u8>;

    /// Contains all informations while executing a issue request needed.
    #[derive(Encode, Decode, Default, Clone, PartialEq)]
    #[cfg_attr(feature = "std", derive(Debug, Deserialize, Serialize))]
    pub struct IssueRequest<AccountId, BlockNumber, XBTC, PCX> {
        /// Vault id
        pub(crate) vault: AccountId,
        /// Block height when the issue requested
        pub(crate) opentime: BlockNumber,
        /// Who requests issue
        pub(crate) requester: AccountId,
        /// Vault's btc address
        pub(crate) btc_address: BtcAddress,
        /// Wheather request finished
        pub(crate) completed: bool,
        /// Wheather request cancelled
        pub(crate) cancelled: bool,
        /// Amount that user wants to issue
        pub(crate) amount: XBTC,
        /// Collateral locked to avoid user griefing
        pub(crate) griefing_collateral: PCX,
    }
}

#[frame_support::pallet]
#[allow(dead_code)]
pub mod pallet {
    use sp_arithmetic::{traits::SaturatedConversion, Percent};
    use sp_runtime::DispatchError;
    use sp_std::{convert::TryInto, marker::PhantomData};

    #[cfg(feature = "std")]
    use frame_support::traits::GenesisBuild;
    use frame_support::{
        dispatch::DispatchResultWithPostInfo,
        ensure,
        storage::types::{StorageMap, StorageValue, ValueQuery},
        traits::{Currency, Hooks, ReservableCurrency},
        Twox64Concat,
    };
    use frame_system::{
        ensure_signed,
        pallet_prelude::{BlockNumberFor, OriginFor},
    };

    use crate::assets::{pallet as assets, pallet::BalanceOf};
    use crate::vault::pallet as vault;

    type IssueRequest<T> = super::types::IssueRequest<
        <T as frame_system::Config>::AccountId,
        <T as frame_system::Config>::BlockNumber,
        BalanceOf<T>,
        BalanceOf<T>,
    >;

    #[pallet::pallet]
    #[pallet::generate_store(pub(crate) trait Store)]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::config]
    pub trait Config: frame_system::Config + vault::Config {}

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// user request issue xbtc
        #[pallet::weight(0)]
        pub fn request_issue(
            origin: OriginFor<T>,
            vault_id: T::AccountId,
            amount: BalanceOf<T>,
            collateral: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {
            //FIXME(wangyafei): break if bridge in liquidation mode.
            let sender = ensure_signed(origin)?;
            let height = <frame_system::Pallet<T>>::block_number();
            let vault = vault::Pallet::<T>::get_active_vault_by_id(&vault_id)?;
            let required_collateral = Self::get_required_collateral_from_btc_amount(amount)?;
            ensure!(
                collateral >= required_collateral,
                Error::<T>::InsufficientGriefingCollateral
            );
            assets::Pallet::<T>::lock_collateral(&sender, collateral)?;
            let key = Self::get_request_id();
            Self::insert_issue_request(
                key,
                IssueRequest::<T> {
                    vault: vault.id,
                    opentime: height,
                    requester: sender,
                    btc_address: vault.wallet,
                    amount,
                    griefing_collateral: collateral,
                    ..Default::default()
                },
            );
            Ok(().into())
        }
    }

    /// Error for issue module
    #[pallet::error]
    pub enum Error<T> {
        /// Collateral in request is less than griefing collateral.
        InsufficientGriefingCollateral,
    }

    /// Percentage to lock, when user requests issue
    #[pallet::storage]
    #[pallet::getter(fn issue_griefing_fee)]
    pub(crate) type IssueGriefingFee<T: Config> = StorageValue<_, u8, ValueQuery>;

    /// Auto-increament id to identity each request
    #[pallet::storage]
    pub(crate) type RequestId<T: Config> = StorageValue<_, u128, ValueQuery>;

    /// Mapping from issue id to `IssueRequest`
    #[pallet::storage]
    pub(crate) type IssueRequests<T: Config> = StorageMap<_, Twox64Concat, u128, IssueRequest<T>>;

    /// Genesis configure
    #[pallet::genesis_config]
    #[derive(Default)]
    pub struct GenesisConfig {
        /// fee rate for user to request issue. It's locked till the request done or cancelled.
        pub issue_griefing_fee: u8,
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig {
        fn build(&self) {
            <IssueGriefingFee<T>>::put(self.issue_griefing_fee);
        }
    }

    impl<T: Config> Pallet<T> {
        pub(crate) fn insert_issue_request(key: u128, value: IssueRequest<T>) {
            <IssueRequests<T>>::insert(&key, value)
        }

        /// generate secure key from account id
        pub(crate) fn get_request_id() -> u128 {
            let nonce = <RequestId<T>>::mutate(|n| {
                *n += 1;
                *n
            }); //auto increament
            nonce
        }

        /// Calculated minimium required collateral for a `IssueRequest`
        pub(crate) fn get_required_collateral_from_btc_amount(
            amount: BalanceOf<T>,
        ) -> Result<BalanceOf<T>, DispatchError> {
            let raw_amount = <assets::Pallet<T>>::convert_to_pcx(amount)?;
            let percentage = Self::issue_griefing_fee();
            let griefing_fee = Percent::from_parts(percentage).mul_ceil(raw_amount);
            Ok(griefing_fee)
        }
    }
}
