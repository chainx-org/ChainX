// Copyright 2018-2019 Chainpool.

#![cfg_attr(not(feature = "std"), no_std)]

mod mock;
mod tests;
pub mod types;

// Substrate
use frame_support::{decl_error, decl_event, decl_module, decl_storage, dispatch::DispatchResult};
use frame_system::{self as system, ensure_root};
use sp_std::prelude::*;

// ChainX
use chainx_primitives::{AddrStr, AssetId, Memo};

use xpallet_assets::{AssetType, Chain, ChainT};

use xpallet_support::{error, info, try_addr, warn};

pub use self::types::{Application, HeightOrTime, RecordInfo, TxState, WithdrawalState};

pub trait Trait: system::Trait + xpallet_assets::Trait {
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

decl_error! {
    /// Error for the Gateway Records Module
    pub enum Error for Module<T: Trait> {
        /// reject native asset for this module
        DenyNativeAsset,
        /// id not in withdrawal application records
        NotExisted,
        /// application state not `Applying`
        NotApplyingState,
        /// application state not `Processing`
        NotProcessingState,
        /// the applicant is not this account
        InvalidAccount,
        /// state only allow `RootFinish` and `RootCancel`
        InvalidState,
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event() = default;
        // only for root
        #[weight = 0]
        fn root_deposit(origin, who: T::AccountId, asset_id: AssetId, balance: T::Balance) -> DispatchResult {
            ensure_root(origin)?;
            Self::deposit(&who, &asset_id, balance)
        }

        #[weight = 0]
        fn root_withdrawal(origin, who: T::AccountId, asset_id: AssetId, balance: T::Balance) -> DispatchResult {
            ensure_root(origin)?;
            Self::withdrawal(&who, &asset_id, balance, Default::default(), Default::default())
        }

        #[weight = 0]
        pub fn set_withdrawal_state(origin, withdrawal_id: u32, state: WithdrawalState) -> DispatchResult {
            ensure_root(origin)?;
            match Self::withdrawal_finish_impl(withdrawal_id, state) {
                Ok(_) => {
                    info!("[withdraw]|ID of withdrawal completion: {:}", withdrawal_id);
                    Ok(())
                }
                Err(_e) => {
                    error!("[withdraw]|ID of withdrawal ERROR! {:}, reason:{:?}, please use root to fix it", withdrawal_id, _e);
                    Err(_e)
                }
            }
        }

        #[weight = 0]
        pub fn set_withdrawal_state_list(origin, item: Vec<(u32, WithdrawalState)>) -> DispatchResult {
            ensure_root(origin.clone())?;
            for (withdrawal_id, state) in item {
                let _ = Self::set_withdrawal_state(origin.clone(), withdrawal_id, state);
            }
            Ok(())
        }
    }
}

decl_event!(
    pub enum Event<T> where
        <T as system::Trait>::AccountId,
        <T as xpallet_assets::Trait>::Balance {
        Deposit(AccountId, AssetId, Balance),
        ApplyWithdrawal(u32, AccountId, AssetId, Balance, Memo, AddrStr),
        FinishWithdrawal(u32, WithdrawalState),
    }
);

decl_storage! {
    trait Store for Module<T: Trait> as XAssetsRecords {
        /// withdrawal applications collection, use serial number to mark them, and has prev and next to link them
        pub PendingWithdrawal get(fn pending_withdrawal):map hasher(twox_64_concat) u32
                => Option<Application<T::AccountId, T::Balance, T::BlockNumber>>;

        pub WithdrawalStateOf get(fn state_of): map hasher(twox_64_concat) u32 => Option<WithdrawalState>;
        /// withdrawal application serial number
        pub SerialNumber get(fn number): u32 = 0;
    }
}

impl<T: Trait> Module<T> {
    /// deposit/withdrawal pre-process
    fn before(_: &T::AccountId, asset_id: &AssetId) -> DispatchResult {
        if *asset_id == <xpallet_assets::Module<T> as ChainT>::ASSET_ID {
            Err(Error::<T>::DenyNativeAsset)?;
        }
        // other check
        Ok(())
    }

    fn withdraw_check_before(
        who: &T::AccountId,
        asset_id: &AssetId,
        value: T::Balance,
    ) -> DispatchResult {
        Self::before(who, asset_id)?;

        let free = xpallet_assets::Module::<T>::free_balance_of(who, asset_id);
        if free < value {
            Err(xpallet_assets::Error::<T>::InsufficientBalance)?;
        }

        Ok(())
    }
}

impl<T: Trait> Module<T> {
    /// deposit, notice this func has include deposit_init and deposit_finish (not wait for block confirm process)
    pub fn deposit(who: &T::AccountId, asset_id: &AssetId, balance: T::Balance) -> DispatchResult {
        Self::before(who, asset_id)?;

        info!(
            "[deposit]|who:{:?}|asset_id:{:}|balance:{:?}",
            who, asset_id, balance
        );

        let _ = xpallet_assets::Module::<T>::issue(asset_id, who, balance)?;
        Self::deposit_event(RawEvent::Deposit(who.clone(), *asset_id, balance));
        Ok(())
    }

