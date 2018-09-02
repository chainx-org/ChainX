// Copyright 2018 Chainpool.
extern crate substrate_bft as bft;
extern crate substrate_codec as codec;
extern crate substrate_primitives as primitives;
extern crate substrate_runtime_support as runtime_support;
extern crate substrate_runtime_primitives as runtime_primitives;
extern crate substrate_client as client;
extern crate substrate_network;
extern crate substrate_extrinsic_pool;

extern crate chainx_runtime;
extern crate chainx_primitives;
extern crate chainx_api;
extern crate chainx_pool;

extern crate exit_future;
extern crate tokio;
extern crate rhododendron;
#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate futures;
#[macro_use]
extern crate log;
extern crate ed25519;
extern crate parking_lot;

mod evaluation;
mod error;
mod offline_tracker;
mod service;
mod dynamic_inclusion;

use std::sync::Arc;
use std::time::{Duration, Instant};

use codec::{Decode, Encode};
use primitives::AuthorityId;
use tokio::runtime::TaskExecutor;
use tokio::timer::{Delay, Interval};
use chainx_primitives::{CandidateReceipt, BlockId, Hash, Block, Header, AccountId, BlockNumber, Timestamp, SessionKey};
use chainx_api::ChainXApi;

use futures::prelude::*;
use futures::future;
use parking_lot::RwLock;

pub use self::error::{ErrorKind, Error};
pub use self::offline_tracker::OfflineTracker;
pub use service::Service;
use dynamic_inclusion::DynamicInclusion;

/// Shared offline validator tracker.
pub type SharedOfflineTracker = Arc<RwLock<OfflineTracker>>;
pub type TransactionPool = substrate_extrinsic_pool::Pool<chainx_pool::PoolApi>;

// block size limit.
const MAX_TRANSACTIONS_SIZE: usize = 4 * 1024 * 1024;

/// A handle to a statement table router.
///
/// This is expected to be a lightweight, shared type like an `Arc`.
pub trait TableRouter: Clone {
	/// Errors when fetching data from the network.
	type Error;
}

/// A long-lived network which can create BFT message routing processes on demand.
pub trait Network {
	/// The table router type. This should handle importing of any statements,
	/// routing statements to peers, and driving completion of any `StatementProducers`.
	type TableRouter: TableRouter;
	/// The input stream of BFT messages. Should never logically conclude.
	type Input: Stream<Item=bft::Communication<Block>,Error=Error>;
	/// The output sink of BFT messages. Messages sent here should eventually pass to all
	/// current authorities.
	type Output: Sink<SinkItem=bft::Communication<Block>,SinkError=Error>;

	/// Instantiate a table router using the given task executor.
	fn communication_for(&self, validators: &[SessionKey], task_executor: TaskExecutor) -> (Self::TableRouter, Self::Input, Self::Output);
}

/// ChainX proposer factory.
pub struct ProposerFactory<N, P> 
	where
		P: ChainXApi + Send + Sync + 'static
{
	/// The client instance.
	pub client: Arc<P>,
    /// transaction pool,
    pub transaction_pool: Arc<TransactionPool>,
	/// The backing network handle.
	pub network: N,
	/// handle to remote task executor
	pub handle: TaskExecutor,
	/// Offline-tracker.
	pub offline: SharedOfflineTracker,
}

