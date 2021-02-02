#![cfg_attr(not(feature = "std"), no_std)]

pub mod types {
    use codec::{Decode, Encode};
    use sp_std::vec::Vec;

    pub type BtcAddress = Vec<u8>;

    /// redeem request status
    #[derive(Encode, Decode, PartialEq, Eq)]
    #[cfg_attr(feature = "std", derive(Debug))]
    pub enum RedeemRequestStatus {
        /// waiting for vault transfer btc to user btc address
        WaitForGetBtc,

        /// redeem is cancled by user
        Cancled,

        /// redeem is completed
        Completed,
    }

    impl Default for RedeemRequestStatus {
        fn default() -> Self {
            RedeemRequestStatus::WaitForGetBtc
        }
    }

    #[derive(Encode, Decode, Default, PartialEq, Eq)]
    #[cfg_attr(feature = "std", derive(Debug))]
    pub struct RedeemRequest<AccountId, BlockNumber, XBTC, PCX> {
        /// Vault id
        pub(crate) vault: AccountId,
        /// Block height when the redeem requested
        pub(crate) open_time: BlockNumber,
        /// Who requests redeem
        pub(crate) requester: AccountId,
        /// Vault's btc address
        pub(crate) btc_address: BtcAddress,
        /// Amount that user wants to redeem
        pub(crate) amount: XBTC,
        /// redeem fee amount
        pub(crate) redeem_fee: PCX,
        /// Request status
        pub(crate) status: RedeemRequestStatus,
    }
}

#[frame_support::pallet]
#[allow(dead_code)]
pub mod pallet {

    use frame_support::{
        dispatch::{DispatchResult, DispatchResultWithPostInfo},
        ensure,
        storage::types::{StorageMap, StorageValue, ValueQuery},
        traits::{Hooks, IsType},
        Twox64Concat,
    };
    use frame_system::{
        ensure_root, ensure_signed,
        pallet_prelude::{BlockNumberFor, OriginFor},
    };
    use light_bitcoin::chain::Transaction;
    use sp_std::{marker::PhantomData, vec::Vec};

    // import vault,issue,assets code.
    use crate::assets::{
        pallet as assets,
        pallet::BalanceOf,
        pallet::BridgeStatus,
        types::{ErrorCode, Status},
    };
    use crate::issue::pallet as issue;
    use crate::vault::pallet as vault;

