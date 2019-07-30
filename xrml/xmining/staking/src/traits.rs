use super::*;

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

pub trait VoteWeightBase<BlockNumber: As<u64>> {
    fn amount(&self) -> u64;
    fn set_amount(&mut self, new: u64);

    fn last_acum_weight(&self) -> u64;
    fn set_last_acum_weight(&mut self, s: u64);

    fn last_acum_weight_update(&self) -> u64;
    fn set_last_acum_weight_update(&mut self, num: BlockNumber);
}

pub trait VoteWeight<BlockNumber: As<u64>>: VoteWeightBase<BlockNumber> {
    /// Set the new amount after settling the change of nomination.
    fn settle_and_set_amount(&mut self, delta: &Delta) {
        let new = match *delta {
            Delta::Add(x) => self.amount() + x,
            Delta::Sub(x) => self.amount() - x,
        };
        self.set_amount(new);
    }

    /// Set new state on claim.
    ///
    /// This action doesn't involve in a change of amount.
    fn set_state_on_claim(&mut self, latest_acum_weight: u64, current_block: BlockNumber) {
        self.set_last_acum_weight(latest_acum_weight);
        self.set_last_acum_weight_update(current_block);
    }

    /// Set new state on nominate, unnominate and renominate.
    ///
    /// This is similar to set_state_on_claim with the settlement of amount added.
    fn set_state(&mut self, latest_acum_weight: u64, current_block: BlockNumber, delta: &Delta) {
        self.set_last_acum_weight(latest_acum_weight);
        self.set_last_acum_weight_update(current_block);
        self.settle_and_set_amount(delta);
    }

    /// Unsafe settlement of latest_acum_weight.
    ///
    /// FIXME: will be removed once the overflow issue is resolved.
    fn latest_acum_weight(&self, current_block: BlockNumber) -> u64 {
        self.last_acum_weight()
            + self.amount() * (current_block.as_() - self.last_acum_weight_update())
    }
}
