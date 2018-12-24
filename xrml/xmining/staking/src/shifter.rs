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
    AccountId(AccountId),
}

impl<AccountId: Default> Default for RewardHolder<AccountId> {
    fn default() -> Self {
        RewardHolder::AccountId(Default::default())
    }
}

impl<T: Trait> Module<T> {
    /// Get the reward for the session, assuming it ends with this block.
    fn this_session_reward() -> T::Balance {
        let total_stake = Self::total_stake().as_();
        let reward = match total_stake {
            0...100_000_000_000 => total_stake * 1 / 1000,
            100_000_000_001...200_000_000_000 => total_stake * 9 / 10000,
            200_000_000_001...300_000_000_000 => total_stake * 8 / 10000,
            300_000_000_001...400_000_000_000 => total_stake * 7 / 10000,
            400_000_000_001...500_000_000_000 => total_stake * 6 / 10000,
            500_000_000_001...600_000_000_000 => total_stake * 5 / 10000,
            600_000_000_001...700_000_000_000 => total_stake * 4 / 10000,
            700_000_000_001...800_000_000_000 => total_stake * 3 / 10000,
            800_000_000_001...900_000_000_000 => total_stake * 2 / 10000,
            _ => total_stake * 1 / 10000,
        };
        T::Balance::sa(reward / 1000)
    }

    /// Reward a given (potential) validator by a specific amount.
    /// Add the reward to their balance, and their jackpot, pro-rata.
    fn reward(who: &T::AccountId, reward: T::Balance) {
        let off_the_table = T::Balance::sa(reward.as_() * 1 / 10);
        let _ = <xassets::Module<T>>::pcx_reward(who, off_the_table);
        let to_jackpot = reward - off_the_table;
        let mut iprof = <IntentionProfiles<T>>::get(who);
        iprof.jackpot += to_jackpot;
        <IntentionProfiles<T>>::insert(who, iprof);
    }

    /// Session has just changed. We need to determine whether we pay a reward, slash and/or
    /// move to a new era.
    fn new_session(_actual_elapsed: T::Moment, should_reward: bool) {
        if should_reward {
            // apply good session reward
            let reward = Self::this_session_reward();

            let mut total_minted: T::Balance = Zero::zero();

            let mut active_intentions: Vec<(RewardHolder<T::AccountId>, T::Balance)> =
                Self::intentions()
                    .into_iter()
                    .filter(|i| <xaccounts::Module<T>>::intention_props_of(i).is_active)
                    .map(|id| {
                        let total_nomination = Self::total_nomination_of(&id);
                        (RewardHolder::AccountId(id), total_nomination)
                    })
                    .collect::<Vec<_>>();

            // TODO Add non account reward holders
            // let token_list = T::OnNewSessionForTokenStaking::token_staking_info();
            let tokens = Vec::new();
            active_intentions.extend(tokens);

            let total_active_stake = active_intentions
                .iter()
                .fold(Zero::zero(), |acc: T::Balance, (_, x)| acc + *x);

            if !total_active_stake.is_zero() {
                for (holder, stake) in active_intentions.iter() {
                    let reward = *stake * reward / total_active_stake;
                    total_minted += reward;
                    match holder {
                        RewardHolder::AccountId(ref intention) => Self::reward(intention, reward), // TODO Reward to token entity.
                    }
                }
            }

            // Self::deposit_event(RawEvent::Reward(reward));
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
