#![cfg_attr(not(feature = "std"), no_std)]

#[frame_support::pallet]
#[allow(dead_code)]
pub mod pallet {
    use sp_arithmetic::{traits::SaturatedConversion, Percent};
    use sp_runtime::DispatchError;
    use sp_std::{default::Default, marker::PhantomData, vec::Vec};

    #[cfg(feature = "std")]
    use frame_support::traits::GenesisBuild;
    use frame_support::{
        dispatch::DispatchResultWithPostInfo,
        ensure,
        storage::types::{StorageMap, StorageValue, ValueQuery},
        traits::{Get, Hooks, IsType},
        Twox64Concat,
    };
    use frame_system::{
        ensure_root, ensure_signed,
        pallet_prelude::{BlockNumberFor, OriginFor},
    };

    use chainx_primitives::AssetId;

    use crate::pallet::{self as xbridge, BalanceOf};
    use crate::vault::pallet as vault;

    type IssueRequest<T> = crate::types::IssueRequest<
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
    pub trait Config: frame_system::Config + xbridge::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
        type TargetAssetId: Get<AssetId>;
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// User request issue xbtc
        ///
        /// `IssueRequest` couldn't be submitted while bridge during liquidating.
        #[pallet::weight(0)]
        pub fn request_issue(
            origin: OriginFor<T>,
            vault_id: T::AccountId,
            btc_amount: BalanceOf<T>,
            collateral: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {
            xbridge::Pallet::<T>::ensure_bridge_running()?;

            let sender = ensure_signed(origin)?;
            let height = <frame_system::Pallet<T>>::block_number();
            let vault = xbridge::Pallet::<T>::get_active_vault_by_id(&vault_id)?;
            let vault_collateral = xbridge::Pallet::<T>::reserved_balance_of(&vault_id);

            // check if vault is rich enough
            let collateral_ratio_after_requesting =
                xbridge::Pallet::<T>::calculate_collateral_ratio(
                    vault.issued_tokens + vault.to_be_issued_tokens + btc_amount,
                    vault_collateral,
                )?;
            ensure!(
                collateral_ratio_after_requesting >= xbridge::Pallet::<T>::secure_threshold(),
                Error::<T>::InsecureVault
            );

            let required_collateral = Self::calculate_required_collateral(btc_amount)?;
            ensure!(
                collateral >= required_collateral,
                Error::<T>::InsufficientGriefingCollateral
            );

            // insert `IssueRequest` to request map
            xbridge::Pallet::<T>::lock_collateral(&sender, collateral)?;
            let request_id = Self::get_next_request_id();
            Self::insert_issue_request(
                request_id,
                IssueRequest::<T> {
                    vault: vault.id.clone(),
                    open_time: height,
                    requester: sender,
                    btc_address: vault.wallet,
                    btc_amount,
                    griefing_collateral: collateral,
                    ..Default::default()
                },
            );
            xbridge::Vaults::<T>::mutate(&vault.id, |vault| {
                if let Some(vault) = vault {
                    vault.to_be_issued_tokens += btc_amount;
                }
            });
            Self::deposit_event(Event::<T>::IssueRequestSubmitted);
            Ok(().into())
        }

        #[pallet::weight(0)]
        pub fn execute_issue(
            origin: OriginFor<T>,
            request_id: RequestId,
            _tx_id: Vec<u8>,
            _merkle_proof: Vec<u8>,
            _raw_tx: Vec<u8>,
        ) -> DispatchResultWithPostInfo {
            xbridge::Pallet::<T>::ensure_bridge_running()?;

            let _sender = ensure_signed(origin)?;

            //TODO(wangyafei): verify tx

            let issue_request = Self::get_issue_request_by_id(request_id)
                .ok_or(Error::<T>::IssueRequestNotFound)?;

            ensure!(
                !issue_request.completed && !issue_request.cancelled,
                Error::<T>::IssueRequestDealt
            );

            let height = frame_system::Pallet::<T>::block_number();
            ensure!(
                height - issue_request.open_time < Self::issue_request_expired_time(),
                Error::<T>::IssueRequestExpired
            );

            <xpallet_assets::Module<T>>::issue(
                &T::TargetAssetId::get(),
                &issue_request.requester,
                issue_request.btc_amount,
            )?;
            xbridge::Vaults::<T>::mutate(&issue_request.vault, |vault| {
                if let Some(vault) = vault {
                    vault.to_be_issued_tokens -= issue_request.btc_amount;
                    vault.issued_tokens += issue_request.btc_amount;
                }
            });
            xbridge::Pallet::<T>::release_collateral(
                &issue_request.requester,
                issue_request.griefing_collateral,
            )?;

            IssueRequests::<T>::mutate(request_id, |issue_request| {
                if let Some(issue_request) = issue_request {
                    issue_request.completed = true
                }
            });
            Self::deposit_event(Event::<T>::IssueRequestExcuted);
            Ok(().into())
        }

        #[pallet::weight(0)]
        pub fn cancel_issue(
            origin: OriginFor<T>,
            request_id: RequestId,
        ) -> DispatchResultWithPostInfo {
            ensure_signed(origin)?;

            let issue_request = Self::get_issue_request_by_id(request_id)
                .ok_or(Error::<T>::IssueRequestNotFound)?;
            ensure!(
                !issue_request.completed && !issue_request.cancelled,
                Error::<T>::IssueRequestDealt
            );

            let height = <frame_system::Pallet<T>>::block_number();
            let expired_time = <IssueRequestExpiredTime<T>>::get();
            ensure!(
                height - issue_request.open_time > expired_time,
                Error::<T>::IssueRequestNotExpired
            );

            let slashed_collateral = Self::calculate_slashed_collateral(issue_request.btc_amount)?;

            <xbridge::Pallet<T>>::slash_collateral(
                &issue_request.vault,
                &issue_request.requester,
                slashed_collateral,
            )?;

            <xbridge::Pallet<T>>::release_collateral(
                &issue_request.requester,
                issue_request.griefing_collateral,
            )?;

            xbridge::Vaults::<T>::mutate(&issue_request.vault, |vault| {
                if let Some(vault) = vault {
                    vault.to_be_issued_tokens -= issue_request.btc_amount;
                }
            });

            IssueRequests::<T>::mutate(request_id, |issue_request| {
                if let Some(issue_request) = issue_request {
                    issue_request.cancelled = true
                }
            });

            Self::deposit_event(Event::<T>::IssueRequestCancelled);
            Ok(().into())
        }

        /// Update expired time for requesting issue
        #[pallet::weight(0)]
        pub fn update_expired_time(
            origin: OriginFor<T>,
            expired_time: BlockNumberFor<T>,
        ) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;
            <IssueRequestExpiredTime<T>>::put(expired_time);
            Self::deposit_event(Event::<T>::ExpiredTimeUpdated);
            Ok(().into())
        }

