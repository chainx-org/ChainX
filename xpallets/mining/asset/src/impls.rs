use super::*;
use xp_staking::{BaseVoteWeight, ComputeVoteWeight, VoteWeight, WeightFactors};

impl<'a, T: Trait> BaseVoteWeight<T::BlockNumber> for AssetLedgerWrapper<'a, T> {
    fn amount(&self) -> u128 {
        xpallet_assets::Module::<T>::all_type_total_asset_balance(&self.asset_id).saturated_into()
    }

    fn set_amount(&mut self, _new: u128) {}

    fn last_acum_weight(&self) -> VoteWeight {
        self.inner.last_total_mining_weight
    }

    fn set_last_acum_weight(&mut self, latest_mining_weight: VoteWeight) {
        self.inner.last_total_mining_weight = latest_mining_weight;
    }

    fn last_acum_weight_update(&self) -> u32 {
        self.inner
            .last_total_mining_weight_update
            .saturated_into::<u32>()
    }

    fn set_last_acum_weight_update(&mut self, current_block: T::BlockNumber) {
        self.inner.last_total_mining_weight_update = current_block;
    }
}

impl<'a, T: Trait> BaseVoteWeight<T::BlockNumber> for MinerLedgerWrapper<'a, T> {
    fn amount(&self) -> u128 {
        xpallet_assets::Module::<T>::all_type_asset_balance(&self.miner, &self.asset_id)
            .saturated_into()
    }

    fn set_amount(&mut self, _new: u128) {}

    fn last_acum_weight(&self) -> VoteWeight {
        self.inner.last_mining_weight
    }

    fn set_last_acum_weight(&mut self, latest_mining_weight: VoteWeight) {
        self.inner.last_mining_weight = latest_mining_weight;
    }

    fn last_acum_weight_update(&self) -> u32 {
        self.inner.last_mining_weight_update.saturated_into::<u32>()
    }

    fn set_last_acum_weight_update(&mut self, current_block: T::BlockNumber) {
        self.inner.last_mining_weight_update = current_block;
    }
}

fn prepare_weight_factors<T: Trait, V: BaseVoteWeight<T::BlockNumber>>(
    wrapper: V,
    current_block: u32,
) -> WeightFactors {
    (
        wrapper.last_acum_weight(),
        wrapper.amount(),
        current_block - wrapper.last_acum_weight_update(),
    )
}

impl<T: Trait> ComputeVoteWeight<T::AccountId> for Module<T> {
    type Claimee = AssetId;
    type Error = Error<T>;

    fn claimer_weight_factors(
        who: &T::AccountId,
        target: &Self::Claimee,
        current_block: u32,
    ) -> WeightFactors {
        let mut inner = MinerLedgers::<T>::get(who, target);
        let wrapper = MinerLedgerWrapper::<T>::new(who, target, &mut inner);
        prepare_weight_factors::<T, _>(wrapper, current_block)
    }

    fn claimee_weight_factors(target: &Self::Claimee, current_block: u32) -> WeightFactors {
        let mut inner = AssetLedgers::<T>::get(target);
        let wrapper = AssetLedgerWrapper::<T>::new(target, &mut inner);
        prepare_weight_factors::<T, _>(wrapper, current_block)
    }
}
