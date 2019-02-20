// Copyright 2018 Chainpool.
//! Coordidate session and era rotation.

use super::*;
use runtime_primitives::traits::{As, One, Zero};
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

    /// Slash a given (potential) validator by a specific amount.
    fn slash_validator(who: &T::AccountId, slash: T::Balance) {
        let jackpot_addr = T::DetermineIntentionJackpotAccountId::accountid_for(who);
        let fund_id = Self::funding();

        if slash <= <xassets::Module<T>>::pcx_free_balance(&jackpot_addr) {
            let _ = <xassets::Module<T>>::pcx_move_free_balance(&jackpot_addr, &fund_id, slash);
        } else {
            // Put the slashed validator into the punished list when its jackpot is not enough to be slashed.
            // These on the punish list will be enforced inactive on new session when possible.
            if !<PunishList<T>>::get().into_iter().any(|x| x == *who) {
                <PunishList<T>>::mutate(|i| i.push(who.clone()));
            }
        }
    }

    /// Enforce these punished to be inactive, so that they won't become validators and be rewarded.
    fn enforce_inactive() {
        if Self::gather_candidates().len() <= Self::minimum_validator_count() as usize {
            return;
        }

        let punished = <PunishList<T>>::take();
        for v in punished.iter() {
            // Force those punished to be inactive
            <xaccounts::IntentionPropertiesOf<T>>::mutate(v, |props| {
                props.is_active = false;
            });
        }

        Self::deposit_event(RawEvent::EnforceValidatorsInactive(punished));
    }

    /// Session has just changed. We need to determine whether we pay a reward, slash and/or
    /// move to a new era.
    fn new_session(_actual_elapsed: T::Moment, should_reward: bool) {
        Self::enforce_inactive();

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

        let session_index = <session::Module<T>>::current_index();
        if <ForcingNewEra<T>>::take().is_some()
            || ((session_index - Self::last_era_length_change()) % Self::sessions_per_era())
                .is_zero()
        {
            Self::new_era();
        }

        if <xaccounts::Module<T>>::trustee_intentions().is_empty()
            || ((session_index - Self::last_era_length_change()) % (Self::sessions_per_epoch()))
                .is_zero()
        {
            Self::new_trustees();
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

        let old_validators = <session::Module<T>>::validators();

        let new_validators = candidates
            .into_iter()
            .take(desired_validator_count)
            .map(|(stake_weight, account_id)| (account_id, stake_weight.as_()))
            .collect::<Vec<(_, _)>>();

        // Skip set validators when the validator set stays the same.
        if old_validators.len() == new_validators.len()
            && old_validators
                .iter()
                .map(|(val, _)| val)
                .zip(new_validators.iter().map(|(val, _)| val))
                .all(|(a, b)| a == b)
        {
            return;
        }

        <session::Module<T>>::set_validators(&new_validators);
        Self::deposit_event(RawEvent::Rotation(new_validators.clone()));
    }

    pub fn on_offline_validator(v: &T::AccountId) {
        let slash = Self::penalty();
        Self::slash_validator(v, slash);
        Self::deposit_event(RawEvent::OfflineSlash(v.clone(), slash));
    }

    fn new_trustees() {
        let intentions = Self::gather_candidates();
        if intentions.len() as u32 >= Self::minimum_trustee_count() {
            let trustees = intentions
                .into_iter()
                .take(Self::trustee_count() as usize)
                .map(|(_, v)| v)
                .filter(|v| {
                    <xaccounts::TrusteeIntentionPropertiesOf<T>>::get(&(v.clone(), Chain::Bitcoin))
                        .is_some()
                })
                .collect::<Vec<_>>();

            <xaccounts::TrusteeIntentions<T>>::put(trustees.clone());

            let _ = xbitcoin::Module::<T>::update_trustee_addr();

            Self::deposit_event(RawEvent::NewTrustees(trustees));
        }
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
