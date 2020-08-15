#![cfg_attr(not(feature = "std"), no_std)]

//! A crate which contains primitives that are useful for implementation that uses staking
//! approaches in general. Definitions related to sessions, slashing, etc go here.

use chainx_primitives::AssetId;
use sp_std::prelude::Vec;

/// Simple index type with which we can count sessions.
pub type SessionIndex = u32;

/// Simple index type with which we can count unbonded entries.
pub type UnbondedIndex = u32;

/// Type for measuring the non-validator entity's mining power.
pub type MiningPower = u128;

/// Trait to retrieve and operate on Asset Mining participants in Staking.
pub trait AssetMining<Balance> {
    /// Collects the mining power of all mining assets.
    fn asset_mining_power() -> Vec<(AssetId, MiningPower)>;

    /// Issues reward to the reward pot of an Asset.
    fn reward(_asset_id: AssetId, _reward_value: Balance);

    /// Returns the mining power of all mining assets.
    fn total_asset_mining_power() -> MiningPower {
        Self::asset_mining_power()
            .iter()
            .map(|(_, power)| power)
            .sum()
    }
}

impl<Balance> AssetMining<Balance> for () {
    fn asset_mining_power() -> Vec<(AssetId, MiningPower)> {
        Vec::new()
    }

    fn reward(_: AssetId, _: Balance) {}
}

