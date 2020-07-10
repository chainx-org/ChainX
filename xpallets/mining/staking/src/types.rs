use super::*;
use chainx_primitives::AssetId;
use codec::{Decode, Encode};
use sp_runtime::RuntimeDebug;
#[cfg(feature = "std")]
use sp_runtime::{Deserialize, Serialize};
use xp_mining_common::WeightType;

/// Destination for minted fresh PCX on each new session.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub enum MintedDestination<AccountId> {
    Validator(AccountId),
    Asset(AssetId),
}

/// The requirement of a qualified staking candidate.
///
/// If the (potential) validator failed to meet this requirement, force it to be chilled on new election round.
#[derive(PartialEq, Eq, Clone, Default, Encode, Decode, RuntimeDebug)]
pub struct BondRequirement<Balance: Default> {
    /// The minimal amount of self-bonded balance to be a qualified validator candidate.
    pub self_bonded: Balance,
    /// The minimal amount of total-bonded balance to be a qualified validator candidate.
    ///
    /// total-bonded = self-bonded + all the other nominators' nominations.
    pub total: Balance,
}

/// Type for noting when the unbonded fund can be withdrawn.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct Unbonded<Balance: Default, BlockNumber: Default> {
    /// Amount of funds to be unlocked.
    pub value: Balance,
    /// Block number at which point it'll be unlocked.
    pub locked_until: BlockNumber,
}

/// Vote weight properties of validator.
#[derive(PartialEq, Eq, Clone, Default, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct ValidatorLedger<Balance, BlockNumber> {
    /// The total amount of all the nominators' vote balances.
    pub total: Balance,
    /// Last calculated total vote weight of current validator.
    pub last_total_vote_weight: WeightType,
    /// Block number at which point `last_total_vote_weight` just updated.
    pub last_total_vote_weight_update: BlockNumber,
}

/// Vote weight properties of nominator.
#[derive(PartialEq, Eq, Clone, Default, Encode, Decode, RuntimeDebug)]
pub struct NominatorLedger<Balance, BlockNumber> {
    /// The amount of
    pub nomination: Balance,
    ///
    pub last_vote_weight: WeightType,
    ///
    pub last_vote_weight_update: BlockNumber,
}

/// Profile of staking validator.
///
/// These fields are static or updated less frequently.
#[derive(PartialEq, Eq, Clone, Default, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct ValidatorProfile<BlockNumber: Default> {
    /// Block number at which point it's registered on chain.
    pub registered_at: BlockNumber,
    /// Validator is chilled right now.
    pub is_chilled: bool,
    /// Block number of last performed `chill` operation.
    pub last_chilled: Option<BlockNumber>,
}

/// Profile of staking nominator.
#[derive(PartialEq, Eq, Clone, Default, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct NominatorProfile<Balance: Default, BlockNumber: Default> {
    /// Block number of last `rebond` operation.
    pub last_rebond: Option<BlockNumber>,
    ///
    pub unbonded_chunks: Vec<Unbonded<Balance, BlockNumber>>,
}

/// Status of (potential) validator in staking module.
///
/// For RPC usage.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub enum ValidatorStatus {
    /// Declared no desire to be a validator or forced to be chilled due to `MinimumCandidateThreshold`.
    Chilled,
    /// Declared desire to be a validator but haven't won one place.
    Candidate,
    /// Being a validator, responsible for authoring the new blocks.
    Validating,
}

impl Default for ValidatorStatus {
    fn default() -> Self {
        Self::Candidate
    }
}

#[derive(PartialEq, Eq, Clone, Default, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct ValidatorInfo<AccountId: Default, Balance: Default, BlockNumber: Default> {
    pub account: AccountId,
    #[cfg_attr(feature = "std", serde(flatten))]
    pub profile: ValidatorProfile<BlockNumber>,
    #[cfg_attr(feature = "std", serde(flatten))]
    pub ledger: ValidatorLedger<Balance, BlockNumber>,
    pub status: ValidatorStatus,
    pub self_bonded: Balance,
    pub reward_pot_account: AccountId,
    pub reward_pot_balance: Balance,
}

