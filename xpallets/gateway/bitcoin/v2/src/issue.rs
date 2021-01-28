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
        pub(crate) btc_amount: XBTC,
        /// Collateral locked to avoid user griefing
        pub(crate) griefing_collateral: PCX,
    }
}

#[frame_support::pallet]
#[allow(dead_code)]
pub mod pallet {
    use sp_arithmetic::Percent;
    use sp_runtime::DispatchError;
    use sp_std::marker::PhantomData;

    #[cfg(feature = "std")]
    use frame_support::traits::GenesisBuild;
    use frame_support::{
        dispatch::DispatchResultWithPostInfo,
        ensure,
        storage::types::{StorageMap, StorageValue, ValueQuery},
        traits::Hooks,
        Twox64Concat,
    };
    use frame_system::{
        ensure_none, ensure_signed,
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

    type RequestId = u128;

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
            btc_amount: BalanceOf<T>,
            collateral: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {
            //FIXME(wangyafei): break if bridge in liquidation mode.
            let sender = ensure_signed(origin)?;
            let height = <frame_system::Pallet<T>>::block_number();
            let vault = vault::Pallet::<T>::get_active_vault_by_id(&vault_id)?;
            let required_collateral = Self::calculate_required_collateral(btc_amount)?;
            ensure!(
                collateral >= required_collateral,
                Error::<T>::InsufficientGriefingCollateral
            );
            assets::Pallet::<T>::lock_collateral(&sender, collateral)?;
            let request_id = Self::get_next_request_id();
            Self::insert_issue_request(
                request_id,
                IssueRequest::<T> {
                    vault: vault.id,
                    opentime: height,
                    requester: sender,
                    btc_address: vault.wallet,
                    btc_amount,
                    griefing_collateral: collateral,
                    ..Default::default()
                },
            );
            Ok(().into())
        }

        #[pallet::weight(0)]
        pub fn cancel_issue(
            origin: OriginFor<T>,
            issue_id: RequestId,
        ) -> DispatchResultWithPostInfo {
            ensure_none(origin)?;
            let issue =
                Self::get_issue_request_by_id(issue_id).ok_or(Error::<T>::IssueRequestNotFound)?;
            let height = <frame_system::Pallet<T>>::block_number();
            //TODO(wangyafei): move it to genesis_config
            let expired_time = 10;
            ensure!(
                height - issue.opentime > expired_time.into(),
                Error::<T>::IssueRequestNotExpired
            );
            // <assets::Pallet<T>>::slash_collateral(issue.requester, issue.vault)?
            Ok(().into())
        }
    }

    /// Error for issue module
    #[pallet::error]
    pub enum Error<T> {
        /// Collateral in request is less than griefing collateral.
        InsufficientGriefingCollateral,
        /// No such `IssueRequest`
        IssueRequestNotFound,
        /// `IssueRequest` cancelled when it's not expired.
        IssueRequestNotExpired,
    }

    /// Percentage to lock, when user requests issue
    #[pallet::storage]
    #[pallet::getter(fn issue_griefing_fee)]
    pub(crate) type IssueGriefingFee<T: Config> = StorageValue<_, u8, ValueQuery>;

    /// Auto-increament id to identify each issue request.
    /// Also presents total amount of created requests.
    #[pallet::storage]
    pub(crate) type RequestCount<T: Config> = StorageValue<_, RequestId, ValueQuery>;

    /// Mapping from issue id to `IssueRequest`
    #[pallet::storage]
    pub(crate) type IssueRequests<T: Config> =
        StorageMap<_, Twox64Concat, RequestId, IssueRequest<T>>;

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
        pub(crate) fn get_next_request_id() -> RequestId {
            <RequestCount<T>>::mutate(|n| {
                *n += 1;
                *n
            })
        }

        /// Calculated minimium required collateral for a `IssueRequest`
        pub(crate) fn calculate_required_collateral(
            btc_amount: BalanceOf<T>,
        ) -> Result<BalanceOf<T>, DispatchError> {
            let pcx_amount = <assets::Pallet<T>>::convert_to_pcx(btc_amount)?;
            let percentage = Self::issue_griefing_fee();
            let griefing_fee = Percent::from_parts(percentage).mul_ceil(pcx_amount);
            Ok(griefing_fee)
        }

        /// Get `IssueRequest` from id
        pub(crate) fn get_issue_request_by_id(issue_id: RequestId) -> Option<IssueRequest<T>> {
            <IssueRequests<T>>::get(issue_id)
        }
    }
}
