use super::*;
use codec::Encode;
use sp_arithmetic::traits::BaseArithmetic;
use sp_core::crypto::UncheckedFrom;
use sp_runtime::traits::Hash;
use xp_mining_common::{
    generic_weight_factors, BaseMiningWeight, Claim, ComputeMiningWeight, RewardPotAccountFor,
    WeightFactors, WeightType,
};
use xp_mining_staking::SessionIndex;

impl<Balance, BlockNumber> BaseMiningWeight<Balance, BlockNumber>
    for ValidatorLedger<Balance, BlockNumber>
where
    Balance: Default + BaseArithmetic + Copy,
    BlockNumber: Default + BaseArithmetic + Copy,
{
    fn amount(&self) -> Balance {
        self.total
    }

    fn set_amount(&mut self, new: Balance) {
        self.total = new;
    }

    fn last_acum_weight(&self) -> WeightType {
        self.last_total_vote_weight
    }

    fn set_last_acum_weight(&mut self, latest_vote_weight: WeightType) {
        self.last_total_vote_weight = latest_vote_weight;
    }

    fn last_acum_weight_update(&self) -> BlockNumber {
        self.last_total_vote_weight_update
    }

    fn set_last_acum_weight_update(&mut self, current_block: BlockNumber) {
        self.last_total_vote_weight_update = current_block;
    }
}

impl<Balance, BlockNumber> BaseMiningWeight<Balance, BlockNumber>
    for NominatorLedger<Balance, BlockNumber>
where
    Balance: Default + BaseArithmetic + Copy,
    BlockNumber: Default + BaseArithmetic + Copy,
{
    fn amount(&self) -> Balance {
        self.nomination
    }

    fn set_amount(&mut self, new: Balance) {
        self.nomination = new;
    }

    fn last_acum_weight(&self) -> WeightType {
        self.last_vote_weight
    }

    fn set_last_acum_weight(&mut self, latest_vote_weight: WeightType) {
        self.last_vote_weight = latest_vote_weight;
    }

    fn last_acum_weight_update(&self) -> BlockNumber {
        self.last_vote_weight_update
    }

    fn set_last_acum_weight_update(&mut self, current_block: BlockNumber) {
        self.last_vote_weight_update = current_block;
    }
}

impl<T: Trait> ComputeMiningWeight<T::AccountId, T::BlockNumber> for Module<T> {
    type Claimee = T::AccountId;
    type Error = Error<T>;

    fn claimer_weight_factors(
        who: &T::AccountId,
        target: &Self::Claimee,
        current_block: T::BlockNumber,
    ) -> WeightFactors {
        let claimer_ledger = Nominations::<T>::get(who, target);
        generic_weight_factors::<T::Balance, T::BlockNumber, _>(claimer_ledger, current_block)
    }

    fn claimee_weight_factors(
        target: &Self::Claimee,
        current_block: T::BlockNumber,
    ) -> WeightFactors {
        let claimee_ledger = ValidatorLedgers::<T>::get(target);
        generic_weight_factors::<T::Balance, T::BlockNumber, _>(claimee_ledger, current_block)
    }
}

/// Computes the dividend according to the ratio of source_vote_weight/target_vote_weight.
///
/// dividend = source_vote_weight/target_vote_weight * balance_of(claimee_reward_pot)
pub fn compute_dividend<T: Trait>(
    source_vote_weight: WeightType,
    target_vote_weight: WeightType,
    claimee_reward_pot: &T::AccountId,
) -> T::Balance {
    let total_reward_pot = xpallet_assets::Module::<T>::pcx_free_balance(&claimee_reward_pot);
    match source_vote_weight.checked_mul(total_reward_pot.saturated_into()) {
        Some(x) => ((x / target_vote_weight) as u64).saturated_into(),
        None => panic!("source_vote_weight * total_reward_pot overflow, this should not happen"),
    }
}

impl<T: Trait> Module<T> {
    fn allocate_dividend(
        claimer: &T::AccountId,
        pot_account: &T::AccountId,
        dividend: T::Balance,
    ) -> Result<(), AssetErr> {
        xpallet_assets::Module::<T>::pcx_move_free_balance(pot_account, claimer, dividend)
    }

    /// Calculates the new amount given the origin amount and delta
    fn apply_delta(origin: T::Balance, delta: Delta<T::Balance>) -> T::Balance {
        match delta {
            Delta::Add(v) => origin + v,
            Delta::Sub(v) => origin - v,
            Delta::Zero => origin,
        }
    }

    /// Actually update the nominator vote weight given the new vote weight, block number and amount delta.
    pub(crate) fn set_nominator_vote_weight(
        nominator: &T::AccountId,
        validator: &T::AccountId,
        new_weight: WeightType,
        current_block: T::BlockNumber,
        delta: Delta<T::Balance>,
    ) {
        Nominations::<T>::mutate(nominator, validator, |claimer_ledger| {
            claimer_ledger.nomination = Self::apply_delta(claimer_ledger.nomination, delta);
            claimer_ledger.last_vote_weight = new_weight;
            claimer_ledger.last_vote_weight_update = current_block;
        });
    }

