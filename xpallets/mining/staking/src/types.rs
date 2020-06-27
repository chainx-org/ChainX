use chainx_primitives::AssetId;
use codec::{Decode, Encode};
use sp_runtime::RuntimeDebug;
use xp_staking::VoteWeight;

/// Destination for minted fresh PCX on each new session.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub enum MintedDestination<AccountId> {
    Validator(AccountId),
    Asset(AssetId),
}

/// Status of (potential) validator in staking module.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub enum ValidatorStatus {
    /// Declared no desire to be a validator.
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

/// Type for noting when the unbonded fund can be withdrawn.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct Unbonded<Balance, BlockNumber> {
    /// Amount of funds to be unlocked.
    pub value: Balance,
    /// Block number at which point it'll be unlocked.
    pub locked_until: BlockNumber,
}

/// Vote weight properties of validator.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct ValidatorLedger<Balance, BlockNumber> {
    /// The total amount of all the nominators' vote balances.
    pub total: Balance,
    /// Last calculated total vote weight of current validator.
    pub last_total_vote_weight: VoteWeight,
    ///
    pub last_total_vote_weight_update: BlockNumber,
}

/// Vote weight properties of nominator.
#[derive(PartialEq, Eq, Clone, Default, Encode, Decode, RuntimeDebug)]
pub struct NominatorLedger<Balance, BlockNumber> {
    /// The amount of
    pub value: Balance,
    ///
    pub last_vote_weight: VoteWeight,
    ///
    pub last_vote_weight_update: BlockNumber,
}

/// Profile of staking validator.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct ValidatorProfile<BlockNumber> {
    /// Block number at which point it's registered on chain.
    pub registered_at: BlockNumber,
    /// Block number of last performed `chilled` operation.
    pub last_chilled: Option<BlockNumber>,
}

///
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct NominatorProfile<BlockNumber> {
    pub unbonded: Vec<BlockNumber>,
}
