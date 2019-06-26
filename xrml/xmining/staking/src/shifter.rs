// Copyright 2018-2019 Chainpool.
//! Coordidate session and era rotation.

use super::*;

use primitives::traits::{As, One, Zero};
use rstd::cmp;
use xaccounts::IntentionJackpotAccountIdFor;
use xsession::OnSessionChange;
use xsupport::{debug, info, warn};
#[cfg(feature = "std")]
use xsupport::{validators, who};

pub trait OnRewardCalculation<AccountId: Default, Balance> {
    fn psedu_intentions_info() -> Vec<(RewardHolder<AccountId>, Balance)>;
}

impl<AccountId: Default, Balance> OnRewardCalculation<AccountId, Balance> for () {
    fn psedu_intentions_info() -> Vec<(RewardHolder<AccountId>, Balance)> {
        Vec::new()
    }
}

pub trait OnReward<AccountId: Default, Balance> {
    fn reward(_: &Token, _: Balance);
}

impl<AccountId: Default, Balance> OnReward<AccountId, Balance> for () {
    fn reward(_: &Token, _: Balance) {}
}

impl<T: Trait> Module<T> {
    /// Get the reward for the session, assuming it ends with this block.
    fn this_session_reward() -> T::Balance {
        let current_index = <xsession::Module<T>>::current_index().as_();
        let reward = Self::initial_reward().as_()
            / (u32::pow(2, (current_index / SESSIONS_PER_ROUND) as u32)) as u64;
        T::Balance::sa(reward as u64)
    }

    /// Gather all the active intentions sorted by total nomination.
    fn gather_candidates() -> Vec<(T::Balance, T::AccountId)> {
        let mut intentions = Self::intention_set()
            .into_iter()
            .filter(|v| Self::is_active(v) && !Self::total_nomination_of(&v).is_zero())
            .map(|v| (Self::total_nomination_of(&v), v))
            .collect::<Vec<_>>();
        intentions.sort_by(|&(ref b1, _), &(ref b2, _)| b2.cmp(&b1));
        intentions
    }

    /// Reward a given (potential) validator by a specific amount.
    /// Add the reward to their balance, and their jackpot, pro-rata.
    fn reward(who: &T::AccountId, reward: T::Balance) {
        // Validator only gains 10%, the rest 90% goes to the jackpot.
        let off_the_table = T::Balance::sa(reward.as_() / 10);
        let _ = <xassets::Module<T>>::pcx_issue(who, off_the_table);

        let to_jackpot = reward - off_the_table;
        // issue to jackpot
        let jackpot_addr = T::DetermineIntentionJackpotAccountId::accountid_for_unsafe(who);
        let _ = <xassets::Module<T>>::pcx_issue(&jackpot_addr, to_jackpot);
        debug!(
            "[reward] issue to {:?}'s jackpot: {:?}",
            who!(who),
            to_jackpot
        );
    }

    fn reward_of_per_block(session_reward: T::Balance) -> T::Balance {
        let session_length = <xsession::SessionLength<T>>::get().as_();
        let validators_count = <xsession::Validators<T>>::get().len() as u64;
        T::Balance::sa(session_reward.as_() * validators_count / session_length)
    }

    /// Actually slash a given active validator by a specific amount.
    /// If the jackpot of the validator can't afford the penalty and there are more than minimum validators,
    /// then he should be enforced to be inactive and removed from the validator set.
    fn slash_active_offline_validator(
        who: &T::AccountId,
        my_reward: T::Balance,
        validators: &mut Vec<T::AccountId>,
    ) {
        let council = xaccounts::Module::<T>::council_account();

        // Slash 10 times per block reward for each missed block.
        let missed = u64::from(<MissedOfPerSession<T>>::take(who));
        let reward_per_block = Self::reward_of_per_block(my_reward);
        let total_slash = cmp::max(
            T::Balance::sa(
                reward_per_block.as_() * missed * u64::from(Self::missed_blocks_severity()),
            ),
            T::Balance::sa(Self::minimum_penalty().as_() * missed),
        );

        let jackpot_addr = T::DetermineIntentionJackpotAccountId::accountid_for_unsafe(who);
        let jackpot_balance = <xassets::Module<T>>::pcx_free_balance(&jackpot_addr);

        let (slashed, should_be_enforced) = if total_slash <= jackpot_balance {
            (total_slash, false)
        } else {
            (jackpot_balance, true)
        };

        let _ = <xassets::Module<T>>::pcx_move_free_balance(&jackpot_addr, &council, slashed);

        debug!(
            "[slash_active_offline_validator] {:?} is actually slashed: {:?}, should be slashed: {:?}",
            who!(who),
            slashed,
            total_slash
        );

        // Force those slashed yet can't afford the penalty to be inactive when the validators is not too few.
        // Then these inactive validators will not be rewarded.
        if should_be_enforced && validators.len() > Self::minimum_validator_count() as usize {
            <xaccounts::IntentionPropertiesOf<T>>::mutate(who, |props| {
                props.is_active = false;
                props.last_inactive_since = <system::Module<T>>::block_number();
                info!(
                    "[slash_active_offline_validator] validator enforced to be inactive: {:?}",
                    who!(who)
                );
            });

            // remove from the current validator set
            validators.retain(|x| *x != *who);
        }
    }