/// Information regarding the active era (era in used in session).
#[derive(Encode, Decode, RuntimeDebug)]
pub struct ActiveEraInfo {
    /// Index of era.
    pub index: EraIndex,
    /// Moment of start expressed as millisecond from `$UNIX_EPOCH`.
    ///
    /// Start can be none if start hasn't been set for the era yet,
    /// Start is set on the first on_finalize of the era to guarantee usage of `Time`.
    pub start: Option<u64>,
}

/// Mode of era-forcing.
#[derive(Copy, Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum Forcing {
    /// Not forcing anything - just let whatever happen.
    NotForcing,
    /// Force a new era, then reset to `NotForcing` as soon as it is done.
    ForceNew,
    /// Avoid a new era indefinitely.
    ForceNone,
    /// Force a new era at the end of all sessions indefinitely.
    ForceAlways,
}

impl Default for Forcing {
    fn default() -> Self {
        Forcing::NotForcing
    }
}

// Shares of various reward destinations.
#[derive(Copy, Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct GlobalDistribution {
    pub treasury: u32,
    pub mining: u32,
}

impl Default for GlobalDistribution {
    /// According to the ChainX 1.0 referendum proposal09:
    /// (Treasury, Airdrop Asset, X-type Asset and Staking) = (12, 8, 80)
    ///
    /// Airdrop Assets have been retired in ChainX 2.0, now only treasury and mining destinations.
    /// (Treasury, X-type Asset and Staking) = (12, 88)
    fn default() -> Self {
        Self {
            treasury: 12u32,
            mining: 88u32,
        }
    }
}

impl GlobalDistribution {
    pub fn calc_rewards<T: Trait>(&self, reward: T::Balance) -> (T::Balance, T::Balance) {
        let treasury_reward = reward * self.treasury.saturated_into()
            / (self.treasury + self.mining).saturated_into();
        (treasury_reward, reward - treasury_reward)
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct MiningDistribution {
    pub asset: u32,
    pub staking: u32,
}

impl Default for MiningDistribution {
    /// According to the ChainX 1.0 referendum proposal09,
    /// (Asset Mining, Staking) = (10, 90)
    fn default() -> Self {
        Self {
            asset: 10u32,
            staking: 90u32,
        }
    }
}

impl MiningDistribution {
    /// Returns the reward for Staking given the total reward according to the Staking proportion.
    pub fn calc_staking_reward<T: Trait>(&self, reward: T::Balance) -> T::Balance {
        reward.saturating_mul(self.staking.saturated_into())
            / (self.asset + self.staking).saturated_into()
    }

    /// Return a tuple (m1, m2) for comparing whether asset_mining_power are reaching the upper limit.
    ///
    /// If m1 >= m2, the asset mining cap has reached, all the reward calculated by the shares go to
    /// the mining assets, but its unit mining power starts to decrease compared to the inital FixedPower.
    fn asset_mining_vs_staking<T: Trait>(&self) -> (u128, u128) {
        let total_staking_power =
            crate::Module::<T>::total_staked().saturated_into::<xp_mining_staking::MiningPower>();
        let total_asset_mining_power = T::AssetMining::total_asset_mining_power();

        // When:
        //
        //  total_asset_mining_power     1(asset_mining_shares)
        //  ------------------------ >= -----------------------
        //     total_staking_power         9(staking_shares)
        //
        //  i.e., m1 >= m2,
        //
        // there is no extra treasury split, otherwise the difference will
        // be distruted to the treasury account again.
        let m1 = total_asset_mining_power * u128::from(self.staking);
        let m2 = total_staking_power * u128::from(self.asset);

        (m1, m2)
    }

    pub fn has_treasury_extra<T: Trait>(
        &self,
        asset_mining_reward_cap: T::Balance,
    ) -> Option<T::Balance> {
        let (m1, m2) = self.asset_mining_vs_staking::<T>();
        if m1 >= m2 {
            debug!(
                "[has_treasury_extra] m1({}) >= m2({}), no extra treasury split.",
                m1, m2
            );
            None
        } else {
            assert!(
                m2 > 0,
                "cross_mining_shares is ensured to be positive in set_distribution_ratio()"
            );
            // There could be some computation loss here, but it's ok.
            let treasury_extra = (m2 - m1) * asset_mining_reward_cap.saturated_into::<u128>() / m2;
            Some(treasury_extra.saturated_into::<T::Balance>())
        }
    }
}
