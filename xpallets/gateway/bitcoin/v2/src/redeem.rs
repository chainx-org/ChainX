#![cfg_attr(not(feature = "std"), no_std)]

pub mod types {
    use codec::{Decode, Encode};
    use sp_std::vec::Vec;
    pub type BtcAddress = Vec<u8>;

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
        pub(crate) completed: bool,
        pub(crate) cancelled: bool,
        pub(crate) reimburse: bool,
    }
}

#[frame_support::pallet]
#[allow(dead_code)]
pub mod pallet {
    use frame_support::{
        dispatch::DispatchResultWithPostInfo,
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
    use sp_std::marker::PhantomData;
    use sp_std::vec::Vec;
    use xpallet_assets::AssetType;
    // import vault,issue,assets code.
    use crate::assets::{pallet as assets, pallet::BalanceOf, pallet::BridgeStatus};
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
        ChainStatusErro,
        /// redeem request is accepted
        RedeemRequestIsAccepted,
        /// cancle redeem is accepted
        CancleRedeemIsAccepted,
        /// liquidation redeem is accepted
        LiquidationRedeemIsAccepted,
        /// Execute redeem is accepted
        ExecuteRedeemIsAccepted,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::error]
    pub enum Error<T> {
        /// redeem request id is not exsit
        RedeemRequestNotFound,
        /// redeemRequest cancelled for forced redeem when it's not expired.
        RedeemRequestNotExpired,
        /// vault is under Liquidation
        ValtLiquidated,
        /// actioner is not the request's owner
        UnauthorizedUser,
        /// redeem amount is to low
        AmountBelowDustAmount,
        /// redeem amount is not correct
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

    /// Auto-increament id to identify each redeem request.
    /// Also presents total amount of created requests.
    #[pallet::storage]
    pub(crate) type RequestCount<T: Config> = StorageValue<_, RequestId, ValueQuery>;

    /// The minimum amount of btc that is accepted for redeem requests; any lower values would
    /// risk the bitcoin client to reject the payment
    #[pallet::storage]
    pub(crate) type RedeemBtcDustValue<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

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
            Self::ensure_chain_correct_status()?;

            // verify redeemer asset
            let sender = ensure_signed(origin)?;
            let redeemer_balance =
                xpallet_assets::Module::<T>::asset_balance_of(&sender, &1, AssetType::Usable);
            ensure!(
                redeem_amount <= redeemer_balance,
                Error::<T>::InsufficiantAssetsFonds
            );

            // ensure this vault can work.
            let height = <frame_system::Pallet<T>>::block_number();
            let vault = vault::Pallet::<T>::get_active_vault_by_id(&vault_id)?;

            // only allow requests of amount above above the minimum
            let dust_value = <RedeemBtcDustValue<T>>::get();
            ensure!(
                // this is the amount the vault will send (minus fee)
                redeem_amount >= dust_value,
                Error::<T>::AmountBelowDustAmount
            );

            // increase vault's to_be_redeemed_tokens

            // lock redeem's xtbc
            let _ = xpallet_assets::Module::<T>::move_balance(
                &1,
                &sender,
                AssetType::Usable,
                &sender,
                AssetType::Locked,
                redeem_amount,
            );

            // check vault if is below premium

            // generate redeem request identify and insert it to record
            let request_id = Self::get_next_request_id();
            <RedeemRequests<T>>::insert(
                request_id,
                RedeemRequest::<T> {
                    vault: vault_id,
                    open_time: height,
                    requester: sender,
                    btc_address: btc_addr,
                    amount: redeem_amount,
                    redeem_fee: Default::default(),
                    completed: false,
                    cancelled: false,
                    reimburse: false,
                },
            );

            // send msg to user
            Self::deposit_event(Event::<T>::ExecuteRedeemIsAccepted);

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
            Self::ensure_chain_correct_status()?;
            let sender = ensure_signed(origin)?;

            // ensure this is the correct vault
            let request =
                <RedeemRequests<T>>::get(request_id).ok_or(Error::<T>::RedeemRequestNotFound)?;
            ensure!(request.vault == sender, Error::<T>::UnauthorizedUser);

            // ensure this redeem not expired
            let height = <frame_system::Pallet<T>>::block_number();
            let expired_time = <RedeemRequestExpiredTime<T>>::get();
            ensure!(
                height - request.open_time < expired_time,
                Error::<T>::RedeemRequestNotExpired
            );

            // TODO verify tx

            // decrase user's XBTC amount.
            xpallet_assets::Module::<T>::destroy_reserved_withdrawal(
                &1,
                &request.requester,
                request.amount,
            )?;

            Self::deposit_event(Event::<T>::LiquidationRedeemIsAccepted);

