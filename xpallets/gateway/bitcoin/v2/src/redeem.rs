#![cfg_attr(not(feature = "std"), no_std)]

pub mod types {
    use codec::{Decode, Encode};
    use sp_std::vec::Vec;

    pub type BtcAddress = Vec<u8>;
    pub type RedeemRequestIdentify = Vec<u8>;

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
        dispatch::DispatchResultWithPostInfo,
        ensure,
        storage::types::{StorageMap, StorageValue, ValueQuery},
        traits::{Currency, Hooks, IsType, ReservableCurrency},
        Twox64Concat,
    };
    use frame_system::{
        ensure_signed,
        pallet_prelude::{BlockNumberFor, OriginFor},
    };
    use sp_runtime::DispatchError;
    use sp_std::{convert::TryInto, marker::PhantomData, vec::Vec};

    // import vault,issue,assets code.
    use crate::assets::{pallet as assetspallet, pallet::BalanceOf, pallet::BridgeStatus};
    use crate::issue::pallet as issuepallet;
    use crate::vault::pallet as vaultpallet;

    type RedeemRequest<T> = super::types::RedeemRequest<
        <T as frame_system::Config>::AccountId,
        <T as frame_system::Config>::BlockNumber,
        BalanceOf<T>,
        BalanceOf<T>,
    >;

    #[pallet::pallet]
    #[pallet::generate_store(pub(crate) trait Store)]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::config]
    pub trait Config: frame_system::Config + issuepallet::Config + xpallet_assets::Config {
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
        /// force redeem is accepted
        ForceRedeemIsAccepted,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::error]
    pub enum Error<T> {
        /// redeem request id is not exsit
        RedeemRequestNotFound,

        /// `RedeemRequest` cancelled for forced redeem when it's not expired.
        RedeemRequestNotExpired,

        /// Vault is under Liquidation
        ValtLiquidated,

        /// redeem amount is too much
        InsufficiantAssetsFonds

        /// redeem is completed
        RedeemRequestAlreadyCompleted,

        /// redeem is cancled
        CancleRedeemErrOfCancled,
    }

    /// redeem fee when use request redeem
    #[pallet::storage]
    #[pallet::getter(fn redeem_fee)]
    pub(crate) type RedeemFee<T: Config> = StorageValue<_, u8, ValueQuery>;

    /// Auto-increament id to identity each request
    #[pallet::storage]
    pub(crate) type RedeemId<T: Config> = StorageValue<_, u64, ValueQuery>;

    /// Mapping from redeem id to `RedeemRequest`
    #[pallet::storage]
    pub(crate) type RedeemRequests<T: Config> =
        StorageMap<_, Twox64Concat, super::types::RedeemRequestIdentify, RedeemRequest<T>>;

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
            let height = <frame_system::Pallet<T>>::block_number();
            let vault = vaultpallet::Pallet::<T>::get_active_vault_by_id(&vault_id)?;

            //check current bridge status
            let bridge_status = <BridgeStatus<T>>::get();
            match bridge_status {
                crate::assets::types::Status::Running => {}
                crate::assets::types::Status::Error => {
                    Self::deposit_event(Event::<T>::ChainStatusErro);
                    return Ok(().into());
                }
                crate::assets::types::Status::Shutdown => {
                    Self::deposit_event(Event::<T>::ChainStatusErro);
                    return Ok(().into());
                }
            }

            // decrase user's XBTC amount

            // decrese vault's lock collateral.
            let worth_pcx = assetspallet::Pallet::<T>::convert_to_pcx(redeem_amount)?;
            assetspallet::Pallet::<T>::lock_collateral(&vault_id, worth_pcx)?;

            // tell vault to transfer btc to this btc_addr

            // generate redeem request identify
            let redeemid: super::types::RedeemRequestIdentify = Vec::new();
            let redeemidentify: super::types::RedeemRequestIdentify = redeemid;

            // insert RedeemRequest into map
            <RedeemRequests<T>>::insert(
                redeemidentify,
                RedeemRequest::<T> {
                    vault: vault_id,
                    open_time: height,
                    requester: sender,
                    btc_address: btc_addr,
                    amount: redeem_amount,
                    redeem_fee: Default::default(),
                    status: super::types::RedeemRequestStatus::WaitForGetBtc,
                },
            );

            // Redeem id increase
            <RedeemId<T>>::mutate(|c| *c += 1);

            // send msg to user
            Self::deposit_event(Event::<T>::RedeemRequestIsAccepted);

            Ok(().into())
        }

        /// user cancle redeem
        #[pallet::weight(0)]
        pub fn cancle_redeem(
            origin: OriginFor<T>,
            redeemid: super::types::RedeemRequestIdentify,
        ) -> DispatchResultWithPostInfo {
            let sender = ensure_signed(origin)?;
            let height = <frame_system::Pallet<T>>::block_number();
            let request = <RedeemRequests<T>>::get(redeemid).ok_or(Error::<T>::RedeemNotExist)?;
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

            // add user'XBTC amount

            // send msg to user
            Self::deposit_event(Event::<T>::CancleRedeemIsAccepted);

            Ok(().into())
        }

        /// user force redeem. when user do this means he can get pcx only.
        #[pallet::weight(0)]
        pub fn force_redeem(
            origin: OriginFor<T>,
            redeemid: super::types::RedeemRequestIdentify,
        ) -> DispatchResultWithPostInfo {
            let sender = ensure_signed(origin)?;
            let height = <frame_system::Pallet<T>>::block_number();
            let request = <RedeemRequests<T>>::get(redeemid).ok_or(Error::<T>::RedeemNotExist)?;
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
            let worth_pcx = assetspallet::Pallet::<T>::convert_to_pcx(request.amount)?;

            // add user pcx amount of worth_pcx.then we should sub who's pcx ????
            assetspallet::Pallet::<T>::slash_collateral(&sender, &sender, worth_pcx);

            // notice(how to give phone msg or mail msg?) the vault that the user's force redeem action

            // send msg to user
            Self::deposit_event(Event::<T>::ForceRedeemIsAccepted);

            Ok(().into())
        }
    }

    impl<T: Config> Pallet<T> {
        /// caculate redeem fee
        fn caculate_redeem_fee(redeem_amount: BalanceOf<T>) -> u128 {
            // to do ...
            0
        }
    }
}