    ///
    pub(crate) fn set_validator_vote_weight(
        validator: &T::AccountId,
        new_weight: WeightType,
        current_block: T::BlockNumber,
        delta: Delta<T::Balance>,
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
        new_vote_weight: WeightType,
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

        let (source_weight, target_weight) = <Self as ComputeMiningWeight<
            T::AccountId,
            T::BlockNumber,
        >>::settle_weight_on_claim(
            claimer, claimee, current_block
        )?;

        let claimee_pot = T::DetermineRewardPotAccount::reward_pot_account_for(claimee);

        let dividend = compute_dividend::<T>(source_weight, target_weight, &claimee_pot);

        Self::allocate_dividend(claimer, &claimee_pot, dividend)?;

        Self::deposit_event(RawEvent::Claim(claimer.clone(), claimee.clone(), dividend));

        let new_target_weight = target_weight - source_weight;

        Self::update_claimer_vote_weight_on_claim(claimer, claimee, current_block);
        Self::update_claimee_vote_weight_on_claim(claimee, new_target_weight, current_block);

        Ok(())
    }
}

impl<T: Trait> Module<T> {
    fn new_session(session_index: SessionIndex) -> Option<Vec<T::AccountId>> {
        if let Some(current_era) = Self::current_era() {
            // Initial era has been set.

            let current_era_start_session_index = Self::eras_start_session_index(current_era)
                .unwrap_or_else(|| {
                    frame_support::print("Error: start_session_index must be set for current_era");
                    0
                });

            let era_length = session_index
                .checked_sub(current_era_start_session_index)
                .unwrap_or(0); // Must never happen.

            let ideal_era_length = Self::sessions_per_era().saturated_into::<SessionIndex>();

            match ForceEra::get() {
                Forcing::ForceNew => ForceEra::kill(),
                Forcing::ForceAlways => (),
                Forcing::NotForcing if era_length >= ideal_era_length => (),
                _ => {
                    // Either `ForceNone`, or `NotForcing && era_length < T::SessionsPerEra::get()`.
                    if era_length + 1 == ideal_era_length {
                        IsCurrentSessionFinal::put(true);
                    } else if era_length >= ideal_era_length {
                        // Should only happen when we are ready to trigger an era but we have ForceNone,
                        // otherwise previous arm would short circuit.
                        // FIXME: figure out this
                        // Self::close_election_window();
                    }
                    return None;
                }
            }

            // new era.
            Self::new_era(session_index)
        } else {
            // Set initial era
            Self::new_era(session_index)
        }
    }

    fn start_session(start_index: SessionIndex) {
        todo!()
    }

    fn end_session(end_index: SessionIndex) {
        todo!()
    }
}

/// In this implementation `new_session(session)` must be called before `end_session(session-1)`
/// i.e. the new session must be planned before the ending of the previous session.
///
/// Once the first new_session is planned, all session must start and then end in order, though
/// some session can lag in between the newest session planned and the latest session started.
impl<T: Trait> pallet_session::SessionManager<T::AccountId> for Module<T> {
    fn new_session(new_index: SessionIndex) -> Option<Vec<T::AccountId>> {
        Self::new_session(new_index)
    }
    fn start_session(start_index: SessionIndex) {
        Self::start_session(start_index)
    }
    fn end_session(end_index: SessionIndex) {
        Self::end_session(end_index)
    }
}

/// Simple validator reward pot account determiner.
///
/// Formula: `blake2_256(blake2_256(validator_pubkey) + blake2_256(registered_at))`
pub struct SimpleValidatorRewardPotAccountDeterminer<T: Trait>(sp_std::marker::PhantomData<T>);

impl<T: Trait> xp_mining_common::RewardPotAccountFor<T::AccountId, T::AccountId>
    for SimpleValidatorRewardPotAccountDeterminer<T>
where
    T::AccountId: UncheckedFrom<T::Hash> + AsRef<[u8]>,
{
    fn reward_pot_account_for(validator: &T::AccountId) -> T::AccountId {
        let validator_hash = <T as frame_system::Trait>::Hashing::hash(validator.as_ref());
        let registered_at: T::BlockNumber = Validators::<T>::get(validator).registered_at;
        let registered_at_hash =
            <T as frame_system::Trait>::Hashing::hash(registered_at.encode().as_ref());

        let mut buf = Vec::new();
        buf.extend_from_slice(validator_hash.as_ref());
        buf.extend_from_slice(registered_at_hash.as_ref());

        UncheckedFrom::unchecked_from(T::Hashing::hash(&buf[..]))
    }
}
