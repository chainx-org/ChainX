use super::*;
use codec::{Decode, Encode};
use frame_support::storage::IterableStorageDoubleMap;
use sp_runtime::RuntimeDebug;
#[cfg(feature = "std")]
use sp_runtime::{Deserialize, Serialize};
use sp_std::collections::btree_map::BTreeMap;
use xpallet_support::{RpcBalance, RpcWeightType};

/// Mining weight properties of asset miners.
///
/// Aside from the mining weight information, this struct also contains
/// the `last_claim` field, for it's not neccessary to use another
/// storeage item due to the claim restrictions of asset miners.
#[derive(PartialEq, Eq, Clone, Default, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct RpcMinerLedger<BlockNumber> {
    /// Last calculated total vote weight of current validator.
    pub last_mining_weight: RpcWeightType,
    /// Block number at which point `last_total_vote_weight` just updated.
    pub last_mining_weight_update: BlockNumber,
    /// Block number at which point the miner claimed last time.
    pub last_claim: Option<BlockNumber>,
}

impl<BlockNumber> From<MinerLedger<BlockNumber>> for RpcMinerLedger<BlockNumber> {
    fn from(ledger: MinerLedger<BlockNumber>) -> Self {
        Self {
            last_mining_weight: ledger.last_mining_weight.into(),
            last_mining_weight_update: ledger.last_mining_weight_update,
            last_claim: ledger.last_claim,
        }
    }
}

/// Vote weight properties of validator.
#[derive(PartialEq, Eq, Clone, Default, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct RpcAssetLedger<BlockNumber> {
    /// Last calculated total vote weight of current validator.
    pub last_total_mining_weight: RpcWeightType,
    /// Block number at which point `last_total_vote_weight` just updated.
    pub last_total_mining_weight_update: BlockNumber,
}

impl<BlockNumber> From<AssetLedger<BlockNumber>> for RpcAssetLedger<BlockNumber> {
    fn from(ledger: AssetLedger<BlockNumber>) -> Self {
        Self {
            last_total_mining_weight: ledger.last_total_mining_weight.into(),
            last_total_mining_weight_update: ledger.last_total_mining_weight_update,
        }
    }
}

/// Mining asset info.
#[derive(PartialEq, Eq, Clone, Default, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct MiningAssetInfo<AccountId, RpcBalance, BlockNumber> {
    pub asset_id: AssetId,
    pub mining_power: FixedAssetPower,
    pub reward_pot: AccountId,
    pub reward_pot_balance: RpcBalance,
    #[cfg_attr(feature = "std", serde(flatten))]
    pub ledger_info: RpcAssetLedger<BlockNumber>,
}

impl<T: Trait> Module<T> {
    /// Get overall information about all mining assets.
    #[allow(clippy::type_complexity)]
    pub fn mining_assets(
    ) -> Vec<MiningAssetInfo<T::AccountId, RpcBalance<BalanceOf<T>>, T::BlockNumber>> {
        MiningPrevilegedAssets::get()
            .into_iter()
            .map(|asset_id| {
                let mining_power = FixedAssetPowerOf::get(asset_id);
                let reward_pot = T::DetermineRewardPotAccount::reward_pot_account_for(&asset_id);
                let reward_pot_balance: RpcBalance<BalanceOf<T>> =
                    Self::free_balance(&reward_pot).into();
                let ledger_info: RpcAssetLedger<T::BlockNumber> =
                    AssetLedgers::<T>::get(asset_id).into();
                MiningAssetInfo {
                    asset_id,
                    mining_power,
                    reward_pot,
                    reward_pot_balance,
                    ledger_info,
                }
            })
            .collect()
    }

    /// Get the asset mining dividends info given the staker AccountId.
    pub fn mining_dividend(who: T::AccountId) -> BTreeMap<AssetId, RpcBalance<BalanceOf<T>>> {
        let current_block = <frame_system::Module<T>>::block_number();
        MinerLedgers::<T>::iter_prefix(&who)
            .filter_map(|(asset_id, _)| {
                match Self::compute_dividend_at(&who, &asset_id, current_block) {
                    Ok(dividend) => Some((asset_id, dividend.into())),
                    Err(_) => None,
                }
            })
            .collect()
    }

    /// Get the nomination details given the staker AccountId.
    pub fn miner_ledger(who: T::AccountId) -> BTreeMap<AssetId, RpcMinerLedger<T::BlockNumber>> {
        MinerLedgers::<T>::iter_prefix(&who)
            .map(|(asset_id, ledger)| (asset_id, ledger.into()))
            .collect()
    }
}