impl<N, P> bft::Environment<Block> for ProposerFactory<N, P>
	where
		N: Network,
		P: ChainXApi + Send + Sync + 'static,
		N::TableRouter: Send + 'static,
{
	type Proposer = Proposer<P>;
	type Input = N::Input;
	type Output = N::Output;
	type Error = Error;

	fn init(
		&self,
		parent_header: &Header,
		authorities: &[AuthorityId],
		_sign_with: Arc<ed25519::Pair>,
	) -> Result<(Self::Proposer, Self::Input, Self::Output), Error> {
		use runtime_primitives::traits::{Hash as HashT, BlakeTwo256};

		let parent_hash = parent_header.hash().into();

		let id = BlockId::hash(parent_hash);
		let random_seed = self.client.random_seed(&id)?;
		let random_seed = BlakeTwo256::hash(&*random_seed);

		let validators = self.client.validators(&id)?;
		self.offline.write().note_new_block(&validators[..]);

		let (_router, input, output) = self.network.communication_for(
			authorities,
			self.handle.clone()
		);
        let now = Instant::now();
        let dynamic_inclusion = DynamicInclusion::new(
            10,
            now,
            Duration::from_millis(4000),
        );

		let proposer = Proposer {
			client: self.client.clone(),
            dynamic_inclusion, 
			//local_key: sign_with,
			parent_hash,
			parent_id: id,
			parent_number: parent_header.number,
			random_seed,
            transaction_pool: self.transaction_pool.clone(),
			offline: self.offline.clone(),
			validators,
		};

		Ok((proposer, input, output))
	}
}

/// The ChainX proposer logic.
pub struct Proposer<C: ChainXApi + Send + Sync> {
	client: Arc<C>,
    dynamic_inclusion: DynamicInclusion,
	//local_key: Arc<ed25519::Pair>,
	parent_hash: Hash,
	parent_id: BlockId,
	parent_number: BlockNumber,
	random_seed: Hash,
    transaction_pool: Arc<TransactionPool>,
	offline: SharedOfflineTracker,
	validators: Vec<AccountId>,
}

impl<C: ChainXApi + Send + Sync> Proposer<C> {
	fn primary_index(&self, round_number: usize, len: usize) -> usize {
		use primitives::uint::U256;

		let big_len = U256::from(len);
		let offset = U256::from_big_endian(&self.random_seed.0) % big_len;
		let offset = offset.low_u64() as usize + round_number;
		offset % len
	}
}

