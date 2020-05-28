// Copyright 2018-2019 Chainpool.
//! Vote weight calculation.

use super::*;

use xsupport::{debug, error, trace};

/// Compute the dividend by a ration of source_vote_weight/target_vote_weight.
///
/// dividend = source_vote_weight/target_vote_weight * balance_of(claimee_jackpot)
pub fn compute_dividend<T: Trait>(
    source_vote_weight: u128,
    target_vote_weight: u128,
    claimee_jackpot: &T::AccountId,
) -> T::Balance {
    let total_jackpot = xassets::Module::<T>::pcx_free_balance(&claimee_jackpot);
    let dividend = match source_vote_weight.checked_mul(total_jackpot.saturated_into()) {
        Some(x) => ((x / target_vote_weight) as u64).into(),
        None => {
            error!(
                "[compute_dividvid] overflow: source_vote_weight({:?}) * total_jackpot({:?})",
                source_vote_weight, total_jackpot
            );
            panic!("source_vote_weight * total_jackpot overflow")
        }
    };

    trace!(
        target: "claim",
        "[compute_dividvid] source_vote_weight/target_vote_weight: {:?}/{:?}, total_jackpot: {:?}, dividend: {:?}",
        source_vote_weight, target_vote_weight, total_jackpot, dividend
    );

    dividend
}

impl<T: Trait> Module<T> {
    pub(super) fn deposit_claim_event(
        source_weight_info: (u128, bool),
        target_weight_info: (u128, bool),
        _source: &T::AccountId,
        _target: &T::AccountId,
        dividend: T::Balance,
    ) {
        let (source_vote_weight, source_overflow) = source_weight_info;
        let (target_vote_weight, target_overflow) = target_weight_info;
        if !source_overflow && !target_overflow {
            Self::deposit_event(RawEvent::Claim(
                source_vote_weight as u64,
                target_vote_weight as u64,
                dividend,
            ));
        } else {
            Self::deposit_event(RawEvent::ClaimV1(
                source_vote_weight,
                target_vote_weight,
                dividend,
            ));
        }
    }

    pub(super) fn apply_update_staker_vote_weight(
        source: &T::AccountId,
        target: &T::AccountId,
        source_vote_weight: u128,
        current_block: T::BlockNumber,
        delta: &Delta,
    ) {
        let key = (source.clone(), target.clone());

        let record_result = Self::try_get_nomination_record(&key);
        if record_result.is_ok() && source_vote_weight <= u128::from(u64::max_value()) {
            let mut record = match record_result {
                Ok(record) => record,
                _ => panic!("Impossible, checked already; qed"),
            };
            record.set_state(source_vote_weight as u64, current_block, delta);
            <NominationRecords<T>>::insert(&key, record);
        } else {
            let mut record_v1 = match record_result {
                Ok(record) => {
                    debug!("[switch_to_u128] remove {:?} from NominationRecords due to the source_vote_weight is overflow: {:?}", &key, source_vote_weight);
                    <NominationRecords<T>>::remove(&key);
                    record.into()
                }
                Err(record_v1) => record_v1,
            };
            record_v1.set_state(source_vote_weight, current_block, delta);
            <NominationRecordsV1<T>>::insert(&key, record_v1);
        }
    }

    pub(super) fn apply_update_intention_vote_weight(
        target: &T::AccountId,
        new_target_vote_weight: u128,
        current_block: T::BlockNumber,
        delta: &Delta,
    ) {
        let iprof_result = Self::try_get_intention_profs(target);

        if iprof_result.is_ok() && new_target_vote_weight <= u128::from(u64::max_value()) {
            let mut iprof = match iprof_result {
                Ok(iprof) => iprof,
                _ => panic!("Impossible, checked already; qed"),
            };
            iprof.set_state(new_target_vote_weight as u64, current_block, delta);
            <Intentions<T>>::insert(target, iprof);
        } else {
            let mut iprof_v1 = match iprof_result {
                Ok(iprof) => {
                    debug!("[switch_to_u128] remove {:?} from Intentions due to the new_target_vote_weight is overflow: {:?}", target, new_target_vote_weight);
                    <Intentions<T>>::remove(target);
                    iprof.into()
                }
                Err(iprof_v1) => iprof_v1,
            };
            iprof_v1.set_state(new_target_vote_weight, current_block, delta);
            <IntentionsV1<T>>::insert(target, iprof_v1);
        }
    }

    pub(super) fn apply_state_change_on_claim(
        who: &T::AccountId,
        target: &T::AccountId,
        new_target_vote_weight: u128,
        current_block: T::BlockNumber,
    ) {
        Self::apply_update_staker_vote_weight(who, target, 0, current_block, &Delta::Zero);
        Self::apply_update_intention_vote_weight(
            target,
            new_target_vote_weight,
            current_block,
            &Delta::Zero,
        );
    }
}
