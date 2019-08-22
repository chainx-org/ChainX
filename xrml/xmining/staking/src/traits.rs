// Copyright 2018-2019 Chainpool.

use super::*;

/// Collect the staking info of virtual intentions from other modules, e.g., tokens.
pub trait OnRewardCalculation<AccountId: Default, Balance> {
    fn psedu_intentions_info() -> Vec<(RewardHolder<AccountId>, Balance)>;
}

impl<AccountId: Default, Balance> OnRewardCalculation<AccountId, Balance> for () {
    fn psedu_intentions_info() -> Vec<(RewardHolder<AccountId>, Balance)> {
        Vec::new()
    }
}

/// Distribute the reward accrodingly to the virtual intention.
pub trait OnReward<AccountId: Default, Balance> {
    fn reward(_: &Token, _: Balance);
}

impl<AccountId: Default, Balance> OnReward<AccountId, Balance> for () {
    fn reward(_: &Token, _: Balance) {}
}

/// These are three factors to calculate the latest vote weight.
///
/// Lastest vote weight = last_acum_weight(WeightType) + amount(u64) * duration(u64)
pub type WeightFactors = (WeightType, u64, u64);

/// Prepare the neccessary vote weight factors to compute the latest vote weight.
pub trait ComputeWeight<AccountId> {
    type Claimee;

    fn prepare_claimer_weight_factors(_: &AccountId, _: &Self::Claimee, _: u64) -> WeightFactors;
    fn prepare_claimee_weight_factors(_: &Self::Claimee, _: u64) -> WeightFactors;

    fn settle_claimer_weight(
        who: &AccountId,
        target: &Self::Claimee,
        current_block: u64,
    ) -> (u128, bool) {
        Self::settle_latest_vote_weight(Self::prepare_claimer_weight_factors(
            who,
            target,
            current_block,
        ))
    }

    fn settle_claimee_weight(target: &Self::Claimee, current_block: u64) -> (u128, bool) {
        Self::settle_latest_vote_weight(Self::prepare_claimee_weight_factors(target, current_block))
    }

    /// This is used when settling the vote weight in staking.
    fn settle_weight(
        who: &AccountId,
        target: &Self::Claimee,
        current_block: u64,
    ) -> ((u128, bool), (u128, bool)) {
        (
            Self::settle_claimer_weight(who, target, current_block),
            Self::settle_claimee_weight(target, current_block),
        )
    }

    /// This is used when the claimer claims the dividend.
    ///
    /// The difference compared to settle_weight() is to return an error directly
    /// if the claimer's vote weight is zero.
    fn settle_weight_on_claim(
        who: &AccountId,
        target: &Self::Claimee,
        current_block: u64,
    ) -> rstd::result::Result<((u128, bool), (u128, bool)), &'static str> {
        let claimer_weight_info = Self::settle_claimer_weight(who, target, current_block);

        if claimer_weight_info.0 == 0 {
            return Err("the vote weight of claimer is zero.");
        }

        let claimee_weight_info = Self::settle_claimee_weight(target, current_block);

        Ok((claimer_weight_info, claimee_weight_info))
    }

    /// Settle the latest accumlative vote weight.
    ///
    /// Return a tuple containing the final safe u128 result
    /// and a bool indicating whether the result overflowed for u64 in the calculation process.
    fn settle_latest_vote_weight(weight_factors: WeightFactors) -> (u128, bool) {
        let (last_acum_weight, amount, duration) = weight_factors;

        let cur_acum_weight = u128::from(amount) * u128::from(duration);

        // Settle the latest accumlative vote weight.
        // Err(..) only occurs when the overflow happens for u64.
        let latest_acum_weight = match last_acum_weight {
            WeightType::U128(last_acum_weight) => {
                Ok(WeightType::U128(last_acum_weight + cur_acum_weight))
            }
            WeightType::U64(last_acum_weight) => {
                let safe_last_acum_weight = u128::from(last_acum_weight);
                let latest_acum_weight = safe_last_acum_weight + cur_acum_weight;
                if latest_acum_weight <= u128::from(u64::max_value()) {
                    Ok(WeightType::U64(latest_acum_weight as u64))
                } else {
                    Err(WeightType::U128(latest_acum_weight))
                }
            }
        };

        match latest_acum_weight {
            Err(WeightType::U128(x)) => (x, true),
            Ok(x) | Err(x) => (x.into_inner_safe(), false),
        }
    }
}

pub trait Claim<AccountId, Balance> {
    type Claimee;

    /// Allocate the calculated dividend to the receivers.
    ///
    /// For the cross miners, 10% of the initial calculated dividend will be distributed
    /// to the corresponding referral, and then the left 90% will go to the claimer itself.
    fn allocate_dividend(
        claimer: &AccountId,
        claimee: &Self::Claimee,
        claimee_jackpot: &AccountId,
        dividend: Balance,
    ) -> Result;

    /// Main logic for claiming the dividend.
    fn claim(claimer: &AccountId, claimee: &Self::Claimee) -> Result;
}

/// Declare VoteWeightBase and VoteWeight trait, the V1 version only changes to u64 of acum_weight to u128.
macro_rules! decl_vote_weight_trait {
    ( $($weight_trait:ident: $weight_base_trait:ident => $weight_type:ty;)+ ) => {
        $(
            /// Define the get and set methods for the vote weight operations.
            pub trait $weight_base_trait<BlockNumber: As<u64>> {
                fn amount(&self) -> u64;
                fn set_amount(&mut self, new: u64);

                fn last_acum_weight(&self) -> $weight_type;
                fn set_last_acum_weight(&mut self, s: $weight_type);

                fn last_acum_weight_update(&self) -> u64;
                fn set_last_acum_weight_update(&mut self, num: BlockNumber);
            }

            /// General logic for stage changes of the vote weight operations.
            pub trait $weight_trait<BlockNumber: As<u64>>: $weight_base_trait<BlockNumber> {
                /// Set the new amount after settling the change of nomination.
                fn settle_and_set_amount(&mut self, delta: &Delta) {
                    let new = match *delta {
                        Delta::Add(x) => self.amount() + x,
                        Delta::Sub(x) => self.amount() - x,
                        Delta::Zero => return,
                    };
                    self.set_amount(new);
                }

                /// This action doesn't involve in a change of amount, used for tokens module only.
                fn set_state_weight(&mut self, latest_acum_weight: $weight_type, current_block: BlockNumber) {
                    self.set_last_acum_weight(latest_acum_weight);
                    self.set_last_acum_weight_update(current_block);
                }

                /// Set new state on nominate, unnominate and renominate.
                ///
                /// This is similar to set_state_on_claim with the settlement of amount added.
                fn set_state(&mut self, latest_acum_weight: $weight_type, current_block: BlockNumber, delta: &Delta) {
                    self.set_last_acum_weight(latest_acum_weight);
                    self.set_last_acum_weight_update(current_block);
                    self.settle_and_set_amount(delta);
                }
            }
        )+
    }
}

decl_vote_weight_trait! {
    VoteWeight: VoteWeightBase => u64;
    VoteWeightV1: VoteWeightBaseV1 => u128;
}