        /// Update griefing fee for requesting issue
        #[pallet::weight(0)]
        pub fn update_griefing_fee(
            origin: OriginFor<T>,
            griefing_fee: Percent,
        ) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;
            <IssueGriefingFee<T>>::put(griefing_fee);
            Self::deposit_event(Event::<T>::GriefingFeeUpdated);
            Ok(().into())
        }
    }

    /// Error for issue module
    #[pallet::error]
    pub enum Error<T> {
        /// Collateral in request is less than griefing collateral
        InsufficientGriefingCollateral,
        /// No such `IssueRequest`
        IssueRequestNotFound,
        /// `IssueRequest` cancelled when it's not expired
        IssueRequestNotExpired,
        /// Value to be set is invalid
        InvalidConfigValue,
        /// Tried to execute `IssueRequest` while  it's expired
        IssueRequestExpired,
        /// Vault colateral ratio was below than `SecureThreshold`
        InsecureVault,
        /// `IssueRequest` has been excuted or cancelled
        IssueRequestDealt,
    }

    /// Events for issue module
    #[pallet::event]
    #[pallet::generate_deposit(pub(crate) fn deposit_event)]
    pub enum Event<T: Config> {
        // TODO(wangyafei)
        // An issue request was submitted and waiting user to excute.
        IssueRequestSubmitted,
        // `IssueRequest` excuted.
        IssueRequestExcuted,
        // `IssueRequest` cancelled.`
        IssueRequestCancelled,
        // Root updated `IssueRequestExpiredTime`.
        ExpiredTimeUpdated,
        // Root updated `IssueGriefingFee`.
        GriefingFeeUpdated,
    }

    /// Percentage to lock, when user requests issue
    #[pallet::storage]
    #[pallet::getter(fn issue_griefing_fee)]
    pub(crate) type IssueGriefingFee<T: Config> = StorageValue<_, Percent, ValueQuery>;

    /// Auto-increament id to identify each issue request.
    /// Also presents total amount of created requests.
    #[pallet::storage]
    pub(crate) type RequestCount<T: Config> = StorageValue<_, RequestId, ValueQuery>;

    /// Mapping from issue id to `IssueRequest`
    #[pallet::storage]
    pub(crate) type IssueRequests<T: Config> =
        StorageMap<_, Twox64Concat, RequestId, IssueRequest<T>>;

    /// Expired time for an `IssueRequest`
    #[pallet::storage]
    #[pallet::getter(fn issue_request_expired_time)]
    pub(crate) type IssueRequestExpiredTime<T: Config> =
        StorageValue<_, BlockNumberFor<T>, ValueQuery>;

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        pub(crate) issue_griefing_fee: Percent,
        pub(crate) expired_time: BlockNumberFor<T>,
    }

    #[cfg(feature = "std")]
    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            Self {
                issue_griefing_fee: Default::default(),
                expired_time: Default::default(),
            }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
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

        /// Calculate minimium required collateral for a `IssueRequest`
        pub(crate) fn calculate_required_collateral(
            btc_amount: BalanceOf<T>,
        ) -> Result<BalanceOf<T>, DispatchError> {
            let pcx_amount = <xbridge::Pallet<T>>::convert_to_pcx(btc_amount)?;
            let percentage = Self::issue_griefing_fee();
            let griefing_fee = percentage.mul_ceil(pcx_amount);
            Ok(griefing_fee)
        }

        /// Get `IssueRequest` from id
        pub(crate) fn get_issue_request_by_id(request_id: RequestId) -> Option<IssueRequest<T>> {
            <IssueRequests<T>>::get(request_id)
        }

        /// Calculate slashed amount.
        ///
        /// Equals the corresponding pcx times secure threshold
        pub(crate) fn calculate_slashed_collateral(
            btc_amount: BalanceOf<T>,
        ) -> Result<BalanceOf<T>, DispatchError> {
            let pcx_amount = xbridge::Pallet::<T>::convert_to_pcx(btc_amount)?;
            let secure_threshold = xbridge::Pallet::<T>::secure_threshold();
            let slashed_collateral: u32 =
                (pcx_amount.saturated_into::<u128>() * secure_threshold as u128 / 100) as u32;
            Ok(slashed_collateral.into())
        }
    }
}
