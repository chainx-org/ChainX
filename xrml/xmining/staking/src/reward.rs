use super::*;
use xaccounts::IntentionJackpotAccountIdFor;

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
            / u64::from(u32::pow(2, (current_index / SESSIONS_PER_ROUND) as u32));
        T::Balance::sa(reward as u64)
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

    /// Collect the active intentions and psedu intentions.
    fn collect_reward_holders() -> Vec<(RewardHolder<T::AccountId>, T::Balance)> {
        let mut active_intentions = Self::intention_set()
            .into_iter()
            .filter(|i| Self::is_active(i))
            .map(|id| {
                let total_nomination = Self::total_nomination_of(&id);
                (RewardHolder::Intention(id), total_nomination)
            })
            .collect::<Vec<_>>();

        // Extend non-intention reward holders, i.e., Tokens currently, who are always considered as active.
        let psedu_intentions = T::OnRewardCalculation::psedu_intentions_info();
        active_intentions.extend(psedu_intentions);

        active_intentions
    }

    /// In the first round, 20% reward of each session goes to the team.
    fn try_fund_team(this_session_reward: T::Balance) -> T::Balance {
        let current_index = <xsession::Module<T>>::current_index().as_();

        if current_index < SESSIONS_PER_ROUND {
            let to_team = T::Balance::sa(this_session_reward.as_() / 5);
            debug!("[reward] issue to the team: {:?}", to_team);
            let _ =
                <xassets::Module<T>>::pcx_issue(&xaccounts::Module::<T>::team_account(), to_team);
            this_session_reward - to_team
        } else {
            this_session_reward
        }
    }

    /// Distribute the session reward for (psedu-)intentions.
    pub(super) fn distribute_session_reward(validators: &mut Vec<T::AccountId>) {
        // apply good session reward
        let this_session_reward = Self::this_session_reward();

        let mut session_reward = Self::try_fund_team(this_session_reward);

        let active_intentions = Self::collect_reward_holders();

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
                            // FIXME Don't pass validators in slash_active_offline_validator()
                            Self::slash_active_offline_validator(intention, reward, validators);
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
    }
}
