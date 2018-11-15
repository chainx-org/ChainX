// Copyright 2018 Chainpool.

extern crate sr_primitives as runtime_primitives;
extern crate srml_support as runtime_support;
extern crate substrate_primitives as primitives;
extern crate substrate_client as client;
extern crate parity_codec as codec;
extern crate substrate_transaction_pool;
extern crate substrate_bft as bft;
extern crate substrate_network;

extern crate chainx_primitives;
extern crate chainx_runtime;
extern crate chainx_pool;
extern crate chainx_api;

extern crate rhododendron;
extern crate exit_future;
extern crate parking_lot;
#[macro_use]
extern crate error_chain;
extern crate futures;
extern crate tokio;
#[macro_use]
extern crate log;

mod offline_tracker;
mod evaluation;
mod service;
mod error;

use chainx_primitives::{BlockId, Hash, Block, Header, AccountId, BlockNumber, Timestamp, SessionKey};
use primitives::{AuthorityId, ed25519};
use runtime_primitives::generic::Era;
use std::time::{Duration, Instant};
use tokio::runtime::TaskExecutor;
use tokio::timer::Delay;
use codec::{Decode, Encode};
use chainx_api::ChainXApi;
use parking_lot::RwLock;
use futures::prelude::*;
use futures::future;
use std::sync::Arc;

type TransactionPool<A> = substrate_transaction_pool::Pool<chainx_pool::PoolApi<A>>;

pub use self::offline_tracker::OfflineTracker;
pub use self::error::{ErrorKind, Error};
pub use service::Service;

/// Shared offline validator tracker.
pub type SharedOfflineTracker = Arc<RwLock<OfflineTracker>>;


// block size limit.
const MAX_TRANSACTIONS_SIZE: usize = 4 * 1024 * 1024;

/// A long-lived network which can create BFT message routing processes on demand.
pub trait Network {
    /// The input stream of BFT messages. Should never logically conclude.
    type Input: Stream<Item=bft::Communication<Block>, Error=Error>;
    /// The output sink of BFT messages. Messages sent here should eventually pass to all
    /// current authorities.
    type Output: Sink<SinkItem=bft::Communication<Block>, SinkError=Error>;

    fn communication_for(
        &self,
        validators: &[SessionKey],
        local_id: SessionKey,
        parent_hash: Hash,
        task_executor: TaskExecutor,
    ) -> (Self::Input, Self::Output);
}

/// ChainX proposer factory.
pub struct ProposerFactory<N, P>
    where
        P: ChainXApi + Send + Sync + 'static,
{
    /// The client instance.
    pub client: Arc<P>,
    /// transaction pool,
    pub transaction_pool: Arc<TransactionPool<P>>,
    /// The backing network handle.
    pub network: N,
    /// handle to remote task executor
    pub handle: TaskExecutor,
    /// Offline-tracker.
    pub offline: SharedOfflineTracker,
    /// Force delay in evaluation this long.
    pub force_delay: Timestamp,
}

impl<N, P> bft::Environment<Block> for ProposerFactory<N, P>
    where
        N: Network,
        P: ChainXApi + Send + Sync + 'static,
{
    type Proposer = Proposer<P>;
    type Input = N::Input;
    type Output = N::Output;
    type Error = Error;

    fn init(
        &self,
        parent_header: &Header,
        authorities: &[AuthorityId],
        sign_with: Arc<ed25519::Pair>,
    ) -> Result<(Self::Proposer, Self::Input, Self::Output), Error> {
        use runtime_primitives::traits::{Hash as HashT, BlakeTwo256};

        let parent_hash = parent_header.hash().into();

        let id = BlockId::hash(parent_hash);
        let random_seed = self.client.random_seed(&id)?;
        let random_seed = BlakeTwo256::hash(&*random_seed);

        let validators = self.client.validators(&id)?;
        self.offline.write().note_new_block(&validators[..]);

        let local_id = sign_with.public().0.into();
        let (input, output) = self.network.communication_for(
            authorities,
            local_id,
            parent_hash.clone(),
            self.handle.clone(),
        );
        let now = Instant::now();

        let proposer = Proposer {
            client: self.client.clone(),
            start: now,
            local_key: sign_with,
            parent_hash,
            parent_id: id,
            parent_number: parent_header.number,
            random_seed,
            transaction_pool: self.transaction_pool.clone(),
            offline: self.offline.clone(),
            validators,
            minimum_timestamp: current_timestamp() + self.force_delay,
        };

        Ok((proposer, input, output))
    }
}

