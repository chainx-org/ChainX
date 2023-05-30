// Copyright 2019-2023 ChainX Project Authors. Licensed under GPL-3.0.

use codec::{Decode, Encode};
use scale_info::TypeInfo;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::RuntimeDebug;

use chainx_primitives::AssetId;
use xp_mining_common::WeightType;

use crate::Config;

pub type MiningWeight = WeightType;
pub type FixedAssetPower = u32;
pub type StakingRequirement = u32;

/// Vote weight properties of validator.
#[derive(PartialEq, Eq, Clone, Default, Encode, Decode, RuntimeDebug, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct AssetLedger<MiningWeight, BlockNumber> {
    /// Last calculated total vote weight of current validator.
    pub last_total_mining_weight: MiningWeight,
    /// Block number at which point `last_total_vote_weight` just updated.
    pub last_total_mining_weight_update: BlockNumber,
}

pub struct AssetLedgerWrapper<'a, T: Config> {
    pub asset_id: &'a AssetId,
    pub inner: &'a mut AssetLedger<MiningWeight, T::BlockNumber>,
}

impl<'a, T: Config> AssetLedgerWrapper<'a, T> {
    pub fn new(
        asset_id: &'a AssetId,
        inner: &'a mut AssetLedger<MiningWeight, T::BlockNumber>,
    ) -> Self {
        Self { asset_id, inner }
    }
}

/// Mining weight properties of asset miners.
///
/// Aside from the mining weight information, this struct also contains
/// the `last_claim` field, for it's not neccessary to use another
/// storeage item due to the claim restrictions of asset miners.
#[derive(PartialEq, Eq, Clone, Default, Encode, Decode, RuntimeDebug, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct MinerLedger<MiningWeight, BlockNumber> {
    /// Last calculated total vote weight of current validator.
    pub last_mining_weight: MiningWeight,
    /// Block number at which point `last_total_vote_weight` just updated.
    pub last_mining_weight_update: BlockNumber,
    /// Block number at which point the miner claimed last time.
    pub last_claim: Option<BlockNumber>,
}

pub struct MinerLedgerWrapper<'a, T: Config> {
    pub miner: &'a T::AccountId,
    pub asset_id: &'a AssetId,
    pub inner: &'a mut MinerLedger<MiningWeight, T::BlockNumber>,
}

impl<'a, T: Config> MinerLedgerWrapper<'a, T> {
    pub fn new(
        miner: &'a T::AccountId,
        asset_id: &'a AssetId,
        inner: &'a mut MinerLedger<MiningWeight, T::BlockNumber>,
    ) -> Self {
        Self {
            miner,
            asset_id,
            inner,
        }
    }
}

#[derive(PartialEq, Eq, Clone, Default, Encode, Decode, RuntimeDebug, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct ClaimRestriction<BlockNumber> {
    /// Claimer must have `staking_requirement` times of PCX staked.
    pub staking_requirement: StakingRequirement,
    /// Claimer can only claim once per `frequency_limit`.
    pub frequency_limit: BlockNumber,
}
