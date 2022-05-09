// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::type_complexity)]

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
mod types;
pub mod weights;

use sp_std::{collections::btree_map::BTreeMap, prelude::*};

use frame_support::{
    dispatch::{DispatchError, DispatchResult},
    ensure, transactional,
    log::{error, info},
};
use frame_system::ensure_root;
use sp_runtime::traits::StaticLookup;

use chainx_primitives::{AddrStr, AssetId};
use xp_runtime::Memo;
use xpallet_assets::{AssetType, BalanceOf, Chain};
use xpallet_support::try_addr;

pub use self::types::{Withdrawal, WithdrawalRecord, WithdrawalRecordId, WithdrawalState};
pub use self::weights::WeightInfo;

pub type WithdrawalRecordOf<T> = WithdrawalRecord<
    <T as frame_system::Config>::AccountId,
    BalanceOf<T>,
    <T as frame_system::Config>::BlockNumber,
>;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::{pallet_prelude::*, transactional};
    use frame_system::pallet_prelude::*;

    #[pallet::config]
    pub trait Config: frame_system::Config + xpallet_assets::Config {
        /// The overarching event type.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// Weight information for extrinsics in this pallet.
        type WeightInfo: WeightInfo;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(crate) trait Store)]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Deposit asset token.
        ///
        /// This is a root-only operation.
        #[pallet::weight(<T as Config>::WeightInfo::root_deposit())]
        #[transactional]
        pub fn root_deposit(
            origin: OriginFor<T>,
            who: <T::Lookup as StaticLookup>::Source,
            #[pallet::compact] asset_id: AssetId,
            #[pallet::compact] balance: BalanceOf<T>,
        ) -> DispatchResult {
            ensure_root(origin)?;
            let who = T::Lookup::lookup(who)?;
            Self::deposit(&who, asset_id, balance)
        }

        /// Withdraw asset token (only lock token)
        ///
        /// This is a root-only operation.
        #[pallet::weight(<T as Config>::WeightInfo::root_withdraw())]
        #[transactional]
        pub fn root_withdraw(
            origin: OriginFor<T>,
            who: <T::Lookup as StaticLookup>::Source,
            #[pallet::compact] asset_id: AssetId,
            #[pallet::compact] balance: BalanceOf<T>,
            addr: AddrStr,
            memo: Memo,
        ) -> DispatchResult {
            ensure_root(origin)?;
            let who = T::Lookup::lookup(who)?;
            Self::withdraw(&who, asset_id, balance, addr, memo)
        }

        /// Set the state of withdrawal record with given id and state.
        ///
        /// This is a root-only operation.
        #[pallet::weight(<T as Config>::WeightInfo::set_withdrawal_state())]
        pub fn set_withdrawal_state(
            origin: OriginFor<T>,
            #[pallet::compact] withdrawal_id: WithdrawalRecordId,
            state: WithdrawalState,
        ) -> DispatchResult {
            ensure_root(origin)?;
            Self::set_withdrawal_state_by_root(withdrawal_id, state)
        }

        /// Set the state of withdrawal records in batches.
        ///
        /// This is a root-only operation.
        #[pallet::weight(<T as Config>::WeightInfo::set_withdrawal_state_list(item.len() as u32))]
        pub fn set_withdrawal_state_list(
            origin: OriginFor<T>,
            item: Vec<(WithdrawalRecordId, WithdrawalState)>,
        ) -> DispatchResult {
            ensure_root(origin)?;
            for (withdrawal_id, state) in item {
                let _ = Self::set_withdrawal_state_by_root(withdrawal_id, state);
            }
            Ok(())
        }
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(crate) fn deposit_event)]
    pub enum Event<T: Config> {
        /// An account deposited some asset. [who, asset_id, amount]
        Deposited(T::AccountId, AssetId, BalanceOf<T>),
        /// A withdrawal application was created. [withdrawal_id, record_info]
        WithdrawalCreated(WithdrawalRecordId, WithdrawalRecordOf<T>),
        /// A withdrawal proposal was processed. [withdrawal_id]
        WithdrawalProcessed(WithdrawalRecordId),
        /// A withdrawal proposal was recovered. [withdrawal_id]
        WithdrawalRecovered(WithdrawalRecordId),
        /// A withdrawal proposal was canceled. [withdrawal_id, withdrawal_state]
        WithdrawalCanceled(WithdrawalRecordId, WithdrawalState),
        /// A withdrawal proposal was finished successfully. [withdrawal_id, withdrawal_state]
        WithdrawalFinished(WithdrawalRecordId, WithdrawalState),
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Id not in withdrawal records
        NotExisted,
        /// WithdrawalRecord state not `Applying`
        NotApplyingState,
        /// WithdrawalRecord state not `Processing`
        NotProcessingState,
        /// The applicant is not this account
        InvalidAccount,
        /// State only allow `RootFinish` and `RootCancel`
        InvalidState,
        /// Meet unexpected chain
        UnexpectedChain,
    }

    #[pallet::type_value]
    pub fn DefaultForWithdrawalRecordId<T: Config>() -> WithdrawalRecordId {
        0
    }

    /// The id of next withdrawal record.
    #[pallet::storage]
    #[pallet::getter(fn id)]
    pub(crate) type NextWithdrawalRecordId<T: Config> =
        StorageValue<_, WithdrawalRecordId, ValueQuery, DefaultForWithdrawalRecordId<T>>;

    /// Withdraw applications collection, use serial numbers to mark them.
    #[pallet::storage]
    #[pallet::getter(fn pending_withdrawals)]
    pub(crate) type PendingWithdrawals<T: Config> =
        StorageMap<_, Twox64Concat, WithdrawalRecordId, WithdrawalRecordOf<T>>;

    /// The state of withdraw record corresponding to an id.
    #[pallet::storage]
    #[pallet::getter(fn state_of)]
    pub(crate) type WithdrawalStateOf<T: Config> =
        StorageMap<_, Twox64Concat, WithdrawalRecordId, WithdrawalState>;
}