/// The ChainX proposer logic.
pub struct Proposer<C: ChainXApi + Send + Sync>
{
    client: Arc<C>,
    start: Instant,
    local_key: Arc<ed25519::Pair>,
    parent_hash: Hash,
    parent_id: BlockId,
    parent_number: BlockNumber,
    random_seed: Hash,
    transaction_pool: Arc<TransactionPool<C>>,
    offline: SharedOfflineTracker,
    validators: Vec<AccountId>,
    minimum_timestamp: u64,
}

impl<C: ChainXApi + Send + Sync> Proposer<C> {
    fn primary_index(&self, round_number: usize, _len: usize) -> usize {
        use primitives::uint::U256;

        let validators_len = self.validators.len();
        let mut stake_weight = self.validators.iter()
            .map(|account| {
                let weight = self.client.stake_weight(&self.parent_id, *account).unwrap();
                if weight == 0 { 1 } else { weight }
            })
            .collect::<Vec<_>>();
        info!("validator stake weight:{:?}, round_numer:{:?}", stake_weight, round_number);
        for i in 1..validators_len {
            stake_weight[i] = stake_weight[i] + stake_weight[i - 1];
        }
        let big_len = stake_weight[validators_len - 1] as u128;
        let big_value = U256::from_big_endian(&self.random_seed.0).low_u64() as u128;
        let offset = (big_value * big_value) % big_len;

        for i in 0..validators_len {
            if offset < stake_weight[i] as u128 {
                return (i + round_number) % validators_len;
            }
        }
        return round_number % validators_len;
    }

    fn primary_validator(&self, round_number: usize) -> Option<AccountId> {
        let i = self.primary_index(round_number, 0);
        return Some(self.validators[i]);
    }
}

