// Copyright 2018 Chainpool.
//! Coordidate session and era rotation.

use super::*;
use runtime_primitives::traits::{As, One, Zero};
use session::OnSessionChange;
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
    fn total_stake() -> T::Balance {
        Self::intentions()
            .into_iter()
            .map(|i| Self::intention_profiles(i).total_nomination)
            .fold(Zero::zero(), |acc: T::Balance, x| acc + x)
    }

    /// Get the reward for the session, assuming it ends with this block.
    fn this_session_reward() -> T::Balance {
        let total_stake = Self::total_stake().as_();
        // daily_interest: 3 / 10000
        let daily_reward = total_stake * 3 / 10000;
        let blocks_per_session = <session::Module<T>>::length().as_();
        let sessions_per_day = Self::blocks_per_day() / blocks_per_session;
        T::Balance::sa(daily_reward / sessions_per_day)
    }

    /// Reward a given (potential) validator by a specific amount.
    /// Add the reward to their balance, and their jackpot, pro-rata.
    fn reward(who: &T::AccountId, reward: T::Balance) {
        let off_the_table = T::Balance::sa(reward.as_() * 1 / 10);
        let _ = <xassets::Module<T>>::pcx_issue(who, off_the_table);
        let to_jackpot = reward - off_the_table;
        <IntentionProfiles<T>>::mutate(who, |iprof| iprof.jackpot += to_jackpot);
    }

    /// Session has just changed. We need to determine whether we pay a reward, slash and/or
    /// move to a new era.
    fn new_session(_actual_elapsed: T::Moment, should_reward: bool) {
        if should_reward {
            // apply good session reward
            let mut session_reward = Self::this_session_reward();
            Self::deposit_event(RawEvent::Reward(session_reward));

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

            if !total_active_stake.is_zero() {
                for (holder, stake) in active_intentions.iter() {
                    let reward = *stake * session_reward / total_active_stake;
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

            // FIXME
            // T::OnRewardMinted::on_dilution(total_minted, total_minted);
        }

        let session_index = <session::Module<T>>::current_index();
        if <ForcingNewEra<T>>::take().is_some()
            || ((session_index - Self::last_era_length_change()) % Self::sessions_per_era())
                .is_zero()
        {
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
        let mut intentions = Self::intentions()
            .into_iter()
            .filter(|i| <xaccounts::Module<T>>::intention_props_of(i).is_active)
            .map(|v| (Self::total_nomination_of(&v), v))
            .collect::<Vec<_>>();

        // Avoid reevaluate validator set if it would leave us with fewer than the minimum
        // needed validators
        if intentions.len() < Self::minimum_validator_count() as usize {
            return;
        }

        intentions.sort_unstable_by(|&(ref b1, _), &(ref b2, _)| b2.cmp(&b1));

        for (total_nomination, intention) in intentions.iter() {
            <StakeWeight<T>>::insert(intention, total_nomination.clone());
        }

        let desired_validator_count = <ValidatorCount<T>>::get() as usize;

        let vals = &intentions
            .clone()
            .into_iter()
            .map(|(_, v)| v)
            .take(desired_validator_count)
            .collect::<Vec<_>>();

        <session::Module<T>>::set_validators(vals);

        // Update the balances for slashing/rewarding according to the stakes.
        // <CurrentOfflineSlash<T>>::put(Self::offline_slash().times(average_stake));
        // <CurrentSessionReward<T>>::put(Self::session_reward().times(average_stake));

        // Disable slash mechanism
        <CurrentSessionReward<T>>::put(Self::this_session_reward());
    }
}

impl<T: Trait> OnSessionChange<T::Moment> for Module<T> {
    fn on_session_change(elapsed: T::Moment, should_reward: bool) {
        Self::new_session(elapsed, should_reward);
    }
}
