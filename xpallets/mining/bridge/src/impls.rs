use sp_std::{vec, vec::Vec};

use codec::Encode;
use frame_support::traits::{Currency, Get};
use sp_arithmetic::traits::{SaturatedConversion, Saturating};
use sp_core::crypto::UncheckedFrom;
use sp_runtime::traits::Hash;

use chainx_primitives::AssetId;
use xp_mining_common::RewardPotAccountFor;
use xp_mining_staking::MiningPower;
use xpallet_assets::BalanceOf;

use crate::pallet::{Config, Event, Pallet};
use crate::types::BridgeSubPot;

impl<T: Config> Pallet<T> {
    /// Divident reward for bridge reward pot
    ///
    /// result returned by (user_reward, vault_reward).
    fn dividend_reward(total: BalanceOf<T>) -> (BalanceOf<T>, BalanceOf<T>) {
        let vault_reward: BalanceOf<T> = (total.saturated_into::<u128>() / 10).saturated_into();
        let user_reward = total.saturating_sub(vault_reward);
        (user_reward, vault_reward)
    }
}

impl<T: Config> xp_mining_staking::AssetMining<BalanceOf<T>> for Pallet<T> {
    fn asset_mining_power() -> Vec<(AssetId, MiningPower)> {
        let total_issuance = xpallet_assets::Module::<T>::total_issuance(&T::TargetAssetId::get());
        vec![(
            T::TargetAssetId::get(),
            T::TargetAssetMiningPower::get()
                .saturating_mul(total_issuance.saturated_into())
                .saturated_into::<MiningPower>(),
        )]
    }

    fn reward(asset_id: AssetId, reward_value: BalanceOf<T>) {
        let (user_reward, vault_reward) = Self::dividend_reward(reward_value);
        // Reward
        // | ---> User Pot(90%)
        // | ---> Vault Pot(10%)
        let user_reward_pot =
            T::DetermineRewardPotAccount::reward_pot_account_for(&(asset_id, BridgeSubPot::User));
        T::Currency::deposit_creating(&user_reward_pot, user_reward);

        let vault_reward_pot =
            T::DetermineRewardPotAccount::reward_pot_account_for(&(asset_id, BridgeSubPot::Vault));
        T::Currency::deposit_creating(&vault_reward_pot, vault_reward);

        Self::deposit_event(Event::<T>::Minted(
            user_reward_pot,
            user_reward,
            vault_reward_pot,
            vault_reward,
        ));
    }
}

pub struct BridgeRewardPotAccountDeterminer<T: Config>(sp_std::marker::PhantomData<T>);

impl<T: Config> xp_mining_common::RewardPotAccountFor<T::AccountId, (AssetId, BridgeSubPot)>
    for BridgeRewardPotAccountDeterminer<T>
where
    T::AccountId: UncheckedFrom<T::Hash> + AsRef<[u8]>,
{
    fn reward_pot_account_for(entity: &(AssetId, BridgeSubPot)) -> T::AccountId {
        let (asset_id, subpot) = *entity;
        let id: u64 = (asset_id as u64) << 32 & subpot as u64;
        let id_hash = T::Hashing::hash(&id.to_le_bytes()[..]);
        let registered_block = <xpallet_assets_registrar::Module<T>>::registered_at(asset_id);
        let registered_block_hash =
            <T as frame_system::Config>::Hashing::hash(registered_block.encode().as_ref());

        let id_slice = id_hash.as_ref();
        let registered_slice = registered_block_hash.as_ref();

        let mut buf = Vec::with_capacity(id_slice.len() + registered_slice.len());
        buf.extend_from_slice(id_slice);
        buf.extend_from_slice(registered_slice);

        UncheckedFrom::unchecked_from(T::Hashing::hash(&buf[..]))
    }
}
