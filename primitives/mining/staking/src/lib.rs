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

///
pub trait CollectAssetMiningInfo {
    ///
    fn collect_asset_mining_info() -> Vec<(AssetId, MiningPower)>;

    ///
    fn total_asset_mining_power() -> MiningPower {
        Self::collect_asset_mining_info()
            .iter()
            .map(|(_, power)| power)
            .sum()
    }
}

impl CollectAssetMiningInfo for () {
    fn collect_asset_mining_info() -> Vec<(AssetId, MiningPower)> {
        Vec::new()
    }
}

/// Issue the fresh PCX to the non-validator mining entities.
pub trait OnMinting<MiningEntity, Balance> {
    fn mint(_: &MiningEntity, _: Balance);
}

impl<MiningEntity, Balance> OnMinting<MiningEntity, Balance> for () {
    fn mint(_: &MiningEntity, _: Balance) {}
}

/// This trait provides a simple way to get the treasury account.
pub trait TreasuryAccount<AccountId> {
    fn treasury_account() -> AccountId;
}

impl<AccountId: Default> TreasuryAccount<AccountId> for () {
    fn treasury_account() -> AccountId {
        Default::default()
    }
}
