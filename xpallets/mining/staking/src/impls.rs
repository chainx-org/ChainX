use super::*;
use codec::{Decode, Encode};
use frame_support::traits::{LockIdentifier, WithdrawReasons};
use sp_arithmetic::traits::BaseArithmetic;
use sp_core::crypto::UncheckedFrom;
use sp_runtime::RuntimeDebug;
use sp_runtime::{traits::Hash, DispatchResult, Perbill};
#[cfg(feature = "std")]
use sp_runtime::{Deserialize, Serialize};
use sp_staking::offence::{OffenceDetails, OnOffenceHandler};
use xp_mining_common::{
    generic_weight_factors, BaseMiningWeight, Claim, ComputeMiningWeight, WeightFactors, WeightType,
};
use xp_mining_staking::{NativeReservableCurrency, SessionIndex};

const STAKING_ID: LockIdentifier = *b"staking ";

#[derive(PartialEq, PartialOrd, Ord, Eq, Clone, Copy, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum ReservedType {
    Bonded,
    BondedWithdrawal,
}

impl<T: Trait> NativeReservableCurrency<T::AccountId, BalanceOf<T>, ReservedType> for Module<T> {
    fn reserve(who: &T::AccountId, value: BalanceOf<T>) -> DispatchResult {
        // FIXME: figure out set_lock
        T::Currency::set_lock(STAKING_ID, who, value, WithdrawReasons::all());
        Ok(())
    }
    fn unreserve(who: &T::AccountId, value: BalanceOf<T>, ty: ReservedType) -> DispatchResult {
        Ok(())
    }
    fn move_reserved(
        who: &T::AccountId,
        value: BalanceOf<T>,
        from_ty: ReservedType,
        to_ty: ReservedType,
    ) -> DispatchResult {
        Ok(())
    }
}

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
        generic_weight_factors::<BalanceOf<T>, T::BlockNumber, _>(claimer_ledger, current_block)
    }

    fn claimee_weight_factors(
        target: &Self::Claimee,
        current_block: T::BlockNumber,
    ) -> WeightFactors {
        let claimee_ledger = ValidatorLedgers::<T>::get(target);
        generic_weight_factors::<BalanceOf<T>, T::BlockNumber, _>(claimee_ledger, current_block)
    }
}

impl<T: Trait> Module<T> {
    /// Returns the tuple of (dividend, source_weight, target_weight) if the nominator claims right now.
    pub fn calculate_dividend_on_claim(
        nominator: &T::AccountId,
        validator: &T::AccountId,
        block_number: T::BlockNumber,
    ) -> Result<(BalanceOf<T>, WeightType, WeightType, T::AccountId), Error<T>> {
        let validator_pot = T::DetermineRewardPotAccount::reward_pot_account_for(validator);
        let reward_pot_balance = Self::free_balance_of(&validator_pot);

        let (dividend, source_weight, target_weight) =
            <Self as ComputeMiningWeight<T::AccountId, T::BlockNumber>>::compute_dividend(
                nominator,
                validator,
                block_number,
                reward_pot_balance,
            )?;

        Ok((dividend, source_weight, target_weight, validator_pot))
    }

    /// Returns the dividend of `nominator` to `validator` at `block_number`.
    pub fn compute_dividend_at(
        nominator: &T::AccountId,
        validator: &T::AccountId,
        block_number: T::BlockNumber,
    ) -> Result<BalanceOf<T>, Error<T>> {
        Self::calculate_dividend_on_claim(nominator, validator, block_number)
            .map(|(dividend, _, _, _)| dividend)
    }

    fn allocate_dividend(
        claimer: &T::AccountId,
        pot_account: &T::AccountId,
        dividend: BalanceOf<T>,
    ) -> Result<(), AssetErr> {
        Self::move_balance(pot_account, claimer, dividend);
        Ok(())
    }

    /// Actually update the nominator vote weight given the new vote weight, block number and amount delta.
    pub(crate) fn set_nominator_vote_weight(
        nominator: &T::AccountId,
        validator: &T::AccountId,
        new_weight: WeightType,
        current_block: T::BlockNumber,
        delta: Delta<BalanceOf<T>>,
    ) {
        Nominations::<T>::mutate(nominator, validator, |claimer_ledger| {
            claimer_ledger.nomination = delta.calculate(claimer_ledger.nomination);
            claimer_ledger.last_vote_weight = new_weight;
            claimer_ledger.last_vote_weight_update = current_block;
        });
    }

