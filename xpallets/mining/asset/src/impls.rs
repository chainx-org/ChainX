use super::*;
use xp_staking::{BaseVoteWeight, Claim, ComputeVoteWeight, VoteWeight, WeightFactors};

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

fn generic_weight_factors<T: Trait, V: BaseVoteWeight<T::BlockNumber>>(
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
        generic_weight_factors::<T, _>(wrapper, current_block)
    }

    fn claimee_weight_factors(target: &Self::Claimee, current_block: u32) -> WeightFactors {
        let mut inner = AssetLedgers::<T>::get(target);
        let wrapper = AssetLedgerWrapper::<T>::new(target, &mut inner);
        generic_weight_factors::<T, _>(wrapper, current_block)
    }
}

impl<T: Trait> xpallet_assets::OnAssetChanged<T::AccountId, T::Balance> for Module<T> {
    fn on_issue_pre(target: &AssetId, source: &T::AccountId) {
        let current_block = <frame_system::Module<T>>::block_number();
        Self::init_receiver_mining_ledger(source, target, current_block);

        Self::update_mining_weights(source, target, current_block);
    }

    fn on_issue_post(target: &AssetId, source: &T::AccountId, value: T::Balance) -> DispatchResult {
        Self::issue_reward(source, target);
        Ok(())
    }

    fn on_move_pre(
        asset_id: &AssetId,
        from: &T::AccountId,
        _: AssetType,
        to: &T::AccountId,
        _: AssetType,
        _: T::Balance,
    ) {
        let current_block = <frame_system::Module<T>>::block_number();
        Self::init_receiver_mining_ledger(to, asset_id, current_block);

        Self::update_miner_mining_weight(from, asset_id, current_block);
        Self::update_miner_mining_weight(to, asset_id, current_block);
    }

    fn on_destroy_pre(target: &AssetId, source: &T::AccountId) {
        let current_block = <frame_system::Module<T>>::block_number();
        Self::update_mining_weights(source, target, current_block);
    }
}

impl<T: Trait> Module<T> {
    fn allocate_dividend(
        claimer: &T::AccountId,
        claimee: &AssetId,
        claimee_jackpot: &T::AccountId,
        dividend: T::Balance,
    ) -> Result<(), Error<T>> {
        todo!("")
    }
}

impl<T: Trait> Claim<T::AccountId> for Module<T> {
    type Claimee = AssetId;
    type Error = Error<T>;

    fn claim(claimer: &T::AccountId, claimee: &Self::Claimee) -> Result<(), Error<T>> {
        let current_block = <frame_system::Module<T>>::block_number();

        let (source_weight, target_weight) =
            <Self as ComputeVoteWeight<T::AccountId>>::settle_weight_on_claim(
                claimer,
                claimee,
                current_block.saturated_into::<u32>(),
            )?;

        let claimee_jackpot = Self::asset_jackpot_of(claimee);
        let dividend = Self::compute_dividend(source_weight, target_weight, &claimee_jackpot);

        Self::can_claim(claimer, claimee, dividend, current_block)?;

        Self::allocate_dividend(claimer, claimee, &claimee_jackpot, dividend)?;

        Self::apply_update_miner_mining_weight(claimer, claimee, 0, current_block);
        Self::apply_update_asset_mining_weight(
            claimee,
            target_weight - source_weight,
            current_block,
        );

        MinerLedgers::<T>::mutate(claimer, claimee, |miner_ledger| {
            miner_ledger.last_claim = Some(current_block);
        });

        Ok(())
    }
}

impl<T: Trait> xpallet_assets::OnAssetRegisterOrRevoke for Module<T> {
    fn on_register(asset_id: &AssetId, has_mining_rights: bool) -> DispatchResult {
        if !has_mining_rights {
            return Ok(());
        }
        MiningPrevilegedAssets::mutate(|i| i.push(*asset_id));
        AssetLedgers::<T>::insert(
            asset_id,
            AssetLedger {
                last_total_mining_weight_update: <frame_system::Module<T>>::block_number(),
                ..Default::default()
            },
        );
        Ok(())
    }

    fn on_revoke(asset_id: &AssetId) -> DispatchResult {
        MiningPrevilegedAssets::mutate(|v| {
            v.retain(|i| i != asset_id);
        });
        Ok(())
    }
}
