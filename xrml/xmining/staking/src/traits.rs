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

pub trait VoteWeight<BlockNumber: As<u64>> {
    fn amount(&self) -> u64;
    fn set_amount(&mut self, value: u64, to_add: bool);

    fn last_acum_weight(&self) -> u64;
    fn set_last_acum_weight(&mut self, s: u64);

    fn last_acum_weight_update(&self) -> u64;
    fn set_last_acum_weight_update(&mut self, num: BlockNumber);

    fn latest_acum_weight(&self, current_block: BlockNumber) -> u64 {
        self.last_acum_weight()
            + self.amount() * (current_block.as_() - self.last_acum_weight_update())
    }
}