impl<T: Config> Pallet<T> {
    fn ensure_asset_belongs_to_chain(asset_id: AssetId, expected_chain: Chain) -> DispatchResult {
        let asset_chain = xpallet_assets_registrar::Pallet::<T>::chain_of(&asset_id)?;
        ensure!(asset_chain == expected_chain, Error::<T>::UnexpectedChain);
        Ok(())
    }

    fn ensure_withdrawal_available_balance(
        who: &T::AccountId,
        asset_id: AssetId,
        value: BalanceOf<T>,
    ) -> DispatchResult {
        let available = xpallet_assets::Pallet::<T>::usable_balance(who, &asset_id);
        ensure!(
            available >= value,
            xpallet_assets::Error::<T>::InsufficientBalance
        );
        Ok(())
    }

    fn ensure_withdrawal_records_exists(
        id: WithdrawalRecordId,
    ) -> Result<(WithdrawalRecordOf<T>, WithdrawalState), DispatchError> {
        let record = Self::pending_withdrawals(id).ok_or(Error::<T>::NotExisted)?;
        let state = Self::state_of(id).ok_or(Error::<T>::NotExisted)?;
        Ok((record, state))
    }
}

impl<T: Config> Pallet<T> {
    /// Deposit asset.
    ///
    /// NOTE: this function has included deposit_init and deposit_finish (not wait for block confirm)
    pub fn deposit(who: &T::AccountId, asset_id: AssetId, balance: BalanceOf<T>) -> DispatchResult {
        xpallet_assets::Pallet::<T>::ensure_not_native_asset(&asset_id)?;

        info!(
            target: "runtime::gateway::records",
            "[deposit] who:{:?}, id:{}, balance:{:?}",
            who, asset_id, balance
        );

        xpallet_assets::Pallet::<T>::issue(&asset_id, who, balance)?;
        Self::deposit_event(Event::<T>::Deposited(who.clone(), asset_id, balance));
        Ok(())
    }