    /// These offline validators choose to be inactive by themselves.
    /// Since they are already inactive at present, they won't share the reward,
    /// so we only need to slash them at the minimal penalty for the missed blocks when they were active.
    fn slash_inactive_offline_validators() {
        let slashed = <OfflineValidatorsPerSession<T>>::get();
        if slashed.is_empty() {
            return;
        }

        let mut missed_info = Vec::new();
        let mut inactive_slashed = Vec::new();

        for s in slashed {
            let missed_num = <MissedOfPerSession<T>>::get(&s);
            missed_info.push((s.clone(), missed_num));
            if !Self::is_active(&s) {
                inactive_slashed.push(s);
            }
        }

        Self::deposit_event(RawEvent::MissedBlocksOfOfflineValidatorPerSession(
            missed_info,
        ));

        for who in inactive_slashed.iter() {
            let missed = T::Balance::sa(u64::from(<MissedOfPerSession<T>>::take(who)));
            let should_slash = missed * Self::minimum_penalty();
            let council = xaccounts::Module::<T>::council_account();

            let jackpot_addr = T::DetermineIntentionJackpotAccountId::accountid_for_unsafe(who);
            let jackpot_balance = <xassets::Module<T>>::pcx_free_balance(&jackpot_addr);

            let slash = cmp::min(should_slash, jackpot_balance);

            let _ = <xassets::Module<T>>::pcx_move_free_balance(&jackpot_addr, &council, slash);
        }
    }

    /// Report the total missed blocks info to the session module.
    fn report_total_missed_blocks_count() {
        if <xsession::SessionTotalMissedBlocksCount<T>>::exists() {
            warn!("[report_total_missed_blocks] xsession::SessionTotalMissedBlocksCount should not exist on new session.");
        }
        let total_missed = Self::offline_validators_per_session()
            .iter()
            .map(|v| Self::missed_of_per_session(v))
            .fold(0u32, |acc, x| acc + x);
        <xsession::SessionTotalMissedBlocksCount<T>>::put(total_missed);
    }

    /// Session has just changed. We need to determine whether we pay a reward, slash and/or
    /// move to a new era.
    fn new_session() {
        Self::report_total_missed_blocks_count();

        // No reward but only slash for these offline validators that are inactive atm.
        Self::slash_inactive_offline_validators();

        let mut validators = <xsession::Module<T>>::validators()
            .into_iter()
            .map(|(v, _)| v)
            .collect::<Vec<_>>();

        let current_validator_count = validators.len();

        // Try removing the evil validators first.
        let evil_validators = <EvilValidatorsPerSession<T>>::take();
        for evil_val in evil_validators.iter() {
            if validators.len() > Self::minimum_validator_count() as usize {
                validators.retain(|x| *x != *evil_val);
            }
        }

        // apply good session reward
        let this_session_reward = Self::this_session_reward();

        // In the first round, 20% reward goes to the team.
        let current_index = <xsession::Module<T>>::current_index().as_();
        let mut session_reward = if current_index < SESSIONS_PER_ROUND {
            let to_team = T::Balance::sa(this_session_reward.as_() / 5);
            debug!("[reward] issue to the team: {:?}", to_team);
            let _ =
                <xassets::Module<T>>::pcx_issue(&xaccounts::Module::<T>::team_account(), to_team);
            this_session_reward - to_team
        } else {
            this_session_reward
        };

        let mut active_intentions = Self::intention_set()
            .into_iter()
            .filter(|i| Self::is_active(i))
            .map(|id| {
                let total_nomination = Self::total_nomination_of(&id);
                (RewardHolder::Intention(id), total_nomination)
            })
            .collect::<Vec<_>>();

        // Extend non-intention reward holders, i.e., Tokens currently.
        let psedu_intentions = T::OnRewardCalculation::psedu_intentions_info();
        active_intentions.extend(psedu_intentions);

        let mut total_active_stake = active_intentions
            .iter()
            .fold(Zero::zero(), |acc: T::Balance, (_, x)| acc + *x);

        Self::deposit_event(RawEvent::Reward(total_active_stake, this_session_reward));

        for (holder, stake) in active_intentions.iter() {
            // May become zero after meeting the last one.
            if !total_active_stake.is_zero() {
                // stake * session_reward could overflow.
                let reward = match (u128::from(stake.as_()))
                    .checked_mul(u128::from(session_reward.as_()))
                {
                    Some(x) => {
                        let r = x / u128::from(total_active_stake.as_());
                        if r < u128::from(u64::max_value()) {
                            T::Balance::sa(r as u64)
                        } else {
                            panic!("reward of per intention definitely less than u64::max_value()")
                        }
                    }
                    None => panic!("stake * session_reward overflow!"),
                };
                match holder {
                    RewardHolder::Intention(ref intention) => {
                        Self::reward(intention, reward);

                        // It the intention was an offline validator, we should enforce a slash.
                        if <MissedOfPerSession<T>>::exists(intention) {
                            Self::slash_active_offline_validator(
                                intention,
                                reward,
                                &mut validators,
                            );
                        }
                    }
                    RewardHolder::PseduIntention(ref token) => {
                        // Reward to token entity.
                        T::OnReward::reward(token, reward)
                    }
                }
                total_active_stake -= *stake;
                session_reward -= reward;
            }
        }

        // Reset slashed validator set
        <OfflineValidatorsPerSession<T>>::kill();

        let session_index = <xsession::Module<T>>::current_index();
        let is_new_era =
            ((session_index - Self::last_era_length_change()) % Self::sessions_per_era()).is_zero();

        if is_new_era {
            Self::new_era();
        } else if validators.len() < current_validator_count {
            Self::set_validators_on_non_era(validators);
        }
    }

