#![cfg_attr(not(feature = "std"), no_std)]

pub mod types {
    use codec::{Decode, Encode};
    use sp_std::vec::Vec;
    pub type BtcAddress = Vec<u8>;

    #[derive(Encode, Decode, Clone, PartialEq)]
    #[cfg_attr(feature = "std", derive(Debug))]
    pub enum RedeemRequestStatus {
        /// redeem is accepted and vault will transfer btc
        Processing,
        /// redeem is cancled by redeemer
        Cancled,
        /// redeem is compeleted
        Completed,
    }

    // default value
    impl Default for RedeemRequestStatus {
        fn default() -> Self {
            RedeemRequestStatus::Processing
        }
    }

    #[derive(Encode, Decode, Default, Clone, PartialEq)]
    #[cfg_attr(feature = "std", derive(Debug))]
    pub struct RedeemRequest<AccountId, BlockNumber, XBTC, PCX> {
        /// vault id
        pub(crate) vault: AccountId,
        /// block height when the redeem requested
        pub(crate) open_time: BlockNumber,
        /// who requests redeem
        pub(crate) requester: AccountId,
        /// vault's btc address
        pub(crate) btc_address: BtcAddress,
        /// amount that user wants to redeem
        pub(crate) amount: XBTC,
        /// redeem fee amount
        pub(crate) redeem_fee: PCX,
        /// request status
        pub(crate) status: RedeemRequestStatus,
        /// if redeem is reimbursed by redeemer
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
    use super::types::{BtcAddress, RedeemRequestStatus};
    use crate::assets::{pallet as assets, pallet::BalanceOf};
    use crate::issue::pallet as issue;
    use crate::vault::pallet as vault;

    type RedeemRequest<T> = super::types::RedeemRequest<
        <T as frame_system::Config>::AccountId,
        <T as frame_system::Config>::BlockNumber,
        BalanceOf<T>,
        BalanceOf<T>,
    >;
    type RequestId = u128;

    const XBTC_ASSETID: u32 = 1;

    #[pallet::pallet]
    #[pallet::generate_store(pub(crate) trait Store)]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::config]
    pub trait Config: frame_system::Config + issue::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
    }