    /// Withdrawal asset (lock asset token firstly, follow-up operations are required).
    ///
    /// WithdrawalRecord State: `Applying`
    ///
    /// NOTE: this function has included withdrawal_init and withdrawal_locking.
    pub fn withdraw(
        who: &T::AccountId,
        asset_id: AssetId,
        balance: BalanceOf<T>,
        addr: AddrStr,
        ext: Memo,
    ) -> DispatchResult {
        xpallet_assets::Pallet::<T>::ensure_not_native_asset(&asset_id)?;
        Self::ensure_withdrawal_available_balance(who, asset_id, balance)?;

        let id = Self::id();
        info!(
            target: "runtime::gateway::records",
            "[apply_withdrawal] id:{}, who:{:?}, asset id:{}, balance:{:?}, addr:{:?}, memo:{}",
            id,
            who,
            asset_id,
            balance,
            try_addr(&addr),
            ext
        );
        let height = frame_system::Pallet::<T>::block_number();
        let record =
            WithdrawalRecordOf::<T>::new(who.clone(), asset_id, balance, addr, ext, height);

        // Lock usable asset token
        Self::lock(record.applicant(), record.asset_id(), record.balance())?;

        // Set storages
        PendingWithdrawals::<T>::insert(id, record.clone());
        WithdrawalStateOf::<T>::insert(id, WithdrawalState::Applying);
        let next_id = id.checked_add(1_u32).unwrap_or(0);
        NextWithdrawalRecordId::<T>::put(next_id);

        Self::deposit_event(Event::<T>::WithdrawalCreated(id, record));
        Ok(())
    }

    /// Process withdrawal (cannot be canceled, but can be recovered).
    ///
    /// WithdrawalRecord State: `Applying` ==> `Processing`
    pub fn process_withdrawal(id: WithdrawalRecordId, chain: Chain) -> DispatchResult {
        let (record, curr_state) = Self::ensure_withdrawal_records_exists(id)?;
        Self::ensure_asset_belongs_to_chain(record.asset_id(), chain)?;
        Self::process_withdrawal_impl(id, curr_state)
    }

    fn process_withdrawal_impl(
        id: WithdrawalRecordId,
        curr_state: WithdrawalState,
    ) -> DispatchResult {
        if curr_state != WithdrawalState::Applying {
            error!(
                target: "runtime::gateway::records",
                "[process_withdrawal] id:{}, current withdrawal state ({:?}) must be `Applying`",
                id, curr_state
            );
            return Err(Error::<T>::NotApplyingState.into());
        }
        WithdrawalStateOf::<T>::insert(id, WithdrawalState::Processing);
        Self::deposit_event(Event::<T>::WithdrawalProcessed(id));
        Ok(())
    }

    /// Process withdrawal in batches.
    #[transactional]
    pub fn process_withdrawals(ids: &[WithdrawalRecordId], chain: Chain) -> DispatchResult {
        for id in ids {
            Self::process_withdrawal(*id, chain)?;
        }
        Ok(())
    }

    /// Recover withdrawal.
    ///
    /// WithdrawalRecord State: `Processing` ==> `Applying`
    pub fn recover_withdrawal(id: WithdrawalRecordId, chain: Chain) -> DispatchResult {
        let (record, curr_state) = Self::ensure_withdrawal_records_exists(id)?;
        Self::ensure_asset_belongs_to_chain(record.asset_id(), chain)?;
        Self::recover_withdrawal_impl(id, curr_state)
    }

    fn recover_withdrawal_impl(
        id: WithdrawalRecordId,
        curr_state: WithdrawalState,
    ) -> DispatchResult {
        if curr_state != WithdrawalState::Processing {
            error!(
                target: "runtime::gateway::records",
                "[recover_withdrawal] id:{}, current withdrawal state ({:?}) must be `Processing`",
                id, curr_state
            );
            return Err(Error::<T>::NotProcessingState.into());
        }
        WithdrawalStateOf::<T>::insert(id, WithdrawalState::Applying);
        Self::deposit_event(Event::<T>::WithdrawalRecovered(id));
        Ok(())
    }

    /// Cancel withdrawal
    ///
    /// WithdrawalRecord State: `Applying` ==> `NormalCancel`
    pub fn cancel_withdrawal(id: WithdrawalRecordId, who: &T::AccountId) -> DispatchResult {
        let (record, curr_state) = Self::ensure_withdrawal_records_exists(id)?;
        if record.applicant() != who {
            error!(
                target: "runtime::gateway::records",
                "[cancel_withdrawal] id:{}, account {:?} is not the applicant {:?}",
                id,
                who,
                record.applicant()
            );
            return Err(Error::<T>::InvalidAccount.into());
        }

        Self::cancel_withdrawal_impl(id, record, curr_state, WithdrawalState::NormalCancel)
    }

