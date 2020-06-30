use crate::VoteWeight;
use sp_std::result::Result;

/// The getter and setter methods for the further vote weight processing.
pub trait BaseVoteWeight<BlockNumber> {
    fn amount(&self) -> u64;
    fn set_amount(&mut self, new: u64);

    fn last_acum_weight(&self) -> VoteWeight;
    fn set_last_acum_weight(&mut self, s: VoteWeight);

    fn last_acum_weight_update(&self) -> u64;
    fn set_last_acum_weight_update(&mut self, num: BlockNumber);
}

#[derive(Clone, Copy)]
pub enum Delta {
    Add(u64),
    Sub(u64),
    Zero,
}

/// General logic for stage changes of the vote weight operations.
pub trait VoteWightTrait<BlockNumber>: BaseVoteWeight<BlockNumber> {
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
    fn set_state_weight(&mut self, latest_acum_weight: VoteWeight, current_block: BlockNumber) {
        self.set_last_acum_weight(latest_acum_weight);
        self.set_last_acum_weight_update(current_block);
    }

    /// Set new state on nominate, unnominate and renominate.
    ///
    /// This is similar to set_state_on_claim with the settlement of amount added.
    fn set_state(
        &mut self,
        latest_acum_weight: VoteWeight,
        current_block: BlockNumber,
        delta: &Delta,
    ) {
        self.set_state_weight(latest_acum_weight, current_block);
        self.settle_and_set_amount(delta);
    }
}

impl<BlockNumber, T: BaseVoteWeight<BlockNumber>> VoteWightTrait<BlockNumber> for T {}

/// Formula: Latest Vote Weight = last_acum_weight(VoteWeight) + amount(u64) * duration(u64)
pub type WeightFactors = (VoteWeight, u64, u64);

pub struct ZeroVoteWeightError;

pub trait ComputeVoteWeight<AccountId> {
    /// The entity that holds the funds of claimers.
    type Claimee;
    type Error: From<ZeroVoteWeightError>;

    fn claimer_weight_factors(_: &AccountId, _: &Self::Claimee, _: u64) -> WeightFactors;
    fn claimee_weight_factors(_: &Self::Claimee, _: u64) -> WeightFactors;

    fn settle_claimer_weight(
        who: &AccountId,
        target: &Self::Claimee,
        current_block: u64,
    ) -> VoteWeight {
        Self::calc_latest_vote_weight(Self::claimer_weight_factors(who, target, current_block))
    }

    fn settle_claimee_weight(target: &Self::Claimee, current_block: u64) -> VoteWeight {
        Self::calc_latest_vote_weight(Self::claimee_weight_factors(target, current_block))
    }

    fn settle_weight_on_claim(
        who: &AccountId,
        target: &Self::Claimee,
        current_block: u64,
    ) -> Result<(VoteWeight, VoteWeight), Self::Error> {
        let claimer_weight = Self::settle_claimer_weight(who, target, current_block);

        if claimer_weight == 0 {
            return Err(ZeroVoteWeightError.into());
        }

        let claimee_weight = Self::settle_claimee_weight(target, current_block);

        Ok((claimer_weight, claimee_weight))
    }

    fn calc_latest_vote_weight(weight_factors: WeightFactors) -> VoteWeight {
        let (last_acum_weight, amount, duration) = weight_factors;
        last_acum_weight + VoteWeight::from(amount) * VoteWeight::from(duration)
    }
}

pub trait Claim<AccountId> {
    type Claimee;
    type Error;

    fn claim(claimer: &AccountId, claimee: &Self::Claimee) -> Result<(), Self::Error>;
}