impl<C> bft::Proposer<Block> for Proposer<C>
	where
		C: ChainXApi + Send + Sync,
{
	type Error = Error;
	type Create = future::Either<
		CreateProposal<C>,
		future::FutureResult<Block, Error>,
	>;
	type Evaluate = Box<Future<Item=bool, Error=Error>>;

	fn propose(&self) -> Self::Create {
		const ATTEMPT_PROPOSE_EVERY: Duration = Duration::from_millis(100);

        let now = Instant::now();
        let initial_included = 0;
        let enough_candidates = self.dynamic_inclusion.acceptable_in(
             now,
             initial_included,
        ).unwrap_or_else(|| now + Duration::from_millis(1));
		let timing = ProposalTiming {
			attempt_propose: Interval::new(now + ATTEMPT_PROPOSE_EVERY, ATTEMPT_PROPOSE_EVERY),
			enough_candidates: Delay::new(enough_candidates),
			dynamic_inclusion: self.dynamic_inclusion.clone(),
			last_included: initial_included,
		};

		future::Either::A(CreateProposal {
			parent_hash: self.parent_hash.clone(),
			parent_number: self.parent_number.clone(),
			parent_id: self.parent_id.clone(),
			client: self.client.clone(),
			transaction_pool: self.transaction_pool.clone(),
			offline: self.offline.clone(),
			validators: self.validators.clone(),
			timing,
		})
	}

	fn evaluate(&self, unchecked_proposal: &Block) -> Self::Evaluate {
		debug!(target: "bft", "evaluating block on top of parent ({}, {:?})", self.parent_number, self.parent_hash);

		let current_timestamp = current_timestamp();

		// do initial serialization and structural integrity checks.
		let maybe_proposal = evaluation::evaluate_initial(
			unchecked_proposal,
			current_timestamp,
			&self.parent_hash,
			self.parent_number,
		);

		let proposal = match maybe_proposal {
			Ok(p) => p,
			Err(e) => {
				// TODO: these errors are easily re-checked in runtime.
				debug!(target: "bft", "Invalid proposal: {:?}", e);
				return Box::new(future::ok(false));
			}
		};

		let vote_delays = {
			let now = Instant::now();

			// the duration until the given timestamp is current
			let proposed_timestamp = proposal.timestamp();
			let timestamp_delay = if proposed_timestamp > current_timestamp {
				Some(now + Duration::from_secs(proposed_timestamp - current_timestamp))
			} else {
				None
			};

			// delay casting vote until able according to minimum block time,
			// timestamp delay, and count delay.
			// construct a future from the maximum of the two durations.
			let max_delay = timestamp_delay;

			let temporary_delay = match max_delay {
				Some(duration) => future::Either::A(
					Delay::new(duration).map_err(|e| Error::from(ErrorKind::Timer(e)))
				),
				None => future::Either::B(future::ok(())),
			};

			temporary_delay
		};

		// refuse to vote if this block says a validator is offline that we
		// think isn't.
		let offline = proposal.noted_offline();
		if !self.offline.read().check_consistency(&self.validators[..], offline) {
			return Box::new(futures::empty());
		}

		// evaluate whether the block is actually valid.
		// TODO: is it better to delay this until the delays are finished?
		let evaluated = self.client
			.evaluate_block(&self.parent_id, unchecked_proposal.clone())
			.map_err(Into::into);

		let future = future::result(evaluated).and_then(move |good| {
			let end_result = future::ok(good);
			if good {
				// delay a "good" vote.
				future::Either::A(vote_delays.and_then(|_| end_result))
			} else {
				// don't delay a "bad" evaluation.
				future::Either::B(end_result)
			}
		});

		Box::new(future) as Box<_>
	}

	fn round_proposer(&self, round_number: usize, authorities: &[AuthorityId]) -> AuthorityId {
		let offset = self.primary_index(round_number, authorities.len());
		let proposer = authorities[offset].clone();
		trace!(target: "bft", "proposer for round {} is {}", round_number, proposer);

		proposer
	}

	fn import_misbehavior(&self, _misbehavior: Vec<(AuthorityId, bft::Misbehavior<Hash>)>) {
	}

	fn on_round_end(&self, round_number: usize, was_proposed: bool) {
		let primary_validator = self.validators[
			self.primary_index(round_number, self.validators.len())
		];


		// alter the message based on whether we think the empty proposer was forced to skip the round.
		// this is determined by checking if our local validator would have been forced to skip the round.
		let consider_online = was_proposed || {
			let forced_delay = self.dynamic_inclusion.acceptable_in(Instant::now(), 0);
			let public = ::ed25519::Public::from_raw(primary_validator.0);
			match forced_delay {
				None => info!(
					"Potential Offline Validator: {} failed to propose during assigned slot: {}",
					public,
					round_number,
				),
				Some(_) => info!(
					"Potential Offline Validator {} potentially forced to skip assigned slot: {}",
					public,
					round_number,
				),
			}

			forced_delay.is_some()
		};

		self.offline.write().note_round_end(primary_validator, consider_online);
	}
}

fn current_timestamp() -> Timestamp {
	use std::time;

	time::SystemTime::now().duration_since(time::UNIX_EPOCH)
		.expect("now always later than unix epoch; qed")
		.as_secs()
}

struct ProposalTiming {
	attempt_propose: Interval,
	dynamic_inclusion: DynamicInclusion,
	enough_candidates: Delay,
	last_included: usize,
}

impl ProposalTiming {
	// whether it's time to attempt a proposal.
	// shouldn't be called outside of the context of a task.
	fn poll(&mut self, included: usize) -> Poll<(), ErrorKind> {
		// first drain from the interval so when the minimum delay is up
		// we don't have any notifications built up.
		//
		// this interval is just meant to produce periodic task wakeups
		// that lead to the `dynamic_inclusion` getting updated as necessary.
		if let Async::Ready(x) = self.attempt_propose.poll().map_err(ErrorKind::Timer)? {
			x.expect("timer still alive; intervals never end; qed");
		}

		if included == self.last_included {
			return self.enough_candidates.poll().map_err(ErrorKind::Timer);
		}

		// the amount of includable candidates has changed. schedule a wakeup
		// if it's not sufficient anymore.
		match self.dynamic_inclusion.acceptable_in(Instant::now(), included) {
			Some(instant) => {
				self.last_included = included;
				self.enough_candidates.reset(instant);
				self.enough_candidates.poll().map_err(ErrorKind::Timer)
			}
			None => Ok(Async::Ready(())),
		}
	}
}

