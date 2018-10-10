// Copyright 2018 chainpool.

extern crate sr_primitives as runtime_primitives;
extern crate substrate_executor as substrate_executor;
extern crate substrate_client_db as client_db;
extern crate substrate_client as client;
extern crate parity_codec as codec;
extern crate substrate_primitives;
extern crate sr_io as runtime_io;
extern crate srml_executive;

extern crate chainx_primitives as primitives;
extern crate chainx_runtime as runtime;
extern crate chainx_executor;

use primitives::{
    AccountId, Block, BlockId, Hash, Index, SessionKey, Timestamp, BlockNumber,
    UncheckedExtrinsic, InherentData, Balance,
};
use runtime_primitives::{transaction_validity::TransactionValidity, traits::{CurrentHeight, BlockNumberToHash}};
use client::block_builder::BlockBuilder as ClientBlockBuilder;
pub use client::error::{Error, ErrorKind, Result};
use substrate_primitives::Blake2Hasher;
use chainx_executor::NativeExecutor;
use runtime::Address;

mod implement;

/// Build new blocks.
pub trait BlockBuilder {
    /// Push an extrinsic onto the block. Fails if the extrinsic is invalid.
    fn push_extrinsic(&mut self, extrinsic: UncheckedExtrinsic) -> Result<()>;

    /// Bake the block with provided extrinsics.
    fn bake(self) -> Result<Block>;
}

pub type TBackend = client_db::Backend<Block>;
pub type TExecutor = client::LocalCallExecutor<TBackend, NativeExecutor<chainx_executor::Executor>>;
pub type TClient = client::Client<TBackend, TExecutor, Block>;
pub type TClientBlockBuilder = ClientBlockBuilder<TBackend, TExecutor, Block, Blake2Hasher>;


/// Trait encapsulating the ChainX API.
///
/// All calls should fail when the exact runtime is unknown.
pub trait ChainXApi: CurrentHeight<BlockNumber=BlockNumber> + BlockNumberToHash<BlockNumber=BlockNumber, Hash=Hash> {
    /// The block builder for this API type.
    type BlockBuilder: BlockBuilder;

    /// Get session keys at a given block.
    fn session_keys(&self, at: &BlockId) -> Result<Vec<SessionKey>>;

    /// Get validators at a given block.
    fn validators(&self, at: &BlockId) -> Result<Vec<AccountId>>;

    /// Get a validator stake weight at a given block.
    fn stake_weight(&self, at: &BlockId, account: AccountId) -> Result<Balance>;

    /// Get the value of the randomness beacon at a given block.
    fn random_seed(&self, at: &BlockId) -> Result<Hash>;

    /// Get the timestamp registered at a block.
    fn timestamp(&self, at: &BlockId) -> Result<Timestamp>;

    /// Get the nonce (né index) of an account at a block.
    fn index(&self, at: &BlockId, account: AccountId) -> Result<Index>;

    /// Get the account id of an address at a block.
    fn lookup(&self, at: &BlockId, address: Address) -> Result<Option<AccountId>>;

    /// Evaluate a block. Returns true if the block is good, false if it is known to be bad,
    /// and an error if we can't evaluate for some reason.
    fn evaluate_block(&self, at: &BlockId, block: Block) -> Result<bool>;

    fn validate_transaction(&self, at: &BlockId, transaction: UncheckedExtrinsic) -> Result<TransactionValidity>;

    /// Build a block on top of the given, with inherent extrinsics pre-pushed.
    fn build_block(&self, at: &BlockId, inherent_data: InherentData) -> Result<Self::BlockBuilder>;

    /// Attempt to produce the (encoded) inherent extrinsics for a block being built upon the given.
    /// This may vary by runtime and will fail if a runtime doesn't follow the same API.
    fn inherent_extrinsics(&self, at: &BlockId, inherent_data: InherentData) -> Result<Vec<UncheckedExtrinsic>>;
}

/// Mark for all ChainX API implementations, that are making use of state data, stored locally.
pub trait LocalChainXApi: ChainXApi {}

/// Mark for all ChainX API implementations, that are fetching required state data from remote nodes.
pub trait RemoteChainXApi: ChainXApi {}