    ///
    pub(crate) fn set_validator_vote_weight(
        validator: &T::AccountId,
        new_weight: WeightType,
        current_block: T::BlockNumber,
        delta: Delta<BalanceOf<T>>,
    ) {
        ValidatorLedgers::<T>::mutate(validator, |validator_ledger| {
            validator_ledger.total = delta.calculate(validator_ledger.total);
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

        let (dividend, source_weight, target_weight, claimee_pot) =
            Self::calculate_dividend_on_claim(claimer, claimee, current_block)?;

        Self::allocate_dividend(claimer, &claimee_pot, dividend)?;

        Self::deposit_event(RawEvent::Claim(claimer.clone(), claimee.clone(), dividend));

        let new_target_weight = target_weight - source_weight;

        Self::update_claimer_vote_weight_on_claim(claimer, claimee, current_block);
        Self::update_claimee_vote_weight_on_claim(claimee, new_target_weight, current_block);

        Ok(())
    }
}

impl<T: Trait> Module<T> {
    fn mint_and_slash(session_index: SessionIndex) {
        // TODO: the whole flow of session changes?
        //
        // Only the active validators can be rewarded.
        let staking_reward = Self::distribute_session_reward(session_index);

        let force_chilled = Self::slash_offenders_in_session(staking_reward);

        if !force_chilled.is_empty() {
            Self::deposit_event(RawEvent::ForceChilled(session_index, force_chilled));
            // Force a new era if some offender's reward pot has been wholly slashed.
            Self::ensure_new_era();
        }
    }
}

impl<T: Trait> Module<T> {
    fn new_session(session_index: SessionIndex) -> Option<Vec<T::AccountId>> {
        debug!(
            "[new_session]session_index:{:?}, current_era:{:?}",
            session_index,
            Self::current_era(),
        );

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

    /// Start a session potentially starting an era.
    fn start_session(start_session: SessionIndex) {
        // Skip the reward minting for the genesis initialization.
        // Actually start from session index 1.
        if start_session > 0 {
            Self::mint_and_slash(start_session);
        }

        let next_active_era = Self::active_era().map(|e| e.index + 1).unwrap_or(0);
        debug!(
            "[start_session]start_session:{:?}, next_active_era:{:?}",
            start_session, next_active_era
        );
        if let Some(next_active_era_start_session_index) =
            Self::eras_start_session_index(next_active_era)
        {
            if next_active_era_start_session_index == start_session {
                Self::start_era(start_session);
            } else if next_active_era_start_session_index < start_session {
                // This arm should never happen, but better handle it than to stall the
                // staking pallet.
                frame_support::print("Warning: A session appears to have been skipped.");
                Self::start_era(start_session);
            }
        }
    }

    /// End a session potentially ending an era.
    fn end_session(session_index: SessionIndex) {
        if let Some(active_era) = Self::active_era() {
            if let Some(next_active_era_start_session_index) =
                Self::eras_start_session_index(active_era.index + 1)
            {
                if next_active_era_start_session_index == session_index + 1 {
                    Self::end_era(active_era, session_index);
                }
            }
        }
    }

    /// * Increment `active_era.index`,
    /// * reset `active_era.start`,
    /// * update `BondedEras` and apply slashes.
    fn start_era(_start_session: SessionIndex) {
        let _active_era = ActiveEra::mutate(|active_era| {
            let new_index = active_era.as_ref().map(|info| info.index + 1).unwrap_or(0);
            *active_era = Some(ActiveEraInfo {
                index: new_index,
                // Set new active era start in next `on_finalize`. To guarantee usage of `Time`
                start: None,
            });
            new_index
        });
    }

    /// Compute payout for era.
    fn end_era(_active_era: ActiveEraInfo, _session_index: SessionIndex) {
        // Ignore, ChainX has nothing to do in end_era().
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

type OnOffenceRes = u64;
/// Validator ID that reported this offence.
type Reporter<T> = <T as frame_system::Trait>::AccountId;

/// Substrate:
/// A tuple of the validator's ID and their full identification.
/// pub type IdentificationTuple<T> = (<T as crate::Trait>::ValidatorId, <T as Trait>::FullIdentification);
/// ChainX:
/// We do not have the FullIdentification info, but the reward pot.
pub type IdentificationTuple<T> = (
    <T as frame_system::Trait>::AccountId,
    <T as frame_system::Trait>::AccountId,
);

/// Stable ID of a validator.
type Offender<T> = IdentificationTuple<T>;

/// This is intended to be used with `FilterHistoricalOffences` in Substrate/Staking.
/// In ChainX, we always apply the slash immediately, no deferred slash.
impl<T: Trait> OnOffenceHandler<Reporter<T>, IdentificationTuple<T>, OnOffenceRes> for Module<T>
where
    T: pallet_session::Trait<ValidatorId = <T as frame_system::Trait>::AccountId>,
    T::SessionHandler: pallet_session::SessionHandler<<T as frame_system::Trait>::AccountId>,
    T::SessionManager: pallet_session::SessionManager<<T as frame_system::Trait>::AccountId>,
    T::ValidatorIdOf: Convert<
        <T as frame_system::Trait>::AccountId,
        Option<<T as frame_system::Trait>::AccountId>,
    >,
{
    fn on_offence(
        offenders: &[OffenceDetails<Reporter<T>, Offender<T>>],
        slash_fraction: &[Perbill],
        _slash_session: SessionIndex,
    ) -> Result<OnOffenceRes, ()> {
        // TODO: make use of slash_fraction
        for (details, _slash_fraction) in offenders.iter().zip(slash_fraction) {
            // reporters are actually always empty.
            let (offender, _reporters) = &details.offender;

            // FIXME: record the offenders by session_index?
            <OffendersInSession<T>>::mutate(|offenders| {
                if !offenders.contains(offender) {
                    offenders.push(offender.clone())
                }
            });
        }
        Ok(0)
    }

    fn can_report() -> bool {
        true
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

        let validator_slice = validator_hash.as_ref();
        let registered_at_slice = registered_at_hash.as_ref();

        let mut buf = Vec::with_capacity(validator_slice.len() + registered_at_slice.len());
        buf.extend_from_slice(validator_slice);
        buf.extend_from_slice(registered_at_slice);

        UncheckedFrom::unchecked_from(T::Hashing::hash(&buf[..]))
    }
}
