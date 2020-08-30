// Copyright 2018-2019 Chainpool.

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::type_complexity)]

mod benchmarking;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
pub mod types;
mod weight_info;

// Substrate
use frame_support::{
    decl_error, decl_event, decl_module, decl_storage, dispatch::DispatchResult, ensure,
    traits::Currency, IterableStorageMap,
};
use frame_system::ensure_root;
use sp_runtime::traits::StaticLookup;
use sp_std::collections::btree_map::BTreeMap;
use sp_std::prelude::*;

use orml_utilities::with_transaction_result;

// ChainX
use chainx_primitives::{AddrStr, AssetId};
use xp_runtime::Memo;

use xpallet_assets::{AssetType, Chain};

use xpallet_support::{error, info, try_addr, warn};

pub use self::types::{Withdrawal, WithdrawalRecord, WithdrawalState};
use crate::weight_info::WeightInfo;

pub type BalanceOf<T> = <<T as xpallet_assets::Trait>::Currency as Currency<
    <T as frame_system::Trait>::AccountId,
>>::Balance;

pub trait Trait: frame_system::Trait + xpallet_assets::Trait {
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

    type WeightInfo: WeightInfo;
}

decl_error! {
    /// Error for the Gateway Records Module
    pub enum Error for Module<T: Trait> {
        /// reject native asset for this module
        DenyNativeAsset,
        /// id not in withdrawal WithdrawalRecord records
        NotExisted,
        /// WithdrawalRecord state not `Applying`
        NotApplyingState,
        /// WithdrawalRecord state not `Processing`
        NotProcessingState,
        /// the applicant is not this account
        InvalidAccount,
        /// state only allow `RootFinish` and `RootCancel`
        InvalidState,
        /// meet unexpected chain
        UnexpectedChain,
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        type Error = Error<T>;
        fn deposit_event() = default;
        // only for root
        #[weight = 0]
        fn root_deposit(origin, who: <T::Lookup as StaticLookup>::Source, #[compact] asset_id: AssetId, #[compact] balance: BalanceOf<T>) -> DispatchResult {
            ensure_root(origin)?;
            let who = T::Lookup::lookup(who)?;
            Self::deposit(&who, &asset_id, balance)
        }

        #[weight = 0]
        fn root_withdrawal(origin, who: <T::Lookup as StaticLookup>::Source, #[compact] asset_id: AssetId, #[compact] balance: BalanceOf<T>, addr: AddrStr, memo: Memo) -> DispatchResult {
            ensure_root(origin)?;
            let who = T::Lookup::lookup(who)?;
            Self::withdrawal(&who, &asset_id, balance, addr, memo)
        }

        #[weight = 0]
        pub fn set_withdrawal_state(origin, #[compact] withdrawal_id: u32, state: WithdrawalState) -> DispatchResult {
            ensure_root(origin)?;
            ensure!(state != WithdrawalState::Applying || state != WithdrawalState::Processing, "Do not accept this state.");
            match Self::finish_withdrawal_impl(withdrawal_id, state) {
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
        <T as frame_system::Trait>::AccountId,
        Balance = BalanceOf<T>,
        WithdrawalRecord = WithdrawalRecord<<T as frame_system::Trait>::AccountId, BalanceOf<T>, <T as frame_system::Trait>::BlockNumber> {
        Deposit(AccountId, AssetId, Balance),
        ApplyWithdrawal(u32, WithdrawalRecord),
        FinishWithdrawal(u32, WithdrawalState),
    }
);

decl_storage! {
    trait Store for Module<T: Trait> as XGatewayRecords {
        /// withdrawal applications collection, use serial number to mark them, and has prev and next to link them
        pub PendingWithdrawals get(fn pending_withdrawals):map hasher(twox_64_concat) u32
                => Option<WithdrawalRecord<T::AccountId, BalanceOf<T>, T::BlockNumber>>;

        pub WithdrawalStateOf get(fn state_of): map hasher(twox_64_concat) u32 => Option<WithdrawalState>;
        /// withdrawal WithdrawalRecord serial number
        pub SerialNumber get(fn number): u32 = 0;
    }
}

impl<T: Trait> Module<T> {
    /// deposit/withdrawal pre-process
    fn check_asset(_: &T::AccountId, id: &AssetId) -> DispatchResult {
        xpallet_assets::Module::<T>::ensure_not_native_asset(id)?;
        Ok(())
    }

    fn check_withdrawal(
        who: &T::AccountId,
        asset_id: &AssetId,
        value: BalanceOf<T>,
    ) -> DispatchResult {
        Self::check_asset(who, asset_id)?;

        let free = xpallet_assets::Module::<T>::usable_balance(who, asset_id);
        if free < value {
            return Err(xpallet_assets::Error::<T>::InsufficientBalance.into());
        }

        Ok(())
    }

    fn check_chain(id: &AssetId, expected_chain: Chain) -> DispatchResult {
        let record = Self::pending_withdrawals(id).ok_or(Error::<T>::NotExisted)?;
        Self::check_chain_for_asset(&record.asset_id(), expected_chain)
    }

    #[inline]
    fn check_chain_for_asset(asset_id: &AssetId, expected_chain: Chain) -> DispatchResult {
        let asset = xpallet_assets_registrar::Module::<T>::get_asset_info(&asset_id)?;
        let asset_chain = asset.chain();
        if expected_chain != asset_chain {
            return Err(Error::<T>::UnexpectedChain.into());
        }
        Ok(())
    }
}

impl<T: Trait> Module<T> {
    /// deposit, notice this func has include deposit_init and deposit_finish (not wait for block confirm process)
    pub fn deposit(
        who: &T::AccountId,
        asset_id: &AssetId,
        balance: BalanceOf<T>,
    ) -> DispatchResult {
        Self::check_asset(who, asset_id)?;

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
        balance: BalanceOf<T>,
        addr: AddrStr,
        ext: Memo,
    ) -> DispatchResult {
        Self::check_withdrawal(who, asset_id, balance)?;

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

        let appl = WithdrawalRecord::<T::AccountId, BalanceOf<T>, T::BlockNumber>::new(
            who.clone(),
            *asset_id,
            balance,
            addr,
            ext,
            frame_system::Module::<T>::block_number(),
        );

        // set storage
        Self::lock(who, asset_id, balance)?;
        PendingWithdrawals::<T>::insert(id, appl.clone());
        WithdrawalStateOf::insert(id, WithdrawalState::Applying);
        let newid = match id.checked_add(1_u32) {
            Some(r) => r,
            None => 0,
        };
        SerialNumber::put(newid);

        Self::deposit_event(RawEvent::ApplyWithdrawal(id, appl));
        Ok(())
    }

    /// change Applying to Processing
    pub fn process_withdrawal(chain: Chain, serial_number: u32) -> DispatchResult {
        match Self::state_of(serial_number) {
            Some(state) => {
                if state != WithdrawalState::Applying {
                    error!(
                        "[process_withdrawal]|WithdrawalRecord state not `Applying`|id:{:}|state:{:?}",
                        serial_number, state
                    );
                    return Err(Error::<T>::NotApplyingState.into());
                }
                Self::check_chain(&serial_number, chain)?;

                // set storage
                WithdrawalStateOf::insert(serial_number, WithdrawalState::Processing);
                Ok(())
            }
            None => {
                error!(
                    "[process_withdrawal]|id not in WithdrawalRecord records|id:{:}",
                    serial_number
                );
                Err(Error::<T>::NotExisted.into())
            }
        }
    }

    pub fn process_withdrawals(chain: Chain, serial_numbers: &[u32]) -> DispatchResult {
        with_transaction_result(|| {
            for id in serial_numbers.iter() {
                Self::process_withdrawal(chain, *id)?;
            }
            Ok(())
        })
    }

    /// withdrawal finish, let the locking asset destroy
    /// Change Processing to final state
    /// notice parameter `chain`, when the withdrawal_id(serial_number) is passed by runtime
    /// self logic, just pass `None`, when the withdrawal_id is passed by the parameter from call(which
    /// means the id is from outside), should pass `Some(chain)` to verify whether the withdrawal is
    /// related to this chain.
    ///
    /// e.g. bitcoin release reserved by receive a valid withdrawal transaction, the withdraw id is
    /// valid when trustees submit withdrawal info, so that just release it directly.
    /// ethereum released reserved by trustees submit release request directly, so that we should check
    /// whether the withdrawal belongs to Ethereum Chain, in case release other chain withdraw.
    pub fn finish_withdrawal(expected_chain: Option<Chain>, serial_number: u32) -> DispatchResult {
        if let Some(state) = Self::state_of(serial_number) {
            if state != WithdrawalState::Processing {
                error!("[finish_withdrawal]only allow `Processing` for this WithdrawalRecord|id:{:}|state:{:?}", serial_number, state);
                return Err(Error::<T>::NotProcessingState.into());
            }
            // notice if pass Some(), must check the chain
            if let Some(chain) = expected_chain {
                Self::check_chain(&serial_number, chain)?;
            }
        }
        Self::finish_withdrawal_impl(serial_number, WithdrawalState::NormalFinish)
    }

    pub fn finish_withdrawals(
        expected_chain: Option<Chain>,
        serial_numbers: &[u32],
    ) -> DispatchResult {
        with_transaction_result(|| {
            for id in serial_numbers.iter() {
                Self::finish_withdrawal(expected_chain, *id)?;
            }
            Ok(())
        })
    }

    pub fn revoke_withdrawal(who: &T::AccountId, serial_number: u32) -> DispatchResult {
        if let Some(state) = Self::state_of(serial_number) {
            let appl = Self::pending_withdrawals(serial_number).ok_or(Error::<T>::NotExisted)?;

            if appl.applicant() != who {
                error!(
                    "[revoke_withdrawal]|the applicant is not this account|applicant:{:?}|who:{:?}",
                    appl.applicant(),
                    who
                );
                return Err(Error::<T>::InvalidAccount.into());
            }

            if state != WithdrawalState::Applying {
                error!("[finish_withdrawal]|only allow `Applying` for this WithdrawalRecord|id:{:}|state:{:?}", serial_number, state);
                return Err(Error::<T>::NotApplyingState.into());
            }
        }
        Self::finish_withdrawal_impl(serial_number, WithdrawalState::NormalCancel)
    }

    /// revoke to applying
    pub fn recover_withdrawal_by_trustee(chain: Chain, serial_number: u32) -> DispatchResult {
        Self::check_chain(&serial_number, chain)?;
        if let Some(state) = Self::state_of(serial_number) {
            if state != WithdrawalState::Processing {
                error!("[recover_withdrawal_by_trustee]|only allow `Processing` for this WithdrawalRecord|id:{:}|state:{:?}", serial_number, state);
                return Err(Error::<T>::NotProcessingState.into());
            }
            WithdrawalStateOf::insert(serial_number, WithdrawalState::Applying);
            return Ok(());
        }
        Err(Error::<T>::NotExisted.into())
    }

    /// revoke to cancel
    pub fn revoke_withdrawal_by_trustee(chain: Chain, serial_number: u32) -> DispatchResult {
        Self::check_chain(&serial_number, chain)?;
        if let Some(state) = Self::state_of(serial_number) {
            if state != WithdrawalState::Processing {
                error!("[revoke_withdrawal_by_trustee]|only allow `Processing` for this WithdrawalRecord|id:{:}|state:{:?}", serial_number, state);
                return Err(Error::<T>::NotProcessingState.into());
            }
        }
        Self::finish_withdrawal_impl(serial_number, WithdrawalState::RootCancel)
    }

    pub fn set_withdrawal_state_by_trustees(
        chain: Chain,
        withdrawal_id: u32,
        state: WithdrawalState,
    ) -> DispatchResult {
        match state {
            WithdrawalState::RootFinish | WithdrawalState::RootCancel => { /*do nothing*/ }
            _ => {
                error!("[set_withdrawal_state_by_trustees]|state only allow `RootFinish` and `RootCancel`|state:{:?}", state);
                return Err(Error::<T>::InvalidState.into());
            }
        }
        if let Some(state) = Self::state_of(withdrawal_id) {
            Self::check_chain(&withdrawal_id, chain)?;

            if state != WithdrawalState::Processing {
                error!("[set_withdrawal_state_by_trustees]only allow `Processing` for this WithdrawalRecord|id:{:}|state:{:?}", withdrawal_id, state);
                return Err(Error::<T>::NotProcessingState.into());
            }
        }

        Self::set_withdrawal_state(frame_system::RawOrigin::Root.into(), withdrawal_id, state)
    }

    fn finish_withdrawal_impl(serial_number: u32, state: WithdrawalState) -> DispatchResult {
        let record = Self::pending_withdrawals(serial_number).ok_or(Error::<T>::NotExisted)?;

        let who = record.applicant();
        let asset_id = record.asset_id();
        let balance = record.balance();

        info!(
            "[finish_withdrawal]|wirhdrawal id:{:}|who:{:?}|asset_id:{:}|balance:{:?}",
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
                warn!("[finish_withdrawal_impl]|should not meet this branch in normally, except in root|state:{:?}", state);
            }
        }

        PendingWithdrawals::<T>::remove(serial_number);
        WithdrawalStateOf::remove(serial_number);

        Self::deposit_event(RawEvent::FinishWithdrawal(serial_number, state));
        Ok(())
    }

    fn lock(who: &T::AccountId, asset_id: &AssetId, value: BalanceOf<T>) -> DispatchResult {
        let _ = xpallet_assets::Module::<T>::move_balance(
            asset_id,
            who,
            AssetType::Usable,
            who,
            AssetType::ReservedWithdrawal,
            value,
        )
        .map_err::<xpallet_assets::Error<T>, _>(Into::into)?;
        Ok(())
    }

    fn unlock(who: &T::AccountId, asset_id: &AssetId, value: BalanceOf<T>) -> DispatchResult {
        let _ = xpallet_assets::Module::<T>::move_balance(
            asset_id,
            who,
            AssetType::ReservedWithdrawal,
            who,
            AssetType::Usable,
            value,
        )
        .map_err::<xpallet_assets::Error<T>, _>(Into::into)?;
        Ok(())
    }

    fn destroy(who: &T::AccountId, asset_id: &AssetId, value: BalanceOf<T>) -> DispatchResult {
        let _ = xpallet_assets::Module::<T>::destroy(&asset_id, &who, value)?;
        Ok(())
    }

    pub fn withdrawal_list() -> BTreeMap<u32, Withdrawal<T::AccountId, BalanceOf<T>, T::BlockNumber>>
    {
        PendingWithdrawals::<T>::iter()
            .map(|(id, record)| {
                (
                    id,
                    Withdrawal::new(record, Self::state_of(id).unwrap_or_default()),
                )
            })
            .collect()
    }

    pub fn withdrawals_list_by_chain(
        chain: Chain,
    ) -> BTreeMap<u32, Withdrawal<T::AccountId, BalanceOf<T>, T::BlockNumber>> {
        Self::withdrawal_list()
            .into_iter()
            .filter(|(_, withdrawal)| {
                Self::check_chain_for_asset(&withdrawal.asset_id, chain).is_ok()
            })
            .collect()
    }
}
