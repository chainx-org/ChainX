use super::*;
use rstd::convert::TryInto;
use rstd::result;
use xaccounts::IntentionJackpotAccountIdFor;

// Who will be rewarded on new session.
type Payees<T> = Vec<(
    RewardHolder<<T as system::Trait>::AccountId>,
    <T as xassets::Trait>::Balance,
)>;

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

    /// Collect all the active intentions.
    fn get_staked_info() -> (Payees<T>, T::Balance) {
        let active_intentions = Self::intention_set()
            .into_iter()
            .filter(|i| Self::is_active(i))
            .map(|id| {
                let total_nomination = Self::total_nomination_of(&id);
                (RewardHolder::Intention(id), total_nomination)
            })
            .collect::<Vec<_>>();

        let total_staked = active_intentions
            .iter()
            .fold(Zero::zero(), |acc: T::Balance, (_, x)| acc + *x);

        (active_intentions, total_staked)
    }

    /// Collect all the psedu intentions.
    fn get_psedu_intentions_info() -> (Payees<T>, T::Balance) {
        let psedu_intentions = T::OnRewardCalculation::psedu_intentions_info();

        // could overflow?
        let total_cross_chain_assets = psedu_intentions
            .iter()
            .fold(Zero::zero(), |acc: T::Balance, (_, x)| acc + *x);

        (psedu_intentions, total_cross_chain_assets)
    }

    /// Whether the cross chain assets are growing too fast than the staked native assets.
    ///
    /// if total_cross_chain_assets > distribution_ratio * total_staked, return the double discount for cross-chain assets.
    pub(super) fn are_growing_too_fast(
        total_cross_chain_assets: u64,
        total_staked: u64,
    ) -> result::Result<(u128, u128), ()> {
        let (numerator, denominator) = Self::distribution_ratio();

        debug!("[are_growing_too_fast] distribution_ratio: {:?}, total_cross_chain_assets: {:?}, total_staked: {:?}", (numerator, denominator), total_cross_chain_assets, total_staked);

        if u128::from(total_cross_chain_assets) * u128::from(denominator)
            > u128::from(numerator) * u128::from(total_staked)
        {
            let num = u128::from(numerator) * u128::from(total_staked);
            let denom = u128::from(denominator) * u128::from(total_cross_chain_assets);
            return Ok((num, denom));
        }

        Err(())
    }

    /// Calculate the individual reward according to the proportion and total reward.
    fn calculate_reward_by_proportion(
        total_reward: T::Balance,
        proportion: (T::Balance, T::Balance),
    ) -> T::Balance {
        let (mine, total) = proportion;

        match (u128::from(mine.as_())).checked_mul(u128::from(total_reward.as_())) {
            Some(x) => {
                let r = x / u128::from(total.as_());
                assert!(
                    r < u128::from(u64::max_value()),
                    "reward of per intention definitely less than u64::max_value()"
                );
                T::Balance::sa(r as u64)
            }
            None => panic!("stake * session_reward overflow!"),
        }
    }

    /// Actually reward the (psedu-)intentions pro rata.
    fn reward_accordingly(
        payees: Payees<T>,
        session_reward: T::Balance,
        total: T::Balance,
        validators: &mut Vec<T::AccountId>,
    ) -> (T::Balance, T::Balance) {
        let mut staked_received = T::Balance::default();
        let mut tokens_received = T::Balance::default();
        let mut session_reward = session_reward;
        let mut total = total;
        for (payee, stake) in payees.iter() {
            // May become zero after meeting the last one.
            if !total.is_zero() {
                let reward = Self::calculate_reward_by_proportion(session_reward, (*stake, total));
                match payee {
                    RewardHolder::Intention(ref intention) => {
                        Self::reward(intention, reward);
                        staked_received += reward;

                        // It the intention was an offline validator, we should enforce a slash.
                        if <MissedOfPerSession<T>>::exists(intention) {
                            // FIXME Don't pass validators in slash_active_offline_validator()
                            Self::slash_active_offline_validator(intention, reward, validators);
                        }
                    }
                    RewardHolder::PseduIntention(ref token) => {
                        // Reward to token entity.
                        T::OnReward::reward(token, reward);
                        tokens_received += reward;
                    }
                }
                total -= *stake;
                session_reward -= reward;
            }
        }
        (staked_received, tokens_received)
    }

    // This is guarantee not to overflow on whatever values.
    // `num` must be inferior to `den` otherwise it will be reduce to `den`.
    // Credit: substrate
    pub fn multiply_by_rational(value: u64, num: u32, den: u32) -> u64 {
        let num = num.min(den);

        let result_divisor_part: u64 = value / u64::from(den) * u64::from(num);

        let result_remainder_part: u64 = {
            let rem: u64 = value % u64::from(den);

            // Fits into u32 because den is u32 and remainder < den
            let rem_u32 = rem.try_into().unwrap_or(u32::max_value());

            // Multiplication fits into u64 as both term are u32
            let rem_part = u64::from(rem_u32) * u64::from(num) / u64::from(den);

            // Result fits into u32 as num < total_points
            (rem_part as u32).into()
        };

        result_divisor_part + result_remainder_part
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
        let this_session_reward = Self::this_session_reward();

        let session_reward = Self::try_fund_team(this_session_reward);

        let (intentions, total_staked) = Self::get_staked_info();
        let (psedu_intentions, total_cross_chain_assets) = Self::get_psedu_intentions_info();

        let (for_staked, for_cross_chain_assets) = if Self::are_growing_too_fast(
            total_cross_chain_assets.as_(),
            total_staked.as_(),
        )
        .is_ok()
        {
            let (numerator, denominator) = Self::distribution_ratio();

            let for_cross_chain_assets = T::Balance::sa(Self::multiply_by_rational(
                session_reward.as_(),
                numerator,
                numerator + denominator,
            ));

            let for_staked = session_reward - for_cross_chain_assets;

            debug!(
                "[distribute_session_reward] cross-chain assets are growing too fast: cross-chain asssets: {:?}, total_staked: {:?}, ratio: {:?}",
                total_cross_chain_assets,
                total_staked,
                (numerator, denominator)
            );

            Self::reward_accordingly(intentions, for_staked, total_staked, validators);
            Self::reward_accordingly(
                psedu_intentions,
                for_cross_chain_assets,
                total_cross_chain_assets,
                validators,
            );

            (for_staked, for_cross_chain_assets)
        } else {
            // Old way.
            let mut payees = intentions;
            payees.extend(psedu_intentions);
            let total = total_staked + total_cross_chain_assets;
            Self::reward_accordingly(payees, session_reward, total, validators)
        };

        Self::deposit_event(RawEvent::SessionReward(
            total_staked,
            for_staked,
            total_cross_chain_assets,
            for_cross_chain_assets,
        ));
    }
}
