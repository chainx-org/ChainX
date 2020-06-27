use crate::VoteWeight;

/// Define the get and set methods for the vote weight operations.
pub trait BaseVoteWeight<BlockNumber> {
    fn amount(&self) -> u64;
    fn set_amount(&mut self, new: u64);

    fn last_acum_weight(&self) -> VoteWeight;
    fn set_last_acum_weight(&mut self, s: VoteWeight);

    fn last_acum_weight_update(&self) -> u64;
    fn set_last_acum_weight_update(&mut self, num: BlockNumber);
}

pub enum Delta {
    Add(u64),
    Sub(u64),
    Zero,
}

/// General logic for stage changes of the vote weight operations.
pub trait ComputeVoteWight<BlockNumber>: BaseVoteWeight<BlockNumber> {
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
