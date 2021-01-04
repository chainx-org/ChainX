// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use sp_std::{collections::btree_map::BTreeMap, vec::Vec};

use codec::{Decode, Encode};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

use frame_support::storage::{IterableStorageDoubleMap, StorageMap, StorageValue};
use sp_runtime::{RuntimeDebug, SaturatedConversion};

use chainx_primitives::AssetId;
use xp_mining_common::RewardPotAccountFor;

use crate::{
    types::*, AssetLedgers, BalanceOf, ClaimRestrictionOf, Config, FixedAssetPowerOf, MinerLedgers,
    MiningPrevilegedAssets, Module,
};

/// Mining asset info.
#[derive(PartialEq, Eq, Clone, Default, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct MiningAssetInfo<AccountId, Balance, MiningWeight, BlockNumber> {
    pub asset_id: AssetId,
    pub mining_power: FixedAssetPower,
    pub reward_pot: AccountId,
    pub reward_pot_balance: Balance,
    #[cfg_attr(feature = "std", serde(flatten))]
    pub ledger_info: AssetLedger<MiningWeight, BlockNumber>,
}

/// Detailed dividend info of asset miner.
#[derive(PartialEq, Eq, Clone, Default, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct MiningDividendInfo<Balance> {
    /// Actual dividend balance that belongs to the claimer.
    pub own: Balance,
    /// Dividend cut(10% of total) for the referral of claimer or treasury.
    pub other: Balance,
    /// Required more stake to claim the mining dividend.
    pub insufficient_stake: Balance,
}

impl<T: Config> Module<T> {
    /// Get overall information about all mining assets.
    pub fn mining_assets(
    ) -> Vec<MiningAssetInfo<T::AccountId, BalanceOf<T>, MiningWeight, T::BlockNumber>> {
        MiningPrevilegedAssets::get()
            .into_iter()
            .map(|asset_id| {
                let mining_power = FixedAssetPowerOf::get(asset_id);
                let reward_pot = T::DetermineRewardPotAccount::reward_pot_account_for(&asset_id);
                let reward_pot_balance: BalanceOf<T> = Self::free_balance(&reward_pot);
                let ledger_info: AssetLedger<MiningWeight, T::BlockNumber> =
                    AssetLedgers::<T>::get(asset_id);
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

    /// Get the asset mining dividends info given the miner AccountId.
    pub fn mining_dividend(
        who: T::AccountId,
    ) -> BTreeMap<AssetId, MiningDividendInfo<BalanceOf<T>>> {
        let current_block = <frame_system::Module<T>>::block_number();
        MinerLedgers::<T>::iter_prefix(&who)
            .filter_map(|(asset_id, _)| {
                match Self::compute_dividend_at(&who, &asset_id, current_block) {
                    Ok(dividend) => {
                        let ClaimRestriction {
                            staking_requirement,
                            ..
                        } = ClaimRestrictionOf::<T>::get(&asset_id);
                        let insufficient_stake =
                            Self::need_more_stake(&who, dividend, staking_requirement)
                                .unwrap_or_default();
                        let other = dividend / 10u32.saturated_into::<BalanceOf<T>>();
                        let own = dividend - other;
                        Some((
                            asset_id,
                            MiningDividendInfo {
                                own,
                                other,
                                insufficient_stake,
                            },
                        ))
                    }
                    Err(_) => None,
                }
            })
            .collect()
    }

    /// Get the nomination details given the staker AccountId.
    pub fn miner_ledger(
        who: T::AccountId,
    ) -> BTreeMap<AssetId, MinerLedger<MiningWeight, T::BlockNumber>> {
        MinerLedgers::<T>::iter_prefix(&who).collect()
    }
}
