use super::*;
use xp_staking::{BaseVoteWeight, ComputeVoteWeight, VoteWeight, WeightFactors};

impl<T: Trait> ComputeVoteWeight<T::AccountId> for Module<T> {
    type Claimee = AssetId;
    type Error = Error<T>;

    fn claimer_weight_factors(
        who: &T::AccountId,
        target: &Self::Claimee,
        current_block: u32,
    ) -> WeightFactors {
        todo!()
    }

    fn claimee_weight_factors(target: &Self::Claimee, current_block: u32) -> WeightFactors {
        todo!()
    }
}
