use super::*;
use codec::Encode;
use sp_core::crypto::UncheckedFrom;
use sp_runtime::traits::Hash;
use sp_runtime::traits::Saturating;
use xp_mining_common::{
    compute_dividend, generic_weight_factors, BaseMiningWeight, Claim, ComputeMiningWeight,
    WeightFactors, WeightType,
};
use xp_mining_staking::MiningPower;

impl<'a, T: Trait> BaseMiningWeight<T::Balance, T::BlockNumber> for AssetLedgerWrapper<'a, T> {
    fn amount(&self) -> T::Balance {
        xpallet_assets::Module::<T>::all_type_total_asset_balance(&self.asset_id)
    }

    fn last_acum_weight(&self) -> WeightType {
        self.inner.last_total_mining_weight
    }

    fn set_last_acum_weight(&mut self, latest_mining_weight: WeightType) {
        self.inner.last_total_mining_weight = latest_mining_weight;
    }

    fn last_acum_weight_update(&self) -> T::BlockNumber {
        self.inner.last_total_mining_weight_update
    }

    fn set_last_acum_weight_update(&mut self, current_block: T::BlockNumber) {
        self.inner.last_total_mining_weight_update = current_block;
    }
}

impl<'a, T: Trait> BaseMiningWeight<T::Balance, T::BlockNumber> for MinerLedgerWrapper<'a, T> {
    fn amount(&self) -> T::Balance {
        xpallet_assets::Module::<T>::all_type_asset_balance(&self.miner, &self.asset_id)
    }

    fn last_acum_weight(&self) -> WeightType {
        self.inner.last_mining_weight
    }

    fn set_last_acum_weight(&mut self, latest_mining_weight: WeightType) {
        self.inner.last_mining_weight = latest_mining_weight;
    }

    fn last_acum_weight_update(&self) -> T::BlockNumber {
        self.inner.last_mining_weight_update
    }

    fn set_last_acum_weight_update(&mut self, current_block: T::BlockNumber) {
        self.inner.last_mining_weight_update = current_block;
    }
}

impl<T: Trait> ComputeMiningWeight<T::AccountId, T::BlockNumber> for Module<T> {
    type Claimee = AssetId;
    type Error = Error<T>;

    fn claimer_weight_factors(
        who: &T::AccountId,
        target: &Self::Claimee,
        current_block: T::BlockNumber,
    ) -> WeightFactors {
        let mut inner = MinerLedgers::<T>::get(who, target);
        let wrapper = MinerLedgerWrapper::<T>::new(who, target, &mut inner);
        generic_weight_factors::<T::Balance, T::BlockNumber, _>(wrapper, current_block)
    }

    fn claimee_weight_factors(
        target: &Self::Claimee,
        current_block: T::BlockNumber,
    ) -> WeightFactors {
        let mut inner = AssetLedgers::<T>::get(target);
        let wrapper = AssetLedgerWrapper::<T>::new(target, &mut inner);
        generic_weight_factors::<T::Balance, T::BlockNumber, _>(wrapper, current_block)
    }
}

impl<T: Trait> xpallet_assets::OnAssetChanged<T::AccountId, T::Balance> for Module<T> {
    fn on_issue_pre(target: &AssetId, source: &T::AccountId) {
        let current_block = <frame_system::Module<T>>::block_number();
        Self::init_receiver_mining_ledger(source, target, current_block);

        Self::update_mining_weights(source, target, current_block);
    }

    fn on_issue_post(
        target: &AssetId,
        source: &T::AccountId,
        _value: T::Balance,
    ) -> DispatchResult {
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
        _claimer: &T::AccountId,
        _claimee: &AssetId,
        _claimee_reward_pot: &T::AccountId,
        _dividend: T::Balance,
    ) -> Result<(), Error<T>> {
        // todo!("referral_or_treasury 10%, claimer 90%")
        println!("allocate_dividend");
        Ok(())
    }
}

impl<T: Trait> Claim<T::AccountId> for Module<T> {
    type Claimee = AssetId;
    type Error = Error<T>;

    fn claim(claimer: &T::AccountId, claimee: &Self::Claimee) -> Result<(), Error<T>> {
        let current_block = <frame_system::Module<T>>::block_number();

        let (source_weight, target_weight) = <Self as ComputeMiningWeight<
            T::AccountId,
            T::BlockNumber,
        >>::settle_weight_on_claim(
            claimer, claimee, current_block
        )?;

        let claimee_reward_pot = T::DetermineRewardPotAccount::reward_pot_account_for(claimee);
        let dividend = compute_dividend::<T::AccountId, T::Balance, _>(
            source_weight,
            target_weight,
            &claimee_reward_pot,
            xpallet_assets::Module::<T>::pcx_free_balance,
        );

        Self::can_claim(claimer, claimee, dividend, current_block)?;

        Self::allocate_dividend(claimer, claimee, &claimee_reward_pot, dividend)?;

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

/// Simple Asset reward pot account determiner.
///
/// Formula: `blake2_256(blake2_256(asset_id) + blake2_256(registered_block_number))`
pub struct SimpleAssetRewardPotAccountDeterminer<T: Trait>(sp_std::marker::PhantomData<T>);

impl<T: Trait> xp_mining_common::RewardPotAccountFor<T::AccountId, AssetId>
    for SimpleAssetRewardPotAccountDeterminer<T>
where
    T::AccountId: UncheckedFrom<T::Hash> + AsRef<[u8]>,
{
    fn reward_pot_account_for(asset_id: &AssetId) -> T::AccountId {
        let id_hash = T::Hashing::hash(&asset_id.to_le_bytes()[..]);
        let registered_block = <xpallet_assets::Module<T>>::asset_registered_block(asset_id);
        let registered_block_hash =
            <T as frame_system::Trait>::Hashing::hash(registered_block.encode().as_ref());

        let id_slice = id_hash.as_ref();
        let registered_slice = registered_block_hash.as_ref();

        let mut buf = Vec::with_capacity(id_slice.len() + registered_slice.len());
        buf.extend_from_slice(id_slice);
        buf.extend_from_slice(registered_slice);

        UncheckedFrom::unchecked_from(T::Hashing::hash(&buf[..]))
    }
}

impl<T: Trait> xp_mining_staking::AssetMining<T::Balance> for Module<T> {
    /// Collects the mining power of all mining assets.
    fn asset_mining_power() -> Vec<(AssetId, MiningPower)> {
        // Currently only X-BTC asset.
        XTypeAssetPowerMap::iter()
            .map(|(asset_id, fixed_power)| {
                let total_balance =
                    <xpallet_assets::Module<T>>::all_type_total_asset_balance(&asset_id);
                (
                    asset_id,
                    total_balance
                        .saturating_mul(fixed_power.saturated_into())
                        .saturated_into::<MiningPower>(),
                )
            })
            .collect()
    }

    /// Issues reward to the reward pot of an Asset.
    fn reward(asset_id: AssetId, value: T::Balance) {
        let reward_pot = T::DetermineRewardPotAccount::reward_pot_account_for(&asset_id);
        let _ = xpallet_assets::Module::<T>::pcx_issue(&reward_pot, value);
    }
}
