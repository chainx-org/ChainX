#![cfg_attr(not(feature = "std"), no_std)]

//! Common concepts with regard to the ChainX Mining system, particularly the user-level ones.
//!
//! There are two approaches of mining in ChainX:
//!
//! 1. As a PoS-based blockchain, **Staking** is inherently the fundamental way of mining.
//! In this way, users(stakers) nominate some validators with some balances locked, earning
//! the staking reward.
//!
//! 2. One goal of ChainX is to embrace more the eixsting cryptocurrencies into one ecosystem,
//! therefore **Asset Mining** is introduced for winning more external assets like BTC, ETH, etc.
//! For example, Bitcoin users can deposit their BTC into ChainX, then they'll get the X_BTC
//! in 1:1 and the mining rights in ChainX system accordingly, earning the new minted PCX
//! like the stakers in Staking.
//!
//! Both of these two approaches share one same rule when calculating the individual reward, i.e.,
//! **time-sensitive weight calculation**.
//!
//! ```
//! Amount(Balance) * Duration(BlockNumber) = Weight
//! ```
//!
//! For Staking:
//!
//! ```
//! staked_balance(Balance) * time(BlockNumber) = vote_weight
//! ```
//!
//! All the nominators split the reward of the validator's jackpot according to the proportion of vote weight.
//!
//! For Asset Mining:
//!
//! ```
//! ext_asset_balance(Balance) * time(BlockNumber) = ext_mining_weight
//! ```
//!
//! All asset miners split the reward of asset's jackpot according to the proportion of asset mining weight.
//!

use sp_arithmetic::traits::{BaseArithmetic, SaturatedConversion};
use sp_std::result::Result;

/// Type for calculating the mining weight.
pub type WeightType = u128;

/// The getter and setter methods for the further mining weight processing.
pub trait BaseMiningWeight<Balance, BlockNumber> {
    fn amount(&self) -> Balance;
    /// Set the new amount.
    ///
    /// Amount management of asset miners is handled by assets module,
    /// hence the default implementation is provided here.
    fn set_amount(&mut self, _new: Balance) {}

    fn last_acum_weight(&self) -> WeightType;
    fn set_last_acum_weight(&mut self, s: WeightType);

    fn last_acum_weight_update(&self) -> BlockNumber;
    fn set_last_acum_weight_update(&mut self, num: BlockNumber);
}

/// Amount changes of miner's state.
///
/// `Zero` happens:
/// 1. stakers performs the `rebond` operation.
/// 2. claim the reward.
#[derive(Clone, Copy, sp_runtime::RuntimeDebug)]
pub enum Delta<Balance> {
    Add(Balance),
    Sub(Balance),
    Zero,
}

/// General logic for state changes of the mining weight operations.
pub trait MiningWeight<Balance: BaseArithmetic + Copy, BlockNumber>:
    BaseMiningWeight<Balance, BlockNumber>
{
    /// Set the new amount after settling the change of nomination.
    fn settle_and_set_amount(&mut self, delta: &Delta<Balance>) {
        let new = match *delta {
            Delta::Add(x) => self.amount() + x,
            Delta::Sub(x) => self.amount() - x,
            Delta::Zero => return,
        };
        self.set_amount(new);
    }

    /// This action doesn't involve in the change of amount.
    ///
    /// Used for asset mining module.
    fn set_state_weight(&mut self, latest_acum_weight: WeightType, current_block: BlockNumber) {
        self.set_last_acum_weight(latest_acum_weight);
        self.set_last_acum_weight_update(current_block);
    }

    /// Set new state on bond, unbond and rebond in Staking.
    fn set_state(
        &mut self,
        latest_acum_weight: WeightType,
        current_block: BlockNumber,
        delta: &Delta<Balance>,
    ) {
        self.set_state_weight(latest_acum_weight, current_block);
        self.settle_and_set_amount(delta);
    }
}

impl<Balance: BaseArithmetic + Copy, BlockNumber, T: BaseMiningWeight<Balance, BlockNumber>>
    MiningWeight<Balance, BlockNumber> for T
{
}

/// Skips the next processing when the latest mining weight is zero.
pub struct ZeroMiningWeightError;

/// General logic for calculating the latest mining weight.
pub trait ComputeMiningWeight<AccountId, BlockNumber: Copy> {
    /// The entity that holds the funds of claimers.
    type Claimee;
    type Error: From<ZeroMiningWeightError>;

    fn claimer_weight_factors(_: &AccountId, _: &Self::Claimee, _: BlockNumber) -> WeightFactors;
    fn claimee_weight_factors(_: &Self::Claimee, _: BlockNumber) -> WeightFactors;

    fn settle_claimer_weight(
        who: &AccountId,
        target: &Self::Claimee,
        current_block: BlockNumber,
    ) -> WeightType {
        Self::_calc_latest_vote_weight(Self::claimer_weight_factors(who, target, current_block))
    }

    fn settle_claimee_weight(target: &Self::Claimee, current_block: BlockNumber) -> WeightType {
        Self::_calc_latest_vote_weight(Self::claimee_weight_factors(target, current_block))
    }

    fn settle_weight_on_claim(
        who: &AccountId,
        target: &Self::Claimee,
        current_block: BlockNumber,
    ) -> Result<(WeightType, WeightType), Self::Error> {
        let claimer_weight = Self::settle_claimer_weight(who, target, current_block);

        if claimer_weight == 0 {
            return Err(ZeroMiningWeightError.into());
        }

        let claimee_weight = Self::settle_claimee_weight(target, current_block);

        Ok((claimer_weight, claimee_weight))
    }

    fn _calc_latest_vote_weight(weight_factors: WeightFactors) -> WeightType {
        let (last_acum_weight, amount, duration) = weight_factors;
        last_acum_weight + WeightType::from(amount) * WeightType::from(duration)
    }
}

/// Weight Formula:
///
/// LatestVoteWeight(WeightType) = last_acum_weight(WeightType) + amount(Balance) * duration(BlockNumber)
///
/// Using u128 for calculating the weights won't run into the overflow issue practically.
pub type WeightFactors = (WeightType, u128, u128);

/// Prepares the factors for calculating the latest mining weight.
pub fn generic_weight_factors<
    Balance: BaseArithmetic,
    BlockNumber: BaseArithmetic,
    W: BaseMiningWeight<Balance, BlockNumber>,
>(
    w: W,
    current_block: BlockNumber,
) -> WeightFactors {
    (
        w.last_acum_weight(),
        w.amount().saturated_into(),
        (current_block - w.last_acum_weight_update()).saturated_into(),
    )
}

/// Claims the reward for participating in the mining.
pub trait Claim<AccountId> {
    /// Entity of holder of individual miners.
    ///
    /// Validator for Staking, Asset for Asset Mining.
    type Claimee;
    /// Claim error type.
    type Error;

    fn claim(claimer: &AccountId, claimee: &Self::Claimee) -> Result<(), Self::Error>;
}