    type RedeemRequest<T> = super::types::RedeemRequest<
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
    pub trait Config: frame_system::Config + issue::Config + xpallet_assets::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
    }

    /// Events for redeem module
    #[pallet::event]
    #[pallet::generate_deposit(pub(crate) fn deposit_event)]
    pub enum Event<T: Config> {
        /// current chain status is not right
        ChainStatusError,
        /// redeem request is accepted
        RedeemRequestIsAccepted,
        /// cancle redeem is accepted
        CancleRedeemIsAccepted,
        /// force redeem is accepted
        ForceRedeemIsAccepted,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::error]
    pub enum Error<T> {
        /// Redeem amount exceeds user has
        AmoundExceedsAvaliable,
        /// Bridge has multiple errors
        BridgeInComlicatedError,
        /// Bridge shutdown
        BridgeShutdown,
        /// redeem request id is not exsit
        RedeemRequestNotFound,

        /// `RedeemRequest` cancelled for forced redeem when it's not expired.
        RedeemRequestNotExpired,

        /// Vault is under Liquidation
        ValtLiquidated,

        /// redeem amount is too much
        InsufficiantAssetsFonds,

        /// redeem is completed
        CancleRedeemErrOfCompleted,

        /// redeem is cancled
        CancleRedeemErrOfCancled,
    }

    /// redeem fee when use request redeem
    #[pallet::storage]
    #[pallet::getter(fn redeem_fee)]
    pub(crate) type RedeemFee<T: Config> = StorageValue<_, u8, ValueQuery>;

    /// Auto-increament id to identify each issue request.
    /// Also presents total amount of created requests.
    #[pallet::storage]
    pub(crate) type RequestCount<T: Config> = StorageValue<_, RequestId, ValueQuery>;

    /// Mapping from redeem id to `RedeemRequest`
    #[pallet::storage]
    pub(crate) type RedeemRequests<T: Config> =
        StorageMap<_, Twox64Concat, RequestId, RedeemRequest<T>>;

    /// Expired time for an `RedeemRequest`
    #[pallet::storage]
    pub(crate) type RedeemRequestExpiredTime<T: Config> =
        StorageValue<_, BlockNumberFor<T>, ValueQuery>;

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// User request redeem
        #[pallet::weight(0)]
        pub fn request_redeem(
            origin: OriginFor<T>,
            vault_id: T::AccountId,
            redeem_amount: BalanceOf<T>,
            btc_addr: super::types::BtcAddress,
        ) -> DispatchResultWithPostInfo {
            let sender = ensure_signed(origin)?;
            Self::ensure_bridge_running_or_error_liquidated()?;

            let btc_balances = xpallet_assets::Module::<T>::usable_balance(&sender, &1);
            ensure!(
                btc_balances >= redeem_amount,
                Error::<T>::AmoundExceedsAvaliable
            );

            let vault = vault::Pallet::<T>::get_active_vault_by_id(&vault_id)?;
            // generate redeem request identify
            let request_id = Self::get_next_request_id();
            let height = <frame_system::Pallet<T>>::block_number();

            let (btc_amount, pcx_amount) = {
                //TODO(wangyafei): partial redeem when liquidating.
                (redeem_amount, 0)
            };

            <RedeemRequests<T>>::insert(
                request_id,
                RedeemRequest::<T> {
                    vault: vault_id,
                    open_time: height,
                    requester: sender,
                    btc_address: btc_addr,
                    amount: btc_amount,
                    redeem_fee: Default::default(),
                    status: super::types::RedeemRequestStatus::WaitForGetBtc,
                },
            );

            // send msg to user
            // Self::deposit_event(Event::<T>::RedeemRequestIsAccepted);

            Ok(().into())
        }

        #[pallet::weight(0)]
        pub fn execute_redeem(
            origin: OriginFor<T>,
            request_id: RequestId,
            _tx_id: Vec<u8>,
            _merkle_proof: Vec<u8>,
            _raw_tx: Transaction,
        ) -> DispatchResultWithPostInfo {
            let _sender = ensure_signed(origin)?;
            //TODO verify tx
            let request =
                <RedeemRequests<T>>::get(request_id).ok_or(Error::<T>::RedeemRequestNotFound)?;
            // decrase user's XBTC amount.
            xpallet_assets::Module::<T>::destroy_reserved_withdrawal(
                &1,
                &request.requester,
                request.amount,
            )?;

            Ok(().into())
        }

        /// user cancle redeem
        #[pallet::weight(0)]
        pub fn cancle_redeem(
            origin: OriginFor<T>,
            request_id: RequestId,
        ) -> DispatchResultWithPostInfo {
            let sender = ensure_signed(origin)?;
            let height = <frame_system::Pallet<T>>::block_number();
            let request =
                <RedeemRequests<T>>::get(request_id).ok_or(Error::<T>::RedeemRequestNotFound)?;
            let expired_time = <RedeemRequestExpiredTime<T>>::get();
            ensure!(
                height - request.open_time > expired_time,
                Error::<T>::RedeemRequestNotExpired
            );
            ensure!(
                request.status != super::types::RedeemRequestStatus::Cancled,
                Error::<T>::CancleRedeemErrOfCancled
            );
            ensure!(
                request.status != super::types::RedeemRequestStatus::Completed,
                Error::<T>::CancleRedeemErrOfCompleted
            );

            // send msg to user
            Self::deposit_event(Event::<T>::CancleRedeemIsAccepted);

            Ok(().into())
        }

        /// user force redeem. when user do this means he can get pcx only.
        #[pallet::weight(0)]
        pub fn force_redeem(
            origin: OriginFor<T>,
            request_id: RequestId,
        ) -> DispatchResultWithPostInfo {
            let sender = ensure_signed(origin)?;
            let height = <frame_system::Pallet<T>>::block_number();
            let request =
                <RedeemRequests<T>>::get(request_id).ok_or(Error::<T>::RedeemRequestNotFound)?;
            let expired_time = <RedeemRequestExpiredTime<T>>::get();
            ensure!(
                height - request.open_time > expired_time,
                Error::<T>::RedeemRequestNotExpired
            );
            ensure!(
                request.status != super::types::RedeemRequestStatus::Cancled,
                Error::<T>::CancleRedeemErrOfCancled
            );
            ensure!(
                request.status != super::types::RedeemRequestStatus::Completed,
                Error::<T>::CancleRedeemErrOfCompleted
            );

            // catulate user's XBTC worth how much pcx, then give he the pcx
            let worth_pcx = assets::Pallet::<T>::convert_to_pcx(request.amount)?;

            // add user pcx amount of worth_pcx.then we should sub vault pcx
            assets::Pallet::<T>::slash_collateral(&request.vault, &sender, worth_pcx);

            // notice(how to give phone msg or mail msg?) the vault that the user's force redeem action

            // send msg to user
            Self::deposit_event(Event::<T>::ForceRedeemIsAccepted);

            Ok(().into())
        }

        /// Update expired time for requesting redeem
        #[pallet::weight(0)]
        pub fn update_expired_time(
            origin: OriginFor<T>,
            expired_time: BlockNumberFor<T>,
        ) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;
            <RedeemRequestExpiredTime<T>>::put(expired_time);
            Ok(().into())
        }
    }

    impl<T: Config> Pallet<T> {
        /// generate secure key from account id
        pub(crate) fn get_next_request_id() -> RequestId {
            <RequestCount<T>>::mutate(|n| {
                *n += 1;
                *n
            })
        }

        /// Get `IssueRequest` from id
        pub(crate) fn get_redeem_request_by_id(request_id: RequestId) -> Option<RedeemRequest<T>> {
            <RedeemRequests<T>>::get(request_id)
        }

        pub(crate) fn ensure_bridge_running_or_error_liquidated() -> DispatchResult {
            let status = assets::Pallet::<T>::bridge_status();
            match status {
                Status::Running => Ok(()),
                Status::Error => {
                    let error_codes: Vec<_> = assets::Pallet::<T>::bridge_error_codes();
                    for error_code in error_codes {
                        if error_code != ErrorCode::Liquidating {
                            return Err(Error::<T>::BridgeInComlicatedError.into());
                        }
                    }
                    Ok(().into())
                }
                Status::Shutdown => Err(Error::<T>::BridgeShutdown.into()),
            }
        }
    }
}
