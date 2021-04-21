// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

#![cfg_attr(not(feature = "std"), no_std)]

//! A crate which contains primitives that are useful for implementation that uses staking
//! approaches in general. Definitions related to sessions, slashing, etc go here.

use chainx_primitives::AssetId;

use sp_std::{vec, prelude::Vec};

use impl_trait_for_tuples::{impl_for_tuples};

/// Simple index type with which we can count sessions.
pub type SessionIndex = u32;

/// Simple index type with which we can count unbonded entries.
pub type UnbondedIndex = u32;

/// Type for measuring the non-validator entity's mining power.
pub type MiningPower = u128;

/// Trait to retrieve and operate on Asset Mining participants in Staking.
pub trait AssetMining<Balance: Copy + Clone> {
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

#[impl_for_tuples(1,5)]
impl<Balance: Copy + Clone> AssetMining<Balance> for TupleIdentifier {
    fn asset_mining_power() -> Vec<(AssetId, MiningPower)> {
        let mut result = vec![];
        for_tuples!( #( result.extend(TupleIdentifier::asset_mining_power()); )* );
        result
    }

    fn reward(asset_id: AssetId, reward_value: Balance) {
       for_tuples!( #(TupleIdentifier::reward(asset_id, reward_value);)* ) 
    }
}

impl<Balance: Copy + Clone> AssetMining<Balance> for () {
    fn asset_mining_power() -> Vec<(AssetId, MiningPower)> {
        Vec::new()
    }

    fn reward(_: AssetId, _: Balance) {}
}