            Ok(().into())
        }

        /// user cancle redeem
        #[pallet::weight(0)]
        pub fn cancle_redeem(
            origin: OriginFor<T>,
            request_id: RequestId,
            reimburse: bool,
        ) -> DispatchResultWithPostInfo {
            let sender = ensure_signed(origin)?;

            // ensure sender is is redeem's owner
            let request =
                <RedeemRequests<T>>::get(request_id).ok_or(Error::<T>::RedeemRequestNotFound)?;
            ensure!(request.requester == sender, Error::<T>::UnauthorizedUser);

            // ensure the redeem request right status
            ensure!(!request.cancelled, Error::<T>::CancleRedeemErrOfCancled);
            ensure!(!request.completed, Error::<T>::CancleRedeemErrOfCompleted);

            //ensure the redeem request is outdate
            let height = <frame_system::Pallet<T>>::block_number();
            let expired_time = <RedeemRequestExpiredTime<T>>::get();
            ensure!(
                height - request.open_time > expired_time,
                Error::<T>::RedeemRequestNotExpired
            );

            let worth_pcx = assets::Pallet::<T>::convert_to_pcx(request.amount)?;
            // punish vault fee
            let punishment_fee: BalanceOf<T> = 0.into();
            if reimburse {
                // decrease vault tokens

                // burn user xbtc
                let _ = xpallet_assets::Module::<T>::move_balance(
                    &1,
                    &request.requester,
                    AssetType::Locked,
                    &request.requester,
                    AssetType::ReservedWithdrawal,
                    punishment_fee,
                );

                // vault give pcx to sender
                let _ = xpallet_assets::Module::<T>::move_balance(
                    &1,
                    &request.vault,
                    AssetType::Reserved,
                    &request.requester,
                    AssetType::Usable,
                    worth_pcx,
                );
            } else {
                // punishfee give redeemer
                let _ = xpallet_assets::Module::<T>::move_balance(
                    &1,
                    &request.vault,
                    AssetType::Usable,
                    &request.requester,
                    AssetType::Usable,
                    punishment_fee,
                );
            }

            //ban vault.

            Self::remove_redeem_request(request_id, true, reimburse);
            Self::deposit_event(Event::<T>::CancleRedeemIsAccepted);

            Ok(().into())
        }

        /// user liquidation redeem. when user do this means he can get pcx only.
        #[pallet::weight(0)]
        pub fn liquidation_redeem(
            origin: OriginFor<T>,
            redeem_amount: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {
            Self::ensure_chain_correct_status()?;
            let sender = ensure_signed(origin)?;

            // ensure redeem amount less than have
            let redeemer_balance =
                xpallet_assets::Module::<T>::asset_balance_of(&sender, &1, AssetType::Usable);
            ensure!(
                redeem_amount <= redeemer_balance,
                Error::<T>::InsufficiantAssetsFonds
            );

            // user burn xbtc
            let _ = xpallet_assets::Module::<T>::move_balance(
                &1,
                &sender,
                AssetType::Usable,
                &sender,
                AssetType::ReservedWithdrawal,
                redeem_amount,
            );

            // catulate user's XBTC worth how much pcx, then give he the pcx
            let worth_pcx = assets::Pallet::<T>::convert_to_pcx(redeem_amount)?;

            // system vault give him pcx
            //let system_vault: T::AccountId = vault::pallet::<T>::<vault::pallet::<T>>::get();

            // send msg to user
            Self::deposit_event(Event::<T>::LiquidationRedeemIsAccepted);
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
        /// ensure the chain is in correct status
        fn ensure_chain_correct_status() -> DispatchResultWithPostInfo {
            Ok(().into())
        }

        /// generate secure key from account id
        pub(crate) fn get_next_request_id() -> RequestId {
            <RequestCount<T>>::mutate(|n| {
                *n += 1;
                *n
            })
        }

        /// Get `RedeemssueRequest` from id
        pub(crate) fn get_redeem_request_by_id(request_id: RequestId) -> Option<RedeemRequest<T>> {
            <RedeemRequests<T>>::get(request_id)
        }

        /// mark the request as removed
        fn remove_redeem_request(request_id: RequestId, cancelled: bool, reimburse: bool) {
            // TODO: delete redeem request from storage
            <RedeemRequests<T>>::mutate(request_id, |request| {
                // request.completed = !cancelled;
                // request.cancelled = cancelled;
                // request.reimburse = reimburse
            });
        }

        ///check request if is expired
        fn has_request_expired(opentime: T::BlockNumber, period: T::BlockNumber) -> bool {
            let height = <frame_system::Module<T>>::block_number();
            height > opentime + period
        }
    }
}