    /// withdrawal, notice this func has include withdrawal_init and withdrawal_locking
    pub fn withdrawal(
        who: &T::AccountId,
        asset_id: &AssetId,
        balance: T::Balance,
        addr: AddrStr,
        ext: Memo,
    ) -> DispatchResult {
        Self::withdraw_check_before(who, asset_id, balance)?;

        let id = Self::number();

        info!(
            "[withdrawal]|id:{:}|who:{:?}|asset_id:{:}|balance:{:?}|addr:{:?}|memo:{:}",
            id,
            who,
            asset_id,
            balance,
            try_addr!(&addr),
            ext
        );

        let appl = Application::<T::AccountId, T::Balance, T::BlockNumber>::new(
            who.clone(),
            *asset_id,
            balance,
            addr,
            ext,
            system::Module::<T>::block_number(),
        );

        // set storage
        Self::lock(who, asset_id, balance)?;
        PendingWithdrawal::<T>::insert(id, appl.clone());
        WithdrawalStateOf::insert(id, WithdrawalState::Applying);
        let newid = match id.checked_add(1_u32) {
            Some(r) => r,
            None => 0,
        };
        SerialNumber::put(newid);

        Self::deposit_event(RawEvent::ApplyWithdrawal(
            id,
            appl.applicant,
            *asset_id,
            appl.balance,
            appl.ext,
            appl.addr, // if btc, the addr is base58 addr
        ));
        Ok(())
    }

    fn check_chain(id: &AssetId, expected_chain: Chain) -> DispatchResult {
        let appl = Self::pending_withdrawal(id).ok_or(Error::<T>::NotExisted)?;
        let asset = xpallet_assets::Module::<T>::get_asset(&appl.asset_id)?;
        let asset_chain = asset.chain();
        if expected_chain != asset_chain {}
        Ok(())
    }

    /// change Applying to Processing
    pub fn process_withdrawal(chain: Chain, serial_number: &[u32]) -> DispatchResult {
        let mut v = Vec::new();

        for id in serial_number.iter() {
            if let Some(state) = Self::state_of(id) {
                if state != WithdrawalState::Applying {
                    error!(
                        "[process_withdrawal]|application state not `Applying`|id:{:}|state:{:?}",
                        id, state
                    );
                    Err(Error::<T>::NotApplyingState)?;
                }
                Self::check_chain(id, chain)?;

                v.push(*id);
            } else {
                error!(
                    "[process_withdrawal]|id not in application records|id:{:}",
                    id
                );
                Err(Error::<T>::NotExisted)?;
            }
        }

        // mark all records is `Processing`
        for id in v.iter_mut() {
            WithdrawalStateOf::insert(id, WithdrawalState::Processing);
        }
        Ok(())
    }

    /// withdrawal finish, let the locking asset destroy
    /// Change Processing to final state
    pub fn withdrawal_finish(serial_number: u32) -> DispatchResult {
        if let Some(state) = Self::state_of(serial_number) {
            if state != WithdrawalState::Processing {
                error!("[withdrawal_finish]only allow `Processing` for this application|id:{:}|state:{:?}", serial_number, state);
                Err(Error::<T>::NotProcessingState)?;
            }
        }
        Self::withdrawal_finish_impl(serial_number, WithdrawalState::NormalFinish)
    }

    pub fn withdrawal_revoke(who: &T::AccountId, serial_number: u32) -> DispatchResult {
        if let Some(state) = Self::state_of(serial_number) {
            let appl = Self::pending_withdrawal(serial_number).ok_or(Error::<T>::NotExisted)?;

            if appl.applicant != *who {
                error!(
                    "[withdrawal_revoke]|the applicant is not this account|applicant:{:?}|who:{:?}",
                    appl.applicant, who
                );
                Err(Error::<T>::InvalidAccount)?;
            }

            if state != WithdrawalState::Applying {
                error!("[withdrawal_finish]|only allow `Applying` for this application|id:{:}|state:{:?}", serial_number, state);
                Err(Error::<T>::NotApplyingState)?;
            }
        }
        Self::withdrawal_finish_impl(serial_number, WithdrawalState::NormalCancel)
    }

    /// revoke to applying
    pub fn withdrawal_recover_by_trustee(serial_number: u32) -> DispatchResult {
        if let Some(state) = Self::state_of(serial_number) {
            if state != WithdrawalState::Processing {
                error!("[withdrawal_recover_by_trustee]|only allow `Processing` for this application|id:{:}|state:{:?}", serial_number, state);
                Err(Error::<T>::NotProcessingState)?;
            }
            WithdrawalStateOf::insert(serial_number, WithdrawalState::Applying);
            return Ok(());
        }
        Err(Error::<T>::NotExisted)?
    }