    fn cancel_withdrawal_impl(
        id: WithdrawalRecordId,
        record: WithdrawalRecordOf<T>,
        curr_state: WithdrawalState,
        new_state: WithdrawalState,
    ) -> DispatchResult {
        if curr_state != WithdrawalState::Applying {
            error!(
                target: "runtime::gateway::records",
                "[cancel_withdrawal] id:{}, current withdrawal state ({:?}) must be `Applying`",
                id, curr_state
            );
            return Err(Error::<T>::NotApplyingState.into());
        }

        // Unlock reserved asset
        Self::unlock(record.applicant(), record.asset_id(), record.balance())?;

        // Remove storage
        PendingWithdrawals::<T>::remove(id);
        WithdrawalStateOf::<T>::remove(id);

        Self::deposit_event(Event::<T>::WithdrawalCanceled(id, new_state));
        Ok(())
    }

    /// Finish withdrawal, destroy the reserved withdrawal asset token.
    ///
    /// WithdrawalRecord State: `Processing` ==> `NormalFinish`
    ///
    /// NOTE:
    /// when the withdrawal id is passed by runtime self logic, just pass `None`,
    /// when the withdrawal id is passed by the parameter from call(which means the id is from outside),
    /// should pass `Some(chain)` to verify whether the withdrawal is related to this chain.
    ///
    /// e.g. bitcoin release reserved by receive a valid withdrawal transaction, the withdraw id is
    /// valid when trustees submit withdrawal info, so that just release it directly.
    /// ethereum released reserved by trustees submit release request directly, so that we should check
    /// whether the withdrawal belongs to Ethereum Chain, in case release other chain withdraw.
    pub fn finish_withdrawal(
        id: WithdrawalRecordId,
        expected_chain: Option<Chain>,
    ) -> DispatchResult {
        let (record, curr_state) = Self::ensure_withdrawal_records_exists(id)?;
        if let Some(chain) = expected_chain {
            Self::ensure_asset_belongs_to_chain(record.asset_id(), chain)?;
        }
        Self::finish_withdrawal_impl(id, record, curr_state, WithdrawalState::NormalFinish)
    }

    fn finish_withdrawal_impl(
        id: WithdrawalRecordId,
        record: WithdrawalRecordOf<T>,
        curr_state: WithdrawalState,
        new_state: WithdrawalState,
    ) -> DispatchResult {
        if curr_state != WithdrawalState::Processing {
            error!(
                target: "runtime::gateway::records",
                "[finish_withdrawal] id:{}, current withdrawal state ({:?}) must be `Processing`",
                id, curr_state
            );
            return Err(Error::<T>::NotProcessingState.into());
        }

        // Destroy locked asset
        Self::destroy(record.applicant(), record.asset_id(), record.balance())?;

        // Remove storage
        PendingWithdrawals::<T>::remove(id);
        WithdrawalStateOf::<T>::remove(id);

        Self::deposit_event(Event::<T>::WithdrawalFinished(id, new_state));
        Ok(())
    }

    /// Finish withdrawal in batches.
    #[transactional]
    pub fn finish_withdrawals(
        ids: &[WithdrawalRecordId],
        expected_chain: Option<Chain>,
    ) -> DispatchResult {
        for id in ids {
            Self::finish_withdrawal(*id, expected_chain)?;
        }
        Ok(())
    }

    pub fn set_withdrawal_state_by_root(
        id: WithdrawalRecordId,
        new_state: WithdrawalState,
    ) -> DispatchResult {
        let (record, curr_state) = Self::ensure_withdrawal_records_exists(id)?;
        match (curr_state, new_state) {
            (curr, new) if curr == new => Ok(()),
            (WithdrawalState::Applying, WithdrawalState::Processing) => {
                // State: `Applying` ==> `Processing`
                Self::process_withdrawal_impl(id, curr_state)
            }
            (WithdrawalState::Processing, WithdrawalState::Applying) => {
                // State: `Processing` ==> `Applying`
                Self::recover_withdrawal_impl(id, curr_state)
            }
            (WithdrawalState::Applying, WithdrawalState::NormalCancel)
            | (WithdrawalState::Applying, WithdrawalState::RootCancel) => {
                // State: `Applying` ==> `NormalCancel`|`RootCancel`
                Self::cancel_withdrawal_impl(id, record, curr_state, new_state)
            }
            (WithdrawalState::Applying, WithdrawalState::NormalFinish)
            | (WithdrawalState::Applying, WithdrawalState::RootFinish) => {
                // State: `Applying` ==> `Processing` ==> `NormalFinish`|`RootFinish`
                Self::process_withdrawal_impl(id, curr_state)?;
                let curr_state = Self::state_of(id).ok_or(Error::<T>::NotExisted)?;
                Self::finish_withdrawal_impl(id, record, curr_state, new_state)
            }
            (WithdrawalState::Processing, WithdrawalState::NormalFinish)
            | (WithdrawalState::Processing, WithdrawalState::RootFinish) => {
                // State: `Processing` ==> `NormalFinish`|`RootFinish`
                Self::finish_withdrawal_impl(id, record, curr_state, new_state)
            }
            _ => {
                error!(
                    target: "runtime::gateway::records",
                    "[set_withdrawal_state_by_root] Shouldn't happen normally, unless called by root, \
                    current state:{:?}, new state:{:?}",
                    curr_state, new_state
                );
                Err("Do not expect this state in set_withdrawal_state_by_root".into())
            }
        }
    }

