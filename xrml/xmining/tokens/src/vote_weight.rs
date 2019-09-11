// Copyright 2018-2019 Chainpool.

use super::*;
use xstaking::{VoteWeight, VoteWeightV1};

impl<T: Trait> Module<T> {
    pub(super) fn update_depositor_vote_weight(
        from: &T::AccountId,
        target: &Token,
        current_block: T::BlockNumber,
    ) {
        let (new_deposit_weight, _overflow) =
            <Self as ComputeWeight<T::AccountId>>::settle_claimer_weight(
                from,
                target,
                current_block.as_(),
            );

        Self::apply_update_depositor_vote_weight(from, target, new_deposit_weight, current_block);
    }

    pub(super) fn apply_update_depositor_vote_weight(
        from: &T::AccountId,
        target: &Token,
        new_deposit_weight: u128,
        current_block: T::BlockNumber,
    ) {
        let key = (from.clone(), target.clone());

        let record_result = Self::try_get_deposit_record(&key);

        if record_result.is_ok() && new_deposit_weight <= u128::from(u64::max_value()) {
            let mut d = match record_result {
                Ok(d) => d,
                _ => panic!("Impossible, checked already; qed"),
            };
            let mut record = DepositRecord::<T>::new(from, target, &mut d);
            record.set_state_weight(new_deposit_weight as u64, current_block);
            <DepositRecords<T>>::insert(&key, d);
        } else {
            let mut d1 = match record_result {
                Ok(d) => {
                    debug!("[switch_to_u128] remove {:?} from DepositRecords due to the new_deposit_weight is overflow: {:?}", &key, new_deposit_weight);
                    <DepositRecords<T>>::remove(&key);
                    d.into()
                }
                Err(d1) => d1,
            };
            let mut record_v1 = DepositRecordV1::<T>::new(from, target, &mut d1);
            record_v1.set_state_weight(new_deposit_weight, current_block);
            <DepositRecordsV1<T>>::insert(&key, d1);
        }
    }

    pub(super) fn apply_update_psedu_intention_vote_weight(
        target: &Token,
        new_deposit_weight: u128,
        current_block: T::BlockNumber,
    ) {
        let pprof_result = Self::try_get_psedu_intention_profs(target);

        if pprof_result.is_ok() && new_deposit_weight <= u128::from(u64::max_value()) {
            let mut p = match pprof_result {
                Ok(p) => p,
                _ => panic!("Impossible, checked already; qed"),
            };
            let mut prof = PseduIntentionProfs::<T>::new(target, &mut p);
            prof.set_state_weight(new_deposit_weight as u64, current_block);
            <PseduIntentionProfiles<T>>::insert(target, p);
        } else {
            let mut p1 = match pprof_result {
                Ok(p) => {
                    debug!("[switch_to_u128] remove {:?} from PseduIntentionProfiles due to the new_deposit_weight is overflow: {:?}", target, new_deposit_weight);
                    <PseduIntentionProfiles<T>>::remove(target);
                    p.into()
                }
                Err(p1) => p1,
            };
            let mut prof_v1 = PseduIntentionProfsV1::<T>::new(target, &mut p1);
            prof_v1.set_state_weight(new_deposit_weight, current_block);
            <PseduIntentionProfilesV1<T>>::insert(target, p1);
        }
    }

    pub(super) fn update_psedu_intention_vote_weight(
        target: &Token,
        current_block: T::BlockNumber,
    ) {
        let (new_deposit_weight, _overflow) =
            <Self as ComputeWeight<T::AccountId>>::settle_claimee_weight(
                target,
                current_block.as_(),
            );
        Self::apply_update_psedu_intention_vote_weight(target, new_deposit_weight, current_block);
    }

    pub(super) fn apply_state_change_on_claim(
        who: &T::AccountId,
        target: &Token,
        new_target_vote_weight: u128,
        current_block: T::BlockNumber,
    ) {
        Self::apply_update_depositor_vote_weight(who, target, 0, current_block);
        Self::apply_update_psedu_intention_vote_weight(
            target,
            new_target_vote_weight,
            current_block,
        );
    }
}