    /// events for redeem module
    #[pallet::event]
    #[pallet::generate_deposit(pub(crate) fn deposit_event)]
    pub enum Event<T: Config> {
        /// current chain status is not right
        ChainStatusError,
        /// redeem request is accepted
        RedeemRequest,
        /// cancle redeem is accepted
        RedeemCanceled,
        /// liquidation redeem is accepted
        RedeemLiquidation,
        /// Execute redeem is accepted
        RedeemExecuted,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::error]
    pub enum Error<T> {
        /// redeem request id is not exsit
        RedeemRequestNotFound,
        /// redeem request cancelled for forced redeem when it's not expired.
        RedeemRequestNotExpired,
        /// redeem request is expierd
        RedeemRequestExpired,
        /// vault is under Liquidation
        VaultLiquidated,
        /// actioner is not the request's owner
        UnauthorizedUser,
        /// redeem amount is to low
        AmountBelowDustAmount,
        /// redeem amount is not correct
        InsufficiantAssetsFunds,
        /// redeem is completed
        RedeemRequestAlreadyCompleted,
        /// redeem is cancled
        RedeemRequestAlreadyCancled,
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
            btc_addr: BtcAddress,
        ) -> DispatchResultWithPostInfo {
            Self::ensure_chain_correct_status()?;

            // verify redeemer asset
            let sender = ensure_signed(origin)?;
            let redeemer_balance = Self::asset_balance_of(&sender);
            ensure!(
                redeem_amount <= redeemer_balance,
                Error::<T>::InsufficiantAssetsFunds
            );

            // ensure this vault can work.
            let height = <frame_system::Pallet<T>>::block_number();
            let vault = vault::Pallet::<T>::get_active_vault_by_id(&vault_id)?;
            ensure!(
                redeem_amount <= vault.issued_tokens,
                Error::<T>::InsufficiantAssetsFunds
            );

            // only allow requests of amount above above the minimum
            let dust_value = <RedeemBtcDustValue<T>>::get();
            ensure!(
                // this is the amount the vault will send (minus fee)
                redeem_amount >= dust_value,
                Error::<T>::AmountBelowDustAmount
            );

            // increase vault's to_be_redeemed_tokens
            <vault::Vaults<T>>::mutate(&vault.id, |vault| {
                if let Some(vault) = vault {
                    vault.to_be_redeemed_tokens += redeem_amount;
                }
            });

            // lock redeem's xtbc
            Self::lock_xbtc(&sender, redeem_amount)?;

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
                    status: Default::default(),
                    reimburse: false,
                },
            );

            // send msg to user
            Self::deposit_event(Event::<T>::RedeemRequest);
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

            // ensure this redeem not expired
            let height = <frame_system::Pallet<T>>::block_number();
            let expired_time = <RedeemRequestExpiredTime<T>>::get();
            ensure!(
                height - request.open_time < expired_time,
                Error::<T>::RedeemRequestExpired
            );

            // TODO verify tx

            // decrase user's XBTC amount.
            Self::burn_xbtc(&request.requester, request.amount)?;

            Self::deposit_event(Event::<T>::RedeemExecuted);
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
            ensure!(
                request.status != RedeemRequestStatus::Completed,
                Error::<T>::RedeemRequestAlreadyCompleted
            );
            ensure!(
                request.status != RedeemRequestStatus::Cancled,
                Error::<T>::RedeemRequestAlreadyCancled
            );

            //ensure the redeem request is outdate
            let height = <frame_system::Pallet<T>>::block_number();
            let expired_time = <RedeemRequestExpiredTime<T>>::get();
            ensure!(
                height - request.open_time > expired_time,
                Error::<T>::RedeemRequestNotExpired
            );

            let vault = vault::Pallet::<T>::get_active_vault_by_id(&request.vault)?;
            let worth_pcx = assets::Pallet::<T>::convert_to_pcx(request.amount)?;

            // punish vault fee
            let punishment_fee: BalanceOf<T> = 0.into();
            if reimburse {
                // decrease vault tokens
                <vault::Vaults<T>>::mutate(&vault.id, |vault| {
                    if let Some(vault) = vault {
                        vault.to_be_redeemed_tokens += request.amount;
                    }
                });

                // burn user xbtc
                Self::burn_xbtc(&request.requester, punishment_fee)?;

                // vault give pcx to sender
                assets::Pallet::<T>::slash_collateral(
                    &request.vault,
                    &request.requester,
                    worth_pcx,
                )?;
            } else {
                // punish fee give redeemer
                Self::transfer_xbtc(&request.vault, &request.requester, punishment_fee)?;
            }

            Self::remove_redeem_request(request_id, RedeemRequestStatus::Completed);
            Self::deposit_event(Event::<T>::RedeemCanceled);
            Ok(().into())
        }

        /// user liquidation redeem. when user do this means he can get pcx only.
        #[pallet::weight(0)]
        pub fn liquidation_redeem(
            origin: OriginFor<T>,
            redeem_amount: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {
            let sender = ensure_signed(origin)?;

            // ensure redeem amount less than have
            let redeemer_balance = Self::asset_balance_of(&sender);
            ensure!(
                redeem_amount <= redeemer_balance,
                Error::<T>::InsufficiantAssetsFunds
            );

            // user burn xbtc
            Self::burn_xbtc(&sender, redeem_amount)?;

            // catulate user's XBTC worth how much pcx, then give he the pcx
            let worth_pcx = assets::Pallet::<T>::convert_to_pcx(redeem_amount)?;

            // system vault give him pcx
            let system_vault = <vault::LiquidateVault<T>>::get();
            assets::Pallet::<T>::slash_collateral(&system_vault.id, &sender, worth_pcx)?;

            // send msg to user
            Self::deposit_event(Event::<T>::RedeemLiquidation);
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
        fn remove_redeem_request(request_id: RequestId, status: RedeemRequestStatus) {
            // TODO: delete redeem request from storage
            <RedeemRequests<T>>::mutate(request_id, |request| {
                if let Some(request) = request {
                    request.status = status;
                }
            });
        }

        /// transfer XBTC
        fn transfer_xbtc(
            sender: &T::AccountId,
            receiver: &T::AccountId,
            count: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {
            xpallet_assets::Module::<T>::move_balance(
                &XBTC_ASSETID,
                &sender,
                AssetType::Usable,
                &receiver,
                AssetType::Usable,
                count,
            )
            .map_err::<xpallet_assets::Error<T>, _>(Into::into)?;
            Ok(().into())
        }

        /// lock XBTC
        fn lock_xbtc(user: &T::AccountId, count: BalanceOf<T>) -> DispatchResultWithPostInfo {
            xpallet_assets::Module::<T>::move_balance(
                &XBTC_ASSETID,
                &user,
                AssetType::Usable,
                &user,
                AssetType::Locked,
                count,
            )
            .map_err::<xpallet_assets::Error<T>, _>(Into::into)?;
            Ok(().into())
        }

        /// burn XBTC
        fn burn_xbtc(user: &T::AccountId, count: BalanceOf<T>) -> DispatchResultWithPostInfo {
            xpallet_assets::Module::<T>::move_balance(
                &XBTC_ASSETID,
                &user,
                AssetType::Locked,
                &user,
                AssetType::ReservedWithdrawal,
                count,
            )
            .map_err::<xpallet_assets::Error<T>, _>(Into::into)?;
            Ok(().into())
        }

        /// user have XBTC count
        fn asset_balance_of(user: &T::AccountId) -> BalanceOf<T> {
            xpallet_assets::Module::<T>::asset_balance_of(&user, &XBTC_ASSETID, AssetType::Usable)
        }
    }
}