    /// revoke to cancel
    pub fn withdrawal_revoke_by_trustee(serial_number: u32) -> DispatchResult {
        if let Some(state) = Self::state_of(serial_number) {
            if state != WithdrawalState::Processing {
                error!("[withdrawal_revoke_by_trustee]|only allow `Processing` for this application|id:{:}|state:{:?}", serial_number, state);
                Err(Error::<T>::NotProcessingState)?;
            }
        }
        Self::withdrawal_finish_impl(serial_number, WithdrawalState::RootCancel)
    }

    pub fn fix_withdrawal_state_by_trustees(
        chain: Chain,
        withdrawal_id: u32,
        state: WithdrawalState,
    ) -> DispatchResult {
        match state {
            WithdrawalState::RootFinish | WithdrawalState::RootCancel => { /*do nothing*/ }
            _ => {
                error!("[fix_withdrawal_state_by_trustees]|state only allow `RootFinish` and `RootCancel`|state:{:?}", state);
                Err(Error::<T>::InvalidState)?;
            }
        }
        if let Some(state) = Self::state_of(withdrawal_id) {
            Self::check_chain(&withdrawal_id, chain)?;

            if state != WithdrawalState::Processing {
                error!("[fix_withdrawal_state_by_trustees]only allow `Processing` for this application|id:{:}|state:{:?}", withdrawal_id, state);
                Err(Error::<T>::NotProcessingState)?;
            }
        }

        Self::set_withdrawal_state(frame_system::RawOrigin::Root.into(), withdrawal_id, state)
    }

    fn withdrawal_finish_impl(serial_number: u32, state: WithdrawalState) -> DispatchResult {
        let appl = Self::pending_withdrawal(serial_number).ok_or(Error::<T>::NotExisted)?;

        let who = appl.applicant();
        let asset_id = appl.asset_id();
        let balance = appl.balance();

        info!(
            "[withdrawal_finish]|wirhdrawal id:{:}|who:{:?}|asset_id:{:}|balance:{:?}",
            serial_number, who, asset_id, balance
        );
        // destroy reserved asset
        match state {
            WithdrawalState::NormalFinish | WithdrawalState::RootFinish => {
                Self::destroy(&who, &asset_id, balance)?;
            }
            WithdrawalState::NormalCancel | WithdrawalState::RootCancel => {
                Self::unlock(&who, &asset_id, balance)?;
            }
            _ => {
                warn!("[withdrawal_finish_impl]|should not meet this branch in normally, except in root|state:{:?}", state);
            }
        }

        PendingWithdrawal::<T>::remove(serial_number);
        WithdrawalStateOf::remove(serial_number);

        Self::deposit_event(RawEvent::FinishWithdrawal(serial_number, state));
        Ok(())
    }

    fn lock(who: &T::AccountId, asset_id: &AssetId, value: T::Balance) -> DispatchResult {
        let _ = xpallet_assets::Module::<T>::move_balance(
            asset_id,
            who,
            AssetType::Free,
            who,
            AssetType::ReservedWithdrawal,
            value,
            true,
        )
        .map_err::<xpallet_assets::Error<T>, _>(Into::into)?;
        Ok(())
    }

    fn unlock(who: &T::AccountId, asset_id: &AssetId, value: T::Balance) -> DispatchResult {
        let _ = xpallet_assets::Module::<T>::move_balance(
            asset_id,
            who,
            AssetType::ReservedWithdrawal,
            who,
            AssetType::Free,
            value,
            true,
        )
        .map_err::<xpallet_assets::Error<T>, _>(Into::into)?;
        Ok(())
    }

    fn destroy(who: &T::AccountId, asset_id: &AssetId, value: T::Balance) -> DispatchResult {
        let _ = xpallet_assets::Module::<T>::destroy(&asset_id, &who, value)?;
        Ok(())
    }

    // pub fn withdrawal_application_numbers(chain: Chain, max_count: u32) -> Option<Vec<u32>> {
    //     let mut vec = Vec::new();
    //     // begin from header
    //     if let Some(header) = Self::application_mheader(chain) {
    //         let mut index = header.index();
    //         for _ in 0..max_count {
    //             if let Some(node) = Self::application_map(&index) {
    //                 vec.push(node.index());
    //                 if let Some(next) = node.next() {
    //                     index = next;
    //                 } else {
    //                     return Some(vec);
    //                 }
    //             }
    //         }
    //         return Some(vec);
    //     }
    //     None
    // }
    //
    // pub fn withdrawal_applications(
    //     chain: Chain,
    // ) -> Vec<Application<T::AccountId, T::Balance, T::BlockNumber>> {
    //     let mut vec = Vec::new();
    //     // begin from header
    //     if let Some(header) = Self::application_mheader(chain) {
    //         let mut index = header.index();
    //         while let Some(node) = Self::application_map(&index) {
    //             let next = node.next().clone();
    //             vec.push(node.data);
    //             if let Some(next) = next {
    //                 index = next;
    //             } else {
    //                 break;
    //             }
    //         }
    //     }
    //     vec
    // }
}