/// Future which resolves upon the creation of a proposal.
pub struct CreateProposal<C: ChainXApi + Send + Sync>  {
	parent_hash: Hash,
	parent_number: BlockNumber,
	parent_id: BlockId,
	client: Arc<C>,
	transaction_pool: Arc<TransactionPool>,
	timing: ProposalTiming,
	validators: Vec<AccountId>,
	offline: SharedOfflineTracker,
}

impl<C> CreateProposal<C> where C: ChainXApi + Send + Sync {
	fn propose_with(&self, _candidates: Vec<CandidateReceipt>) -> Result<Block, Error> {
		use chainx_api::BlockBuilder;
		use runtime_primitives::traits::{Hash as HashT, BlakeTwo256};
		use chainx_primitives::InherentData;

		const MAX_VOTE_OFFLINE_SECONDS: Duration = Duration::from_secs(60);

		// TODO: handle case when current timestamp behind that in state.
		let timestamp = current_timestamp();

		let elapsed_since_start = self.timing.dynamic_inclusion.started_at().elapsed();
		let offline_indices = if elapsed_since_start > MAX_VOTE_OFFLINE_SECONDS {
			Vec::new()
		} else {
			self.offline.read().reports(&self.validators[..])
		};

		if !offline_indices.is_empty() {
			info!(
				"Submitting offline validators {:?} for slash-vote",
				offline_indices.iter().map(|&i| self.validators[i as usize]).collect::<Vec<_>>(),
			)
		}

		let inherent_data = InherentData {
			timestamp,
			offline_indices,
		};

		let mut block_builder = self.client.build_block(&self.parent_id, inherent_data)?;

		{
			//let mut unqueue_invalid = Vec::new();
			let result = self.transaction_pool.cull_and_get_pending(&BlockId::hash(self.parent_hash), |_pending_iterator| {
                // TO DO:
				/*let mut _pending_size = 0;
				for pending in pending_iterator {
					if pending_size + pending.verified.encoded_size() >= MAX_TRANSACTIONS_SIZE { break }

					match block_builder.push_extrinsic(pending.original.clone()) {
						Ok(()) => {
							pending_size += pending.verified.encoded_size();
						}
						Err(e) => {
							trace!(target: "transaction-pool", "Invalid transaction: {}", e);
							unqueue_invalid.push(pending.verified.hash().clone());
						}
					}
				}*/
			});
			if let Err(e) = result {
				warn!("Unable to get the pending set: {:?}", e);
			}

			//self.transaction_pool.remove(&unqueue_invalid, false);
		}

		let chainx_block = block_builder.bake()?;

		info!("Proposing block [number: {}; hash: {}; parent_hash: {}; extrinsics: [{}]]",
			chainx_block.header.number,
			Hash::from(chainx_block.header.hash()),
			chainx_block.header.parent_hash,
			chainx_block.extrinsics.iter()
				.map(|xt| format!("{}", BlakeTwo256::hash_of(xt)))
				.collect::<Vec<_>>()
				.join(", ")
		);

		let substrate_block = Decode::decode(&mut chainx_block.encode().as_slice())
			.expect("chainx blocks defined to serialize to substrate blocks correctly; qed");

		// TODO: full re-evaluation
		assert!(evaluation::evaluate_initial(
			&substrate_block,
			timestamp,
			&self.parent_hash,
			self.parent_number,
		).is_ok());

		Ok(substrate_block)
	}
}

impl<C> Future for CreateProposal<C> where C: ChainXApi + Send + Sync {
	type Item = Block;
	type Error = Error;

	fn poll(&mut self) -> Poll<Block, Error> {
		// 1. try to propose if we have enough includable candidates and other
		// delays have concluded.
		let included = 0;
		try_ready!(self.timing.poll(included));

		// 2. propose
		let proposed_candidates : Vec<CandidateReceipt> = vec![];

		self.propose_with(proposed_candidates).map(Async::Ready)
	}
}
