// Copyright 2018 Chainpool.
//! Coordidate session and era rotation.

use super::*;
use primitives::traits::{As, One, Zero};
use rstd::cmp;
use session::OnSessionChange;
use xaccounts::IntentionJackpotAccountIdFor;
use xassets;

/// RewardHolder includes intention as well as tokens.
#[derive(PartialEq, Eq, Clone, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub enum RewardHolder<AccountId: Default> {
    Intention(AccountId),
    PseduIntention(Token),
}

impl<AccountId: Default> Default for RewardHolder<AccountId> {
    fn default() -> Self {
        RewardHolder::Intention(Default::default())
    }
}

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
        let current_index = <session::Module<T>>::current_index().as_();
        let reward =
            Self::initial_reward().as_() / (u32::pow(2, (current_index / 210_000) as u32)) as u64;
        T::Balance::sa(reward as u64)
    }

    /// Gather all the active intentions sorted by total nomination.
    fn gather_candidates() -> Vec<(T::Balance, T::AccountId)> {
        let mut intentions = Self::intentions()
            .into_iter()
            .filter(|v| <xaccounts::Module<T>>::intention_props_of(v).is_active)
            .filter(|v| !Self::total_nomination_of(&v).is_zero())
            .map(|v| (Self::total_nomination_of(&v), v))
            .collect::<Vec<_>>();
        intentions.sort_unstable_by(|&(ref b1, _), &(ref b2, _)| b2.cmp(&b1));
        intentions
    }

    /// Reward a given (potential) validator by a specific amount.
    /// Add the reward to their balance, and their jackpot, pro-rata.
    fn reward(who: &T::AccountId, reward: T::Balance) {
        // In the first round, 20% reward goes to the team.
        let current_index = <session::Module<T>>::current_index().as_();
        let reward = if current_index <= 210_000 {
            let to_team = T::Balance::sa(reward.as_() * 2 / 10);
            let _ = <xassets::Module<T>>::pcx_issue(&Self::team_address(), to_team);
            reward - to_team
        } else {
            reward
        };

        let off_the_table = T::Balance::sa(reward.as_() * 1 / 10);
        let _ = <xassets::Module<T>>::pcx_issue(who, off_the_table);

        let to_jackpot = reward - off_the_table;
        // issue to jackpot
        let jackpot_addr = T::DetermineIntentionJackpotAccountId::accountid_for(who);
        let _ = <xassets::Module<T>>::pcx_issue(&jackpot_addr, to_jackpot);
    }

    /// Actually slash a given (potential) validator by a specific amount.
    /// If the jackpot of the validator can't afford the penalty, then he
    /// should be enforced to be inactive.
    fn slash_validator(who: &T::AccountId) -> bool {
        let jackpot_addr = T::DetermineIntentionJackpotAccountId::accountid_for(who);
        let council = Self::council_address();

        let total_slash = <TotalSlashOfPerSession<T>>::take(who);
        let jackpot_balance = <xassets::Module<T>>::pcx_free_balance(&jackpot_addr);

        let (slashed, should_be_enforced) = if total_slash <= jackpot_balance {
            (total_slash, false)
        } else {
            (jackpot_balance, true)
        };

        let _ = <xassets::Module<T>>::pcx_move_free_balance(&jackpot_addr, &council, slashed);
        Self::deposit_event(RawEvent::OfflineSlash(who.clone(), slashed));

        should_be_enforced
    }

    fn note_offline_slashed(who: &T::AccountId, slash: T::Balance) {
        if !<SlashedPerSession<T>>::get().into_iter().any(|x| x == *who) {
            <SlashedPerSession<T>>::mutate(|i| i.push(who.clone()));
        }
        let total_slash = Self::total_slash_of_per_session(who);
        <TotalSlashOfPerSession<T>>::insert(who, total_slash + slash);
    }

    /// Enforce these punished to be inactive, so that they won't become validators and be rewarded.
    fn enforce_inactive(is_new_era: bool) {
        let slashed = <SlashedPerSession<T>>::take();

        if slashed.is_empty() {
            return;
        }

        if Self::gather_candidates().len() <= Self::minimum_validator_count() as usize {
            for s in slashed {
                <TotalSlashOfPerSession<T>>::remove(s);
            }
            return;
        }

        let mut validators = <session::Module<T>>::validators()
            .into_iter()
            .map(|(v, _)| v)
            .collect::<Vec<_>>();

        for v in slashed.iter() {
            let should_be_enforced = Self::slash_validator(v);

            if should_be_enforced {
                // Force those slashed yet can't afford the penalty to be inactive
                <xaccounts::IntentionPropertiesOf<T>>::mutate(v, |props| {
                    props.is_active = false;
                    info!("validator enforced to be inactive: {:?}", v);
                });

                if validators.len() > Self::minimum_validator_count() as usize {
                    validators.retain(|x| *x != *v);
                }
            }
        }

        Self::deposit_event(RawEvent::EnforceValidatorsInactive(slashed.clone()));

        // The validator set will be updated on new era, so we don't have to update here.
        if is_new_era {
            return;
        }

        // Update to the latest total nomination
        let validators = validators
            .into_iter()
            .map(|v| (Self::intention_profiles(&v).total_nomination.as_(), v))
            .map(|(a, b)| (b, a))
            .collect::<Vec<_>>();
        info!("new validators due to enforce inactive: {:?}", validators);
        <session::Module<T>>::set_validators(validators.as_slice());
    }

    /// Session has just changed. We need to determine whether we pay a reward, slash and/or
    /// move to a new era.
    fn new_session(_actual_elapsed: T::Moment, should_reward: bool) {
        let session_index = <session::Module<T>>::current_index();
        let is_new_era = <ForcingNewEra<T>>::take().is_some()
            || ((session_index - Self::last_era_length_change()) % Self::sessions_per_era())
                .is_zero();

        Self::enforce_inactive(is_new_era);

        if should_reward {
            // apply good session reward
            let mut session_reward = Self::this_session_reward();

            let mut active_intentions: Vec<(RewardHolder<T::AccountId>, T::Balance)> =
                Self::intentions()
                    .into_iter()
                    .filter(|i| <xaccounts::Module<T>>::intention_props_of(i).is_active)
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

            for (holder, stake) in active_intentions.iter() {
                // May become zero after meeting the last one.
                if !total_active_stake.is_zero() {
                    // stake * session_reward could overflow.
                    let reward = match (stake.as_() as u128)
                        .checked_mul(session_reward.as_() as u128)
                    {
                        Some(x) => T::Balance::sa((x / total_active_stake.as_() as u128) as u64),
                        None => panic!("stake * session_reward overflow!"),
                    };
                    match holder {
                        RewardHolder::Intention(ref intention) => Self::reward(intention, reward),
                        RewardHolder::PseduIntention(ref token) => {
                            // Reward to token entity.
                            T::OnReward::reward(token, reward)
                        }
                    }
                    total_active_stake -= *stake;
                    session_reward -= reward;
                }
            }

            Self::deposit_event(RawEvent::Reward(total_active_stake, session_reward));

            // FIXME
            // T::OnRewardMinted::on_dilution(total_minted, total_minted);
        }

        if is_new_era {
            Self::new_era();
        }
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
                <LastEraLengthChange<T>>::put(&<session::Module<T>>::current_index());
            }
        }

        // evaluate desired staking amounts and nominations and optimise to find the best
        // combination of validators, then use session::internal::set_validators().
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
            <StakeWeight<T>>::insert(intention, total_nomination.clone());
        }

        let desired_validator_count = <ValidatorCount<T>>::get() as usize;

        let validators = candidates
            .clone()
            .into_iter()
            .take(desired_validator_count)
            .map(|(stake_weight, account_id)| (account_id, stake_weight.as_()))
            .collect::<Vec<(_, _)>>();

        info!("new validators: {:?}", validators);
        <session::Module<T>>::set_validators(&validators);
        Self::deposit_event(RawEvent::Rotation(validators));

        let session_index = <session::Module<T>>::current_index();
        if <xaccounts::Module<T>>::trustee_intentions().is_empty()
            || ((session_index - Self::last_era_length_change()) % (Self::sessions_per_epoch()))
                .is_zero()
        {
            Self::new_trustees(candidates.into_iter().map(|(_, v)| v).collect::<Vec<_>>());
        }
    }

    fn new_trustees(validator_candidates: Vec<T::AccountId>) {
        let candidates = validator_candidates
            .into_iter()
            .filter(|v| {
                <xaccounts::TrusteeIntentionPropertiesOf<T>>::get(&(v.clone(), Chain::Bitcoin))
                    .is_some()
            })
            .collect::<Vec<_>>();

        if (candidates.len() as u32) < Self::minimum_trustee_count() {
            return;
        }

        let mut trustees = candidates
            .into_iter()
            .take(Self::trustee_count() as usize)
            .collect::<Vec<_>>();

        trustees.sort();

        let last_trustees = <xaccounts::TrusteeIntentions<T>>::get();
        if last_trustees != trustees {
            info!("new trustees: {:?}", trustees);
            <xaccounts::TrusteeIntentions<T>>::put(trustees.clone());
        }

        let _ = xbitcoin::Module::<T>::update_trustee_addr();

        Self::deposit_event(RawEvent::NewTrustees(trustees));
    }

    fn jackpot_balance_of(who: &T::AccountId) -> T::Balance {
        let jackpot_addr = T::DetermineIntentionJackpotAccountId::accountid_for(who);
        <xassets::Module<T>>::pcx_free_balance(&jackpot_addr)
    }

    pub fn on_offline_validator(v: &T::AccountId) {
        let jackpot_balance = Self::jackpot_balance_of(v);
        let penalty = cmp::max(jackpot_balance.as_() / 100, Self::minimum_penalty().as_());

        Self::note_offline_slashed(v, T::Balance::sa(penalty));
    }
}

impl<T: Trait> OnSessionChange<T::Moment> for Module<T> {
    fn on_session_change(elapsed: T::Moment, should_reward: bool) {
        Self::new_session(elapsed, should_reward);
    }
}

impl<T: Trait> consensus::OnOfflineReport<Vec<u32>> for Module<T> {
    fn handle_report(reported_indices: Vec<u32>) {
        for validator_index in reported_indices {
            let v = <session::Module<T>>::validators()[validator_index as usize].clone();
            Self::on_offline_validator(&v.0);
        }
    }
}
