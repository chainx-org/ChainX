use crate::Trait;
use chainx_primitives::AssetId;
use codec::{Decode, Encode};
use sp_runtime::RuntimeDebug;
#[cfg(feature = "std")]
use sp_runtime::{Deserialize, Serialize};
use xp_staking::VoteWeight;

pub type MiningWeight = u128;
pub type FixedAssetPower = u32;
pub type StakingRequirement = u32;

/// Vote weight properties of validator.
#[derive(PartialEq, Eq, Clone, Default, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct AssetLedger<BlockNumber> {
    /// Last calculated total vote weight of current validator.
    pub last_total_mining__weight: MiningWeight,
    /// Block number at which point `last_total_vote_weight` just updated.
    pub last_total_mining_weight_update: BlockNumber,
}

pub struct AssetLedgerWrapper<'a, T: Trait> {
    pub asset: AssetId,
    pub mining: &'a mut AssetLedger<T::BlockNumber>,
}

/// Mining weight properties of asset miners.
///
/// Aside from the mining weight information, this struct also contains
/// the `last_claim` field, for it's not neccessary to use another
/// storeage item due to the claim restrictions of asset miners.
#[derive(PartialEq, Eq, Clone, Default, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct MinerLedger<BlockNumber> {
    /// Last calculated total vote weight of current validator.
    pub last_mining__weight: MiningWeight,
    /// Block number at which point `last_total_vote_weight` just updated.
    pub last_mining_weight_update: BlockNumber,
    /// Block number at which point the miner claimed last time.
    pub last_claim: Option<BlockNumber>,
}

pub struct MinerLedgerWrapper<'a, T: Trait> {
    pub miner: &'a T::AccountId,
    pub asset: AssetId,
    pub mining: &'a mut MinerLedger<T::BlockNumber>,
}

#[derive(PartialEq, Eq, Clone, Default, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct ClaimRestriction<BlockNumber> {
    /// Claimer must have `staking_requirement` times of PCX staked.
    pub staking_requirement: StakingRequirement,
    /// Claimer can only claim once per `frequency_limit`.
    pub frequency_limit: BlockNumber,
}
