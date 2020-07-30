//! # Asset Mining Module

#![cfg_attr(not(feature = "std"), no_std)]

mod impls;
mod types;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

use frame_support::{
    decl_error, decl_event, decl_module, decl_storage,
    dispatch::DispatchResult,
    ensure,
    storage::IterableStorageMap,
    traits::{Currency, ExistenceRequirement},
};
use frame_system::{self as system, ensure_root, ensure_signed};
use sp_runtime::traits::{SaturatedConversion, Zero};
use sp_std::prelude::*;

use chainx_primitives::AssetId;
use xp_mining_common::{
    Claim, ComputeMiningWeight, MiningWeight, RewardPotAccountFor, WeightType,
    ZeroMiningWeightError,
};
use xp_mining_staking::TreasuryAccount;
use xpallet_assets::AssetType;
use xpallet_support::warn;

use types::*;

pub use impls::SimpleAssetRewardPotAccountDeterminer;

pub type BalanceOf<T> = <<T as xpallet_assets::Trait>::Currency as Currency<
    <T as frame_system::Trait>::AccountId,
>>::Balance;

pub trait Trait: xpallet_assets::Trait {
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

    ///
    type StakingInterface: StakingInterface<Self::AccountId, u128>;

    ///
    type TreasuryAccount: TreasuryAccount<Self::AccountId>;

    ///
    type DetermineRewardPotAccount: RewardPotAccountFor<Self::AccountId, AssetId>;
}

pub trait StakingInterface<AccountId, Balance> {
    fn staked_of(who: &AccountId) -> Balance;
}

