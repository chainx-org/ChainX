use super::*;
use sp_arithmetic::traits::BaseArithmetic;
use xp_staking::{BaseVoteWeight, Claim, ComputeVoteWeight, Delta, VoteWeight, WeightFactors};

impl<Balance, BlockNumber> BaseVoteWeight<BlockNumber> for ValidatorLedger<Balance, BlockNumber>
where
    Balance: Default + BaseArithmetic + Copy,
    BlockNumber: Default + BaseArithmetic + Copy,
{
    fn amount(&self) -> u64 {
        self.total.saturated_into()
    }

    fn set_amount(&mut self, new: u64) {
        self.total = new.saturated_into();
    }

    fn last_acum_weight(&self) -> VoteWeight {
        self.last_total_vote_weight
    }

    fn set_last_acum_weight(&mut self, latest_vote_weight: VoteWeight) {
        self.last_total_vote_weight = latest_vote_weight;
    }

    fn last_acum_weight_update(&self) -> u64 {
        self.last_total_vote_weight_update.saturated_into::<u64>()
    }

    fn set_last_acum_weight_update(&mut self, current_block: BlockNumber) {
        self.last_total_vote_weight_update = current_block;
    }
}

impl<Balance, BlockNumber> BaseVoteWeight<BlockNumber> for NominatorLedger<Balance, BlockNumber>
where
    Balance: Default + BaseArithmetic + Copy,
    BlockNumber: Default + BaseArithmetic + Copy,
{
    fn amount(&self) -> u64 {
        self.value.saturated_into()
    }

    fn set_amount(&mut self, new: u64) {
        self.value = new.saturated_into();
    }

    fn last_acum_weight(&self) -> VoteWeight {
        self.last_vote_weight
    }

    fn set_last_acum_weight(&mut self, latest_vote_weight: VoteWeight) {
        self.last_vote_weight = latest_vote_weight;
    }

    fn last_acum_weight_update(&self) -> u64 {
        self.last_vote_weight_update.saturated_into::<u64>()
    }

    fn set_last_acum_weight_update(&mut self, current_block: BlockNumber) {
        self.last_vote_weight_update = current_block;
    }
}

impl<T: Trait> ComputeVoteWeight<T::AccountId> for Module<T> {
    type Claimee = T::AccountId;
    type Error = Error<T>;

    fn claimer_weight_factors(
        who: &T::AccountId,
        target: &Self::Claimee,
        current_block: u64,
    ) -> WeightFactors {
        let claimer_ledger = Nominations::<T>::get(who, target);
        (
            claimer_ledger.last_vote_weight,
            claimer_ledger.amount(),
            current_block - claimer_ledger.last_acum_weight_update(),
        )
    }

    fn claimee_weight_factors(target: &Self::Claimee, current_block: u64) -> WeightFactors {
        let claimee_ledger = ValidatorLedgers::<T>::get(target);
        (
            claimee_ledger.last_total_vote_weight,
            claimee_ledger.amount(),
            current_block - claimee_ledger.last_acum_weight_update(),
        )
    }
}

/// Computes the dividend according to the ratio of source_vote_weight/target_vote_weight.
///
/// dividend = source_vote_weight/target_vote_weight * balance_of(claimee_jackpot)
pub fn compute_dividend<T: Trait>(
    source_vote_weight: VoteWeight,
    target_vote_weight: VoteWeight,
    claimee_jackpot: &T::AccountId,
) -> T::Balance {
    let total_jackpot = xpallet_assets::Module::<T>::pcx_free_balance(&claimee_jackpot);
    match source_vote_weight.checked_mul(total_jackpot.saturated_into()) {
        Some(x) => ((x / target_vote_weight) as u64).saturated_into(),
        None => panic!("source_vote_weight * total_jackpot overflow, this should not happen"),
    }
}

impl<T: Trait> Module<T> {
    fn jackpot_account_for(validator: &T::AccountId) -> T::AccountId {
        todo!()
    }

    fn allocate_dividend(
        claimer: &T::AccountId,
        pot_account: &T::AccountId,
        dividend: T::Balance,
    ) -> Result<(), AssetErr> {
        xpallet_assets::Module::<T>::pcx_move_free_balance(pot_account, claimer, dividend)
    }

    /// Calculates the new amount given the origin amount and delta
    fn apply_delta(origin: T::Balance, delta: Delta) -> T::Balance {
        match delta {
            Delta::Add(v) => origin + v.saturated_into(),
            Delta::Sub(v) => origin - v.saturated_into(),
            Delta::Zero => origin,
        }
    }

    /// Actually update the nominator vote weight given the new vote weight, block number and amount delta.
    pub(crate) fn set_nominator_vote_weight(
        nominator: &T::AccountId,
        validator: &T::AccountId,
        new_weight: VoteWeight,
        current_block: T::BlockNumber,
        delta: Delta,
    ) {
        Nominations::<T>::mutate(nominator, validator, |claimer_ledger| {
            claimer_ledger.value = Self::apply_delta(claimer_ledger.value, delta);
            claimer_ledger.last_vote_weight = new_weight;
            claimer_ledger.last_vote_weight_update = current_block;
        });
    }

    ///
    pub(crate) fn set_validator_vote_weight(
        validator: &T::AccountId,
        new_weight: VoteWeight,
        current_block: T::BlockNumber,
        delta: Delta,
    ) {
        ValidatorLedgers::<T>::mutate(validator, |validator_ledger| {
            validator_ledger.total = Self::apply_delta(validator_ledger.total, delta);
            validator_ledger.last_total_vote_weight = new_weight;
            validator_ledger.last_total_vote_weight_update = current_block;
        });
    }

    fn update_claimer_vote_weight_on_claim(
        claimer: &T::AccountId,
        target: &T::AccountId,
        current_block: T::BlockNumber,
    ) {
        Self::set_nominator_vote_weight(claimer, target, 0, current_block, Delta::Zero);
    }

    fn update_claimee_vote_weight_on_claim(
        claimee: &T::AccountId,
        new_vote_weight: VoteWeight,
        current_block: T::BlockNumber,
    ) {
        Self::set_validator_vote_weight(claimee, new_vote_weight, current_block, Delta::Zero);
    }
}

impl<T: Trait> Claim<T::AccountId> for Module<T> {
    type Claimee = T::AccountId;
    type Error = Error<T>;

    fn claim(claimer: &T::AccountId, claimee: &Self::Claimee) -> Result<(), Self::Error> {
        let current_block = <frame_system::Module<T>>::block_number();

        let (source_weight, target_weight) =
            <Self as ComputeVoteWeight<T::AccountId>>::settle_weight_on_claim(
                claimer,
                claimee,
                current_block.saturated_into::<u64>(),
            )?;

        let claimee_pot = Self::jackpot_account_for(claimee);

        let dividend = compute_dividend::<T>(source_weight, target_weight, &claimee_pot);

        Self::allocate_dividend(claimer, &claimee_pot, dividend)?;

        Self::deposit_event(RawEvent::Claim(claimer.clone(), claimee.clone(), dividend));

        let new_target_weight = target_weight - source_weight;

        Self::update_claimer_vote_weight_on_claim(claimer, claimee, current_block);
        Self::update_claimee_vote_weight_on_claim(claimee, new_target_weight, current_block);

        Ok(())
    }
}
