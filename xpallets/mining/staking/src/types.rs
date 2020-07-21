use super::*;
use chainx_primitives::AssetId;
use codec::{Decode, Encode};
use sp_runtime::RuntimeDebug;
#[cfg(feature = "std")]
use sp_runtime::{Deserialize, Serialize};
use xp_mining_common::WeightType;
use xp_mining_staking::MiningPower;

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
pub struct BondRequirement<Balance> {
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
pub struct Unbonded<Balance, BlockNumber> {
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
    /// Last calculated total vote weight of current nominator.
    pub last_vote_weight: WeightType,
    /// Block number at which point `last_vote_weight` just updated.
    pub last_vote_weight_update: BlockNumber,
}

/// Profile of staking validator.
///
/// These fields are static or updated less frequently.
#[derive(PartialEq, Eq, Clone, Default, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct ValidatorProfile<BlockNumber> {
    /// Block number at which point it's registered on chain.
    pub registered_at: BlockNumber,
    /// Validator is chilled right now.
    ///
    /// Declared no desire to be a validator or forced to be chilled due to `MinimumCandidateThreshold`.
    pub is_chilled: bool,
    /// Block number of last performed `chill` operation.
    pub last_chilled: Option<BlockNumber>,
}

/// Profile of staking nominator.
#[derive(PartialEq, Eq, Clone, Default, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct NominatorProfile<Balance, BlockNumber> {
    /// Block number of last `rebond` operation.
    pub last_rebond: Option<BlockNumber>,
    /// Total unbonded entries.
    pub unbonded_chunks: Vec<Unbonded<Balance, BlockNumber>>,
}

#[derive(Eq, PartialEq, Encode, Decode, Default, RuntimeDebug)]
// #[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
// #[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct RpcBalance<Balance> {
    // #[cfg_attr(
    // feature = "std",
    // serde(
    // bound(serialize = "Balance: std::fmt::Display"),
    // serialize_with = "serialize_as_string",
    // bound(deserialize = "Balance: std::str::FromStr"),
    // deserialize_with = "deserialize_from_string"
    // )
    // )]
    pub inner: Balance,
}

#[cfg(feature = "std")]
impl<Balance: std::fmt::Display> std::fmt::Display for RpcBalance<Balance> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.inner)
    }
}

#[cfg(feature = "std")]
impl<Balance: std::str::FromStr> std::str::FromStr for RpcBalance<Balance> {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let inner = s.parse::<Balance>().map_err(|_| "Parse Balance failed")?;
        Ok(Self { inner })
    }
}

impl<Balance> From<Balance> for RpcBalance<Balance> {
    fn from(inner: Balance) -> Self {
        Self { inner }
    }
}

// #[cfg(feature = "std")]
// fn serialize_as_string<S: serde::Serializer, T: std::fmt::Display>(
// t: &T,
// serializer: S,
// ) -> Result<S::Ok, S::Error> {
// serializer.serialize_str(&t.to_string())
// }

// #[cfg(feature = "std")]
// fn deserialize_from_string<'de, D: serde::Deserializer<'de>, T: std::str::FromStr>(
// deserializer: D,
// ) -> Result<T, D::Error> {
// let s = String::deserialize(deserializer)?;
// s.parse::<T>()
// .map_err(|_| serde::de::Error::custom("Parse from String failed"))
// }

#[cfg(feature = "std")]
impl<Balance: std::fmt::Display> serde::ser::Serialize for RpcBalance<Balance> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        self.inner.to_string().serialize(serializer)
    }
}

#[cfg(feature = "std")]
impl<'de, Balance: std::str::FromStr> serde::de::Deserialize<'de> for RpcBalance<Balance> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
        // Balance: std::str::FromStr,
        // <Balance as std::str::FromStr>::Err: std::fmt::Display,
    {
        let a = String::deserialize(deserializer)?;
        // let inner = a.parse::<Balance>().map_err(serde::de::Error::custom)?;
        let inner = a
            .parse::<Balance>()
            .map_err(|_| serde::de::Error::custom("Parse Balance from String failed"))?;
        Ok(Self { inner })
    }
}

/// Total information about a validator.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct ValidatorInfo<AccountId, RpcBalance, BlockNumber> {
    /// AccountId of this (potential) validator.
    pub account: AccountId,
    pub dummy: BlockNumber,
    // #[cfg_attr(feature = "std", serde(flatten))]
    // pub profile: ValidatorProfile<BlockNumber>,
    // #[cfg_attr(feature = "std", serde(flatten))]
    // pub ledger: ValidatorLedger<Balance, BlockNumber>,
    /// Being a validator, responsible for authoring the new blocks.
    pub is_validating: bool,
    /// How much balances the validator has bonded itself.
    pub self_bonded: RpcBalance,
    /// AccountId for the reward pot of this validator.
    pub reward_pot_account: AccountId,
    /// Balance of the reward pot account.
    pub reward_pot_balance: RpcBalance,
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

/// Top level shares of various reward destinations.
#[derive(Copy, Clone, PartialEq, Eq, Default, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct GlobalDistribution {
    pub treasury: u32,
    pub mining: u32,
}

impl GlobalDistribution {
    /// Calculates the rewards for treasury and mining accordingly.
    pub fn calc_rewards<T: Trait>(&self, reward: T::Balance) -> (T::Balance, T::Balance) {
        assert!(self.treasury + self.mining > 0);
        let treasury_reward = reward * self.treasury.saturated_into()
            / (self.treasury + self.mining).saturated_into();
        (treasury_reward, reward - treasury_reward)
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Default, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct MiningDistribution {
    pub asset: u32,
    pub staking: u32,
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
            crate::Module::<T>::total_staked().saturated_into::<MiningPower>();
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
                "asset_mining_shares is ensured to be positive in set_distribution_ratio()"
            );
            // There could be some computation loss here, but it's ok.
            let treasury_extra = (m2 - m1) * asset_mining_reward_cap.saturated_into::<u128>() / m2;
            Some(treasury_extra.saturated_into::<T::Balance>())
        }
    }
}

/// Struct for performing the slash.
///
/// Abstracted for caching the treasury account.
#[derive(Copy, Clone, PartialEq, Eq, Default, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct Slasher<T: Trait>(T::AccountId);

impl<T: Trait> Slasher<T> {
    pub fn new(treasury_account: T::AccountId) -> Self {
        Self(treasury_account)
    }

    /// Returns Ok(_) if the reward pot of offender has enough balance to cover the slashing,
    /// otherwise slash the reward pot as much as possible and returns the value actually slashed.
    pub fn try_slash(
        &self,
        offender: &T::AccountId,
        expected_slash: T::Balance,
    ) -> Result<(), T::Balance> {
        let reward_pot = T::DetermineRewardPotAccount::reward_pot_account_for(offender);
        let reward_pot_balance = <xpallet_assets::Module<T>>::pcx_free_balance(&reward_pot);

        if expected_slash <= reward_pot_balance {
            self.apply_slash(&reward_pot, expected_slash);
            Ok(())
        } else {
            self.apply_slash(&reward_pot, reward_pot_balance);
            Err(reward_pot_balance)
        }
    }

    /// Actually slash the account being punished, all slashed balance will go to the treasury.
    fn apply_slash(&self, reward_pot: &T::AccountId, value: T::Balance) {
        let _ = <xpallet_assets::Module<T>>::pcx_move_free_balance(reward_pot, &self.0, value);
    }
}