impl<T: Trait> StakingInterface<<T as frame_system::Trait>::AccountId, u128> for T
where
    T: xpallet_mining_staking::Trait,
{
    fn staked_of(who: &<T as frame_system::Trait>::AccountId) -> u128 {
        xpallet_mining_staking::Module::<T>::staked_of(who).saturated_into()
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as XMiningAsset {
        ///
        pub DepositReward get(fn deposit_reward): BalanceOf<T> = 100_000.into();

        ///
        pub ClaimRestrictionOf get(fn claim_restriction_of):
            map hasher(twox_64_concat) AssetId => ClaimRestriction<T::BlockNumber>;

        /// External Assets that have the mining rights.
        pub MiningPrevilegedAssets get(fn mining_previleged_assets): Vec<AssetId>;

        /// Mining weight information of the asset.
        pub AssetLedgers get(fn asset_ledgers):
            map hasher(twox_64_concat) AssetId => AssetLedger<T::BlockNumber>;

        /// The map from nominator to the vote weight ledger of all nominees.
        pub MinerLedgers get(fn miner_ledgers):
            double_map hasher(twox_64_concat) T::AccountId, hasher(twox_64_concat) AssetId
            => MinerLedger<T::BlockNumber>;

        /// Mining power map of X-type assets.
        pub XTypeAssetPowerMap get(fn x_type_asset_power_map):
            map hasher(twox_64_concat) AssetId => FixedAssetPower;
    }
    add_extra_genesis {
        config(claim_restrictions): Vec<(AssetId, (StakingRequirement, T::BlockNumber))>;
        config(mining_power_map): Vec<(AssetId, FixedAssetPower)>;
        build(|config| {
            for (asset_id, (staking_requirement, frequency_limit)) in &config.claim_restrictions {
                ClaimRestrictionOf::<T>::insert(asset_id, ClaimRestriction {
                    staking_requirement: *staking_requirement,
                    frequency_limit: *frequency_limit
                });
            }
            for(asset_id, fixed_power) in &config.mining_power_map {
                XTypeAssetPowerMap::insert(asset_id, fixed_power);
            }
        });
    }
}

decl_event!(
    pub enum Event<T>
    where
        Balance = BalanceOf<T>,
        <T as frame_system::Trait>::AccountId,
    {
        ///
        Claim(AccountId, AccountId, Balance),
    }
);

decl_error! {
    /// Error for the staking module.
    pub enum Error for Module<T: Trait> {
        /// The asset does not have the mining rights.
        UnprevilegedAsset,
        /// Claimer does not have enough Staking locked balance.
        InsufficientStaking,
        /// Claimer just did a claim recently, the next frequency limit is not expired.
        UnexpiredFrequencyLimit,
        /// Asset error.
        AssetError,
        /// Zero mining weight.
        ZeroMiningWeight
    }
}

impl<T: Trait> From<ZeroMiningWeightError> for Error<T> {
    fn from(_: ZeroMiningWeightError) -> Self {
        Self::ZeroMiningWeight
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        type Error = Error<T>;

        fn deposit_event() = default;

        /// Claims the staking reward given the `target` validator.
        #[weight = 10]
        fn claim(origin, target: AssetId) {
            let sender = ensure_signed(origin)?;

            ensure!(
                Self::mining_previleged_assets().contains(&target),
                Error::<T>::UnprevilegedAsset
            );

            <Self as Claim<T::AccountId>>::claim(&sender, &target)?;
        }

        #[weight = 10]
        fn set_claim_staking_requirement(origin, asset_id: AssetId, new: StakingRequirement) {
            ensure_root(origin)?;
            ClaimRestrictionOf::<T>::mutate(asset_id, |restriction| {
                restriction.staking_requirement = new;
            });
        }

        #[weight = 10]
        fn set_claim_frequency_limit(origin, asset_id: AssetId, new: T::BlockNumber) {
            ensure_root(origin)?;
            ClaimRestrictionOf::<T>::mutate(asset_id, |restriction| {
                restriction.frequency_limit = new;
            });
        }
    }
}

impl<T: Trait> Module<T> {
    #[inline]
    fn last_claim(who: &T::AccountId, asset_id: &AssetId) -> Option<T::BlockNumber> {
        MinerLedgers::<T>::get(who, asset_id).last_claim
    }

    /// This rule doesn't take effect if the interval is zero.
    fn passed_enough_interval(
        who: &T::AccountId,
        asset_id: &AssetId,
        frequency_limit: T::BlockNumber,
        current_block: T::BlockNumber,
    ) -> Result<(), Error<T>> {
        if !frequency_limit.is_zero() {
            if let Some(last_claim) = Self::last_claim(who, asset_id) {
                if current_block <= last_claim + frequency_limit {
                    warn!(
                        "{:?} can not claim until block {:?}",
                        who,
                        last_claim + frequency_limit
                    );
                    return Err(Error::<T>::UnexpiredFrequencyLimit);
                }
            }
        }
        Ok(())
    }

    /// Returns Ok(_) if the claimer has enough staking locked balance regarding the `total_dividend`.
    ///
    /// This rule doesn't take effect if the staking requirement is zero.
    fn has_enough_staking(
        who: &T::AccountId,
        total_dividend: BalanceOf<T>,
        staking_requirement: StakingRequirement,
    ) -> Result<(), Error<T>> {
        if !staking_requirement.is_zero() {
            let staking_locked = T::StakingInterface::staked_of(who);
            if staking_locked.saturated_into::<BalanceOf<T>>()
                < staking_requirement.saturated_into::<BalanceOf<T>>() * total_dividend
            {
                warn!(
                    "cannot claim due to the insufficient staking, total dividend: {:?}, staking locked: {:?}, required staking: {:?}",
                    total_dividend,
                    staking_locked,
                    staking_requirement.saturated_into::<BalanceOf<T>>() * total_dividend
                );
                return Err(Error::<T>::InsufficientStaking);
            }
        }
        Ok(())
    }

    fn init_receiver_mining_ledger(
        who: &T::AccountId,
        asset_id: &AssetId,
        current_block: T::BlockNumber,
    ) {
        if !MinerLedgers::<T>::contains_key(who, asset_id) {
            MinerLedgers::<T>::insert(
                who,
                asset_id,
                MinerLedger::<T::BlockNumber> {
                    last_mining_weight_update: current_block,
                    ..Default::default()
                },
            );
        }
    }

    fn update_miner_mining_weight(
        from: &T::AccountId,
        target: &AssetId,
        current_block: T::BlockNumber,
    ) {
        let new_weight =
            <Self as ComputeMiningWeight<T::AccountId, T::BlockNumber>>::settle_claimer_weight(
                from,
                target,
                current_block,
            );
        Self::apply_update_miner_mining_weight(from, target, new_weight, current_block);
    }

    fn apply_update_miner_mining_weight(
        from: &T::AccountId,
        target: &AssetId,
        new_weight: WeightType,
        current_block: T::BlockNumber,
    ) {
        let mut inner = MinerLedgers::<T>::get(from, target);
        let mut wrapper = MinerLedgerWrapper::<T>::new(from, target, &mut inner);
        wrapper.set_state_weight(new_weight, current_block);
        MinerLedgers::<T>::insert(from, target, inner);
    }

    fn update_asset_mining_weight(target: &AssetId, current_block: T::BlockNumber) {
        let new_weight =
            <Self as ComputeMiningWeight<T::AccountId, T::BlockNumber>>::settle_claimee_weight(
                target,
                current_block,
            );
        Self::apply_update_asset_mining_weight(target, new_weight, current_block);
    }

    fn apply_update_asset_mining_weight(
        target: &AssetId,
        new_weight: WeightType,
        current_block: T::BlockNumber,
    ) {
        let mut inner = AssetLedgers::<T>::get(target);
        let mut wrapper = AssetLedgerWrapper::<T>::new(target, &mut inner);
        wrapper.set_state_weight(new_weight, current_block);
        AssetLedgers::<T>::insert(target, inner);
    }

    fn update_mining_weights(
        source: &T::AccountId,
        target: &AssetId,
        current_block: T::BlockNumber,
    ) {
        Self::update_miner_mining_weight(source, target, current_block);
        Self::update_asset_mining_weight(target, current_block);
    }

    fn issue_deposit_reward(depositor: &T::AccountId, target: &AssetId) -> DispatchResult {
        let deposit_reward = Self::deposit_reward();
        let reward_pot = T::DetermineRewardPotAccount::reward_pot_account_for(target);
        let reward_pot_balance = <T as xpallet_assets::Trait>::Currency::free_balance(&reward_pot);
        if reward_pot_balance >= deposit_reward {
            <T as xpallet_assets::Trait>::Currency::transfer(
                &reward_pot,
                depositor,
                deposit_reward,
                ExistenceRequirement::KeepAlive,
            )?;
        } else {
            warn!("asset {}'s reward pot has only {:?}, skipped issuing deposit reward for depositor {:?}", target, reward_pot_balance, depositor);
        }
        Ok(())
    }
}
