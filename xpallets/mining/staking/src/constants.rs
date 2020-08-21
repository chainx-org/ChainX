use frame_support::traits::LockIdentifier;

pub const STAKING_ID: LockIdentifier = *b"staking ";

/// Session reward of the first 210_000 sessions.
///
/// ChainX uses a Bitcoin like issuance model, the initial reward is 50 PCX.
pub const INITIAL_REWARD: u64 = 5_000_000_000;

/// Every 210_000 sessions, the session reward is cut in half.
///
/// ChainX follows the issuance rule of Bitcoin. The `Session` in ChainX
/// is equivalent to `Block` in Bitcoin with regard to minting new coins.
pub const SESSIONS_PER_ROUND: u32 = 210_000;

/// The maximum number of Staking validators.
///
/// Currently the election will perform a naive sort on the all candidates,
/// so we don't want the candidate list too huge.
pub const DEFAULT_MAXIMUM_VALIDATOR_COUNT: u32 = 1000;

/// The maximum number of ongoing unbonded operations in parallel.
pub const DEFAULT_MAXIMUM_UNBONDED_CHUNK_SIZE: u32 = 10;

/// ChainX 2.0's block time is targeted at 6s, i.e., 5 minutes per session.
///
/// ChainX 1.0 is 2s/block, 150 blocks/session, the duration of each session is also
/// 5 minutes, therefore the issuance rate stays the same in terms of the time dimension, the daily Staking earnings does not change.
pub const DEFAULT_BLOCKS_PER_SESSION: u64 = 50;

/// The default bonding duration for regular staker is 3 days.
///
/// The staker can unbond the staked balances, but these balances will be free immediately, they have to wait for 3 days to withdraw them into the free balances.
pub const DEFAULT_BONDING_DURATION: u64 = DEFAULT_BLOCKS_PER_SESSION * 12 * 24 * 3;

/// The default bonding duration for validator is 3 * 10 days.
pub const DEFAULT_VALIDATOR_BONDING_DURATION: u64 = DEFAULT_BONDING_DURATION * 10;

/// The number of unfinished sessions in the first halving epoch.
///
/// When the ChainX 2.0 migration happens, the first halving epoch is not over yet.
/// FIXME: replace this when the migration offset is determinated.
pub const MIGRATION_SESSION_OFFSET: u32 = 500;