    pub fn set_withdrawal_state_by_trustees(
        id: WithdrawalRecordId,
        chain: Chain,
        new_state: WithdrawalState,
    ) -> DispatchResult {
        let (record, state) = Self::ensure_withdrawal_records_exists(id)?;
        Self::ensure_asset_belongs_to_chain(record.asset_id(), chain)?;
        if state != WithdrawalState::Processing {
            error!(
                target: "runtime::gateway::records",
                "[set_withdrawal_state_by_trustees] id:{}, current withdrawal state ({:?}) must be `Processing`",
                id, state
            );
            return Err(Error::<T>::NotProcessingState.into());
        }

        match new_state {
            WithdrawalState::RootFinish | WithdrawalState::RootCancel => { /*do nothing*/ }
            _ => {
                error!(
                    target: "runtime::gateway::records",
                    "[set_withdrawal_state_by_trustees] id:{}, new withdrawal state ({:?}) must be `RootFinish` or `RootCancel`",
                    id, new_state
                );
                return Err(Error::<T>::InvalidState.into());
            }
        }
        Self::set_withdrawal_state(frame_system::RawOrigin::Root.into(), id, new_state)
    }

    fn lock(who: &T::AccountId, asset_id: AssetId, value: BalanceOf<T>) -> DispatchResult {
        xpallet_assets::Pallet::<T>::move_balance(
            &asset_id,
            who,
            AssetType::Usable,
            who,
            AssetType::ReservedWithdrawal,
            value,
        )
        .map_err::<xpallet_assets::Error<T>, _>(Into::into)?;
        Ok(())
    }

    fn unlock(who: &T::AccountId, asset_id: AssetId, value: BalanceOf<T>) -> DispatchResult {
        xpallet_assets::Pallet::<T>::move_balance(
            &asset_id,
            who,
            AssetType::ReservedWithdrawal,
            who,
            AssetType::Usable,
            value,
        )
        .map_err::<xpallet_assets::Error<T>, _>(Into::into)?;
        Ok(())
    }

    fn destroy(who: &T::AccountId, asset_id: AssetId, value: BalanceOf<T>) -> DispatchResult {
        xpallet_assets::Pallet::<T>::destroy_reserved_withdrawal(&asset_id, who, value)?;
        Ok(())
    }

    #[inline]
    pub fn pending_withdrawal_set(
    ) -> impl Iterator<Item = (WithdrawalRecordId, WithdrawalRecordOf<T>)> {
        PendingWithdrawals::<T>::iter()
    }
}

impl<T: Config> Pallet<T> {
    pub fn withdrawal_list(
    ) -> BTreeMap<WithdrawalRecordId, Withdrawal<T::AccountId, BalanceOf<T>, T::BlockNumber>> {
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
    ) -> BTreeMap<WithdrawalRecordId, Withdrawal<T::AccountId, BalanceOf<T>, T::BlockNumber>> {
        Self::withdrawal_list()
            .into_iter()
            .filter(|(_, withdrawal)| {
                Self::ensure_asset_belongs_to_chain(withdrawal.asset_id, chain).is_ok()
            })
            .collect()
    }

    pub fn withdrawal_state_insert(id: WithdrawalRecordId, state: WithdrawalState) {
        WithdrawalStateOf::<T>::insert(id, state)
    }
}
