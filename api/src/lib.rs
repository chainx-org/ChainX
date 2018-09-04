// Copyright 2018 chainpool.

extern crate substrate_runtime_primitives as runtime_primitives;
extern crate substrate_executor as substrate_executor;
extern crate substrate_state_machine as state_machine;
extern crate substrate_runtime_io as runtime_io;
extern crate substrate_client_db as client_db;
extern crate substrate_runtime_executive;
extern crate substrate_client as client;
extern crate substrate_codec as codec;
extern crate substrate_primitives;

extern crate chainx_primitives as primitives;
extern crate chainx_runtime as runtime;
extern crate chainx_executor;

#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate log;

use primitives::{
	AccountId, Block, BlockId, Hash, Index, SessionKey, Timestamp,
	UncheckedExtrinsic, InherentData, Header,
};
use client::block_builder::BlockBuilder as ClientBlockBuilder;
use substrate_primitives::{KeccakHasher, RlpCodec};
use chainx_executor::NativeExecutor;
use runtime::Address;

mod implement;

error_chain! {
	errors {
		/// Unknown runtime code.
		UnknownRuntime {
			description("Unknown runtime code")
			display("Unknown runtime code")
		}
		/// Unknown block ID.
		UnknownBlock(b: String) {
			description("Unknown block")
			display("Unknown block {}", b)
		}
		/// Execution error.
		Execution(e: String) {
			description("Execution error")
			display("Execution error: {}", e)
		}
		/// Some other error.
		// TODO: allow to be specified as associated type of ChainXApi
		Other(e: Box<::std::error::Error + Send>) {
			description("Other error")
			display("Other error: {}", e.description())
		}
	}
}

impl From<client::error::Error> for Error {
	fn from(e: client::error::Error) -> Error {
		match e {
			client::error::Error(client::error::ErrorKind::UnknownBlock(b), _) => Error::from_kind(ErrorKind::UnknownBlock(b)),
			client::error::Error(client::error::ErrorKind::Execution(e), _) =>
				Error::from_kind(ErrorKind::Execution(format!("{}", e))),
			other => Error::from_kind(ErrorKind::Other(Box::new(other) as Box<_>)),
		}
	}
}

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
pub type TClientBlockBuilder = ClientBlockBuilder<TBackend, TExecutor, Block, KeccakHasher, RlpCodec>;


/// Trait encapsulating the ChainX API.
///
/// All calls should fail when the exact runtime is unknown.
pub trait ChainXApi {
	/// The block builder for this API type.
	type BlockBuilder: BlockBuilder;

	/// Get session keys at a given block.
	fn session_keys(&self, at: &BlockId) -> Result<Vec<SessionKey>>;

	/// Get validators at a given block.
	fn validators(&self, at: &BlockId) -> Result<Vec<AccountId>>;

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