impl<C> bft::Proposer<Block> for Proposer<C>
    where
        C: ChainXApi + Send + Sync,
{
    type Error = Error;
    type Create = Result<Block, Error>;
    type Evaluate = Box<Future<Item=bool, Error=Error>>;

    fn propose(&self) -> Self::Create {
        use chainx_api::BlockBuilder;
        use runtime_primitives::traits::{Hash as HashT, BlakeTwo256};
        use chainx_primitives::InherentData;

        const MAX_VOTE_OFFLINE_SECONDS: Duration = Duration::from_secs(60);

        // TODO: handle case when current timestamp behind that in state.
        let timestamp = ::std::cmp::max(self.minimum_timestamp, current_timestamp());

        let elapsed_since_start = self.start.elapsed();
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

        let block_producer: AccountId = self.local_key.public().0.into();

        let inherent_data = InherentData {
            timestamp,
            offline_indices,
            block_producer: block_producer.clone(),
        };

        let mut block_builder = self.client.build_block(&self.parent_id, inherent_data)?;

        {
            let mut unqueue_invalid = Vec::new();
            let result = self.transaction_pool.cull_and_get_pending(&BlockId::hash(self.parent_hash), |pending_iterator| {
                let mut pending_size = 0;
                for pending in pending_iterator {
                    if pending_size + pending.verified.encoded_size() >= MAX_TRANSACTIONS_SIZE { break; }

                    match block_builder.push_extrinsic(pending.original.clone()) {
                        Ok(()) => {
                            pending_size += pending.verified.encoded_size();
                        }
                        Err(e) => {
                            trace!(target: "transaction-pool", "Invalid transaction: {}", e);
                            unqueue_invalid.push(pending.verified.hash().clone());
                        }
                    }
                }
            });
            if let Err(e) = result {
                warn!("Unable to get the pending set: {:?}", e);
            }

            self.transaction_pool.remove(&unqueue_invalid, false);
        }

        let block = block_builder.bake()?;
        info!("generate a new block#{:}, producer:[{:}]", block.header.number, block_producer);
        trace!("Proposing block [number: {}; hash: {}; parent_hash: {}; extrinsics: [{}]]",
               block.header.number,
               Hash::from(block.header.hash()),
               block.header.parent_hash,
               block.extrinsics.iter()
                   .map(|xt| format!("{}", BlakeTwo256::hash_of(xt)))
                   .collect::<Vec<_>>()
                   .join(", ")
        );

        let substrate_block = Decode::decode(&mut block.encode().as_slice())
            .expect("blocks are defined to serialize to substrate blocks correctly; qed");

        assert!(evaluation::evaluate_initial(
            &substrate_block,
            timestamp,
            &self.parent_hash,
            self.parent_number,
        ).is_ok());

        Ok(substrate_block)
    }

    fn evaluate(&self, unchecked_proposal: &Block) -> Self::Evaluate {
        trace!(target: "bft", "evaluating block on top of parent ({}, {:?})", self.parent_number, self.parent_hash);

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
                let delay_s = proposed_timestamp - current_timestamp;
                debug!(target: "bft", "Delaying evaluation of proposal for {} seconds", delay_s);
                Some(now + Duration::from_secs(delay_s))
            } else {
                None
            };

            match timestamp_delay {
                Some(duration) => future::Either::A(Delay::new(duration).map_err(
                    |e| Error::from(ErrorKind::Timer(e)),
                )),
                None => future::Either::B(future::ok(())),
            }
        };

        // refuse to vote if this block says a validator is offline that we
        // think isn't.
        let offline = proposal.noted_offline();
        if !self.offline.read().check_consistency(
            &self.validators[..],
            offline,
        )
            {
                return Box::new(futures::empty());
            }

        // check block_producer
        match proposal.block_producer() {
            Some(a) => { info!("current block#{:}, producer is [{:}]", self.parent_number + 1, a); }
            None => { info!("current block#{:}, not set producer", self.parent_number + 1); }
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
        debug!(target: "bft", "proposer for round {} is {}", round_number, proposer);

        proposer
    }

    fn import_misbehavior(&self, misbehavior: Vec<(AuthorityId, bft::Misbehavior<Hash>)>) {
        use rhododendron::Misbehavior as GenericMisbehavior;
        use runtime_primitives::bft::{MisbehaviorKind, MisbehaviorReport};
        use chainx_primitives::UncheckedExtrinsic as GenericExtrinsic;
        use chainx_runtime::{Call, UncheckedExtrinsic, ConsensusCall};

        let local_id = self.local_key.public().0.into();
        let mut next_index = {
            let cur_index = self.transaction_pool.cull_and_get_pending(&BlockId::hash(self.parent_hash), |pending| pending
                .filter(|tx| tx.verified.sender == local_id)
                .last()
                .map(|tx| Ok(tx.verified.index()))
                .unwrap_or_else(|| self.client.index(&self.parent_id, local_id)),
            );

            match cur_index {
                Ok(Ok(cur_index)) => cur_index + 1,
                Ok(Err(e)) => {
                    warn!(target: "consensus", "Error computing next transaction index: {}", e);
                    return;
                }
                Err(e) => {
                    warn!(target: "consensus", "Error computing next transaction index: {}", e);
                    return;
                }
            }
        };

        for (target, misbehavior) in misbehavior {
            let report = MisbehaviorReport {
                parent_hash: self.parent_hash,
                parent_number: self.parent_number,
                target,
                misbehavior: match misbehavior {
                    GenericMisbehavior::ProposeOutOfTurn(_, _, _) => continue,
                    GenericMisbehavior::DoublePropose(_, _, _) => continue,
                    GenericMisbehavior::DoublePrepare(round, (h1, s1), (h2, s2))
                    => MisbehaviorKind::BftDoublePrepare(round as u32, (h1, s1.signature), (h2, s2.signature)),
                    GenericMisbehavior::DoubleCommit(round, (h1, s1), (h2, s2))
                    => MisbehaviorKind::BftDoubleCommit(round as u32, (h1, s1.signature), (h2, s2.signature)),
                },
            };
            let payload = (next_index, Call::Consensus(ConsensusCall::report_misbehavior(report)));
            let signature = self.local_key.sign(&payload.encode()).into();
            next_index += 1;

            let local_id = self.local_key.public().0.into();
            let extrinsic = UncheckedExtrinsic {
                signature: Some((chainx_runtime::RawAddress::Id(local_id), signature, payload.0, Era::immortal())),
                function: payload.1,
            };
            let uxt: GenericExtrinsic = Decode::decode(&mut extrinsic.encode().as_slice()).expect("Encoded extrinsic is valid");
            self.transaction_pool.submit_one(&BlockId::hash(self.parent_hash), uxt)
                .expect("locally signed extrinsic is valid; qed");
        }
    }

    fn on_round_end(&self, round_number: usize, was_proposed: bool) {
        let primary_validator = self.primary_validator(round_number).unwrap();

        // alter the message based on whether we think the empty proposer was forced to skip the round.
        // this is determined by checking if our local validator would have been forced to skip the round.
        if !was_proposed {
            let public = ed25519::Public::from_raw(primary_validator.0);
            info!(
                "Potential Offline Validator: {} failed to propose during assigned slot: {}",
                public,
                round_number,
            );
        }

        self.offline.write().note_round_end(
            primary_validator, was_proposed,
        );
    }
}

fn current_timestamp() -> Timestamp {
    use std::time;

    time::SystemTime::now()
        .duration_since(time::UNIX_EPOCH)
        .expect("now always later than unix epoch; qed")
        .as_secs()
}
