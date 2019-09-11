// Copyright 2018-2019 Chainpool.

use super::*;

impl<T: Trait> ComputeWeight<T::AccountId> for Module<T> {
    type Claimee = T::AccountId;

    /// current_block is ensured to be no less than the last_acum_weight_update()
    fn prepare_claimer_weight_factors(
        who: &T::AccountId,
        target: &Self::Claimee,
        current_block: u64,
    ) -> WeightFactors {
        let key = (who.clone(), target.clone());
        match Self::try_get_nomination_record(&key) {
            Ok(v) => (
                WeightType::U64(v.last_vote_weight),
                v.amount(),
                current_block - v.last_acum_weight_update(),
            ),
            Err(v1) => (
                WeightType::U128(v1.last_vote_weight),
                v1.amount(),
                current_block - v1.last_acum_weight_update(),
            ),
        }
    }

    fn prepare_claimee_weight_factors(target: &Self::Claimee, current_block: u64) -> WeightFactors {
        match Self::try_get_intention_profs(target) {
            Ok(i) => (
                WeightType::U64(i.last_total_vote_weight),
                i.amount(),
                current_block - i.last_acum_weight_update(),
            ),
            Err(i1) => (
                WeightType::U128(i1.last_total_vote_weight),
                i1.amount(),
                current_block - i1.last_acum_weight_update(),
            ),
        }
    }
}

impl<T: Trait> Claim<T::AccountId, T::Balance> for Module<T> {
    type Claimee = T::AccountId;

    fn allocate_dividend(
        claimer: &T::AccountId,
        _claimee: &Self::Claimee,
        claimee_jackpot: &T::AccountId,
        dividend: T::Balance,
    ) -> Result {
        xassets::Module::<T>::pcx_move_free_balance(claimee_jackpot, claimer, dividend)
                    .map_err(|e| {
                        error!(
                            "[allocate staker dividend] fail to move {:?} from jackpot_addr to some nominator as current jackpot_balance is not sufficient: {:?}",
                            dividend,
                            xassets::Module::<T>::pcx_free_balance(claimee_jackpot),
                        );
                        e.info()
                    })
    }

    fn claim(claimer: &T::AccountId, claimee: &Self::Claimee) -> Result {
        let current_block = <system::Module<T>>::block_number();

        let ((source_vote_weight, source_overflow), (target_vote_weight, target_overflow)) =
            <Self as ComputeWeight<T::AccountId>>::settle_weight_on_claim(
                claimer,
                claimee,
                current_block.as_(),
            )?;

        let claimee_jackpot = xstaking::Module::<T>::jackpot_accountid_for_unsafe(claimee);

        let dividend =
            compute_dividend::<T>(source_vote_weight, target_vote_weight, &claimee_jackpot);

        Self::allocate_dividend(claimer, claimee, &claimee_jackpot, dividend)?;

        xstaking::Module::<T>::deposit_claim_event(
            (source_vote_weight, source_overflow),
            (target_vote_weight, target_overflow),
            claimer,
            claimee,
            dividend,
        );

        let new_target_vote_weight = target_vote_weight - source_vote_weight;

        xstaking::Module::<T>::apply_state_change_on_claim(
            claimer,
            claimee,
            new_target_vote_weight,
            current_block,
        );

        Ok(())
    }
}
