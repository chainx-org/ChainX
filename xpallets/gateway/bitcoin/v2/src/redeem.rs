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
        /// Block height when the redeem requested
        pub(crate) opentime: BlockNumber,
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
        traits::{Currency, Hooks,IsType, ReservableCurrency},
        Twox64Concat,
    };
    use frame_system::{
        ensure_signed,
        pallet_prelude::{BlockNumberFor, OriginFor},
    };
    use sp_runtime::DispatchError;
    use sp_std::{convert::TryInto, marker::PhantomData};

    // import vault,issue,assets code.
    use crate::assets::{pallet as assets, pallet::BalanceOf};
    use crate::issue::pallet as issuepallet;
    use crate::vault::pallet as vaultpallet;
    use crate::assets::pallet as assetspallet;

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
    pub trait Config: frame_system::Config + issuepallet::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
    }

    /// Events for redeem module
    #[pallet::event]
    #[pallet::generate_deposit(pub(crate) fn deposit_event)]
    pub enum Event<T: Config> {
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
        /// Vault is under Liquidation
        ValtLiquidated,

        /// redeem amount is too much
        RedeemAmountLargerThanHave,

        /// redeem is completed
        CancleRedeemErrOfCompleted,

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
        StorageMap<_, Twox64Concat, T::AccountId, RedeemRequest<T>>;

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// User request redeem
        #[pallet::weight(0)]
        pub fn request_redeem(
            origin: OriginFor<T>,
            redeem_amount: BalanceOf<T>,
            btc_addr: super::types::BtcAddress,
        ) -> DispatchResultWithPostInfo {
            let sender = ensure_signed(origin)?;
            let height = <frame_system::Pallet<T>>::block_number();

            // Assign valt id by random
            // let vault_id: T::AccountId = Self::get_vaultid_by_redeem(&sender, redeem_amount);
            // let vault = vaultpallet::Pallet::<T>::get_active_vault_by_id(&vault_id)?;

            //check if exist one valt is Liquidated
            let mut exsit_vault_liquidated = false;
            for vt in <vaultpallet::Vaults<T>>::iter() {
                if vt.1.status == crate::vault::types::VaultStatus::Liquidated {
                    exsit_vault_liquidated = true;
                    break;
                }
            }
            if exsit_vault_liquidated == true {
                // to do ...
            } else {
                // to do ...
            }

            // add the vault xbtc count

            // tell vault to transfer btc to this btc_addr

            // insert RedeemRequest into map
            <RedeemRequests<T>>::insert(
                sender.clone(),
                RedeemRequest::<T> {
                    opentime: height,
                    requester: sender,
                    btc_address: btc_addr,
                    amount: Default::default(),
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
        pub fn cancle_redeem(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
            let sender = ensure_signed(origin)?;
            let height = <frame_system::Pallet<T>>::block_number();

            // check user`s redeem status
            let user_redeem = <RedeemRequests<T>>::get(&sender);
            match user_redeem {
                Some(re) => {
                    match re.status {
                        WaitForGetBtc => {
                            // redeem can be cancled.
        
                            // subtract the vault xbtc count.
        
                            // tell vault do not transfer btc to user btc addr
                        }
                        Cancled => {
                            // redeem is already cancled. can not be cancled twice
        
                            // to do...
                        }
                        completed => {
                            // redeem is already completed. vault has given btc to user`s btc address.
        
                            // to do...
                        }
                    }
                },
                None => (),
            }
            
            // send msg to user
            Self::deposit_event(Event::<T>::CancleRedeemIsAccepted);

            Ok(().into())
        }

        /// user force redeem. when user do this means he can get pcx only.
        #[pallet::weight(0)]
        pub fn force_redeem(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
            let sender = ensure_signed(origin)?;
            let height = <frame_system::Pallet<T>>::block_number();

            // check user`s redeem status
            let user_redeem = <RedeemRequests<T>>::get(&sender);
            match user_redeem {
                Some(re) => {
                    match re.status {
                        WaitForGetBtc => {
                            // redeem can be cancled.

                            // subtract the vault xbtc count.

                            // tell vault do not transfer btc to user btc addr
                        }
                        Cancled => {
                            // redeem is already cancled. can not be force redeem

                            // to do...
                        }
                        completed => {
                            // redeem is already completed.

                            // to do...
                        }
                    }
                },
                None => (),
            }

            // send msg to user
            Self::deposit_event(Event::<T>::ForceRedeemIsAccepted);

            Ok(().into())
        }
    }

    impl<T: Config> Pallet<T> {

        /// get correct vault to serve redeem request
        // fn get_vaultid_by_redeem(user: &T::AccountId, redeem_amount: BalanceOf<T>) -> Result<vaultpallet::Vault<T::AccountId, T::BlockNumber, BalanceOf<T>>, DispatchError> {
        //     /// to do ...
        //     //let vt = vaultpallet::get_vault_by_id(user);
        //     Ok(())
        // }

        /// caculate redeem fee
        fn caculate_redeem_fee(redeem_amount: BalanceOf<T>) -> u8 {
            /// to do ...
            0
        }
    }
}