    /// We only reduce the offline validators on non-era session.
    /// This happens when there are offline validators that are enforced to be inactive.
    pub fn set_validators_on_non_era(validators: Vec<T::AccountId>) {
        // Update to the latest total nomination
        let validators = validators
            .into_iter()
            .map(|v| (Self::intentions(&v).total_nomination.as_(), v))
            .map(|(a, b)| (b, a))
            .collect::<Vec<_>>();
        <xsession::Module<T>>::set_validators(validators.as_slice());
        info!(
            "[set_validators_on_non_era] new validator set due to enforcing inactive: {:?}",
            validators!(validators)
        );
    }

    /// The era has changed - enact new staking set.
    ///
    /// NOTE: This always happens immediately before a session change to ensure that new validators
    /// get a chance to set their session keys.
    fn new_era() {
        // Increment current era.
        <CurrentEra<T>>::put(&(<CurrentEra<T>>::get() + One::one()));

        // Enact era length change.
        if let Some(next_spe) = Self::next_sessions_per_era() {
            if next_spe != Self::sessions_per_era() {
                <SessionsPerEra<T>>::put(&next_spe);
                <LastEraLengthChange<T>>::put(&<xsession::Module<T>>::current_index());
            }
        }

        // evaluate desired staking amounts and nominations and optimise to find the best
        // combination of validators, then use xsession::internal::set_validators().
        // for now, this just orders would-be stakers by their balances and chooses the top-most
        // <ValidatorCount<T>>::get() of them.
        // TODO: this is not sound. this should be moved to an off-chain solution mechanism.
        let candidates = Self::gather_candidates();

        // Avoid reevaluate validator set if it would leave us with fewer than the minimum
        // needed validators
        if candidates.len() < Self::minimum_validator_count() as usize {
            return;
        }

        for (total_nomination, intention) in candidates.iter() {
            <StakeWeight<T>>::insert(intention, *total_nomination);
        }

        let desired_validator_count = <ValidatorCount<T>>::get() as usize;

        let validators = candidates
            .clone()
            .into_iter()
            .take(desired_validator_count)
            .map(|(stake_weight, account_id)| (account_id, stake_weight.as_()))
            .collect::<Vec<(_, _)>>();

        info!("[new_era] new validator set: {:?}", validators!(validators));
        <xsession::Module<T>>::set_validators(&validators);
        Self::deposit_event(RawEvent::Rotation(validators));
    }

    /// We only note these offline validators that are still active at the moment.
    pub fn on_offline_validator(v: &T::AccountId) {
        if !Self::is_active(v) {
            return;
        }

        debug!(
            "[note_offline_validator]: active offline validator noted: {:?}",
            who!(v)
        );

        <OfflineValidatorsPerSession<T>>::mutate(|offline| {
            if !offline.contains(v) {
                offline.push(v.clone())
            }
        });

        let missed = Self::missed_of_per_session(v);
        <MissedOfPerSession<T>>::insert(v, missed + 1);
    }
}

impl<T: Trait> OnSessionChange<T::Moment> for Module<T> {
    fn on_session_change() {
        Self::new_session();
    }
}

impl<T: Trait> consensus::OnOfflineReport<Vec<u32>> for Module<T> {
    fn handle_report(reported_indices: Vec<u32>) {
        for validator_index in reported_indices {
            let v = <xsession::Module<T>>::validators()[validator_index as usize].clone();
            Self::on_offline_validator(&v.0);
        }
    }
}
