// Copyright 2018 Chainpool.

use std::{cmp::Ordering, collections::HashMap, sync::Arc};

use extrinsic_pool::{Pool, ChainApi, VerifiedFor, ExtrinsicFor, scoring,
    Readiness, VerifiedTransaction, Transaction, Error, ErrorKind, Options,
    scoring::Choice};
use runtime_primitives::traits::{Hash as HashT, BlakeTwo256};
use codec::{Encode, Decode};
use substrate_client::{self, Client};
use substrate_client_db;
use substrate_network;
use substrate_executor::NativeExecutor;

use chainx_primitives::{Block, Hash, BlockId, AccountId};
use chainx_runtime::UncheckedExtrinsic;
use chainx_executor;

pub type Backend = substrate_client_db::Backend<Block>;
pub type Executor = substrate_client::LocalCallExecutor<Backend, NativeExecutor<chainx_executor::Executor>>;

#[derive(Debug, Clone)]
pub struct VerifiedExtrinsic {
    sender: Hash,
    hash: Hash,
    encoded_size: usize,
}

impl VerifiedExtrinsic {
    /// Get the 256-bit hash of this transaction.
    pub fn hash(&self) -> &Hash {
        &self.hash
    }
    /// Get encoded size of the transaction.
    pub fn encoded_size(&self) -> usize {
        self.encoded_size
    }
    /// Get the account ID of the sender of this transaction.
    pub fn sender(&self) -> Option<Hash> {
        Some(self.sender)
    }
}

impl VerifiedTransaction for VerifiedExtrinsic {
    type Hash = Hash;
    type Sender = Hash;

    fn hash(&self) -> &Self::Hash {
        &self.hash
    }

    fn sender(&self) -> &Self::Sender {
        &self.sender
    }

    fn mem_usage(&self) -> usize {
        self.encoded_size
    }
}

pub struct PoolApi;
impl PoolApi {
    pub fn default() -> Self {
        PoolApi
    }
}

impl ChainApi for PoolApi {
    type Block = Block;
    type Hash = Hash;
    type Sender = AccountId;
    type VEx = VerifiedExtrinsic;
    type Ready = HashMap<AccountId, u64>;
    type Error = Error;
    type Score = u64;
    type Event = ();

    fn verify_transaction(
        &self,
        _at: &BlockId,
        uxt: &ExtrinsicFor<Self>,
    ) -> Result<Self::VEx, Self::Error> {
        let encoded = uxt.encode();
        let (encoded_size, hash) = (uxt.len(), BlakeTwo256::hash(&encoded));
        Ok(VerifiedExtrinsic{
            sender:hash,
            hash,
            encoded_size,
        }
        )
    }

    fn ready(&self) -> Self::Ready {

        HashMap::default()
    }


    fn is_ready(
        &self,
        _at: &BlockId,
        _nonce_cache: &mut Self::Ready,
        _xt: &VerifiedFor<Self>,
    ) -> Readiness {
        Readiness::Ready
    }

    fn compare(_old: &VerifiedFor<Self>, _other: &VerifiedFor<Self>) -> Ordering {
        Ordering::Equal
    }

    fn choose(_old: &VerifiedFor<Self>, _new: &VerifiedFor<Self>) -> scoring::Choice {
        Choice::InsertNew
    }

    fn update_scores(
        _xts: &[Transaction<VerifiedFor<Self>>],
        _scores: &mut [Self::Score],
        _change: scoring::Change<()>,
    ) {}

    fn should_replace(_old: &VerifiedFor<Self>, _new: &VerifiedFor<Self>) -> scoring::Choice {
        Choice::InsertNew
    }
}


pub struct TransactionPool {
    inner: Arc<Pool<PoolApi>>,
    client: Arc<Client<Backend, Executor, Block>>,
}

impl TransactionPool {
    /// Create a new transaction pool.
    pub fn new(
        options: Options,
        api: PoolApi,
        client: Arc<Client<Backend, Executor, Block>>,
    ) -> Self {
        TransactionPool {
            inner: Arc::new(Pool::new(options, api)),
            client,
        }
    }

    pub fn best_block_id(&self) -> Option<BlockId> {
        self.client
            .info()
            .map(|info| BlockId::hash(info.chain.best_hash))
            .ok()
    }

    pub fn inner(&self) -> Arc<Pool<PoolApi>> {
        self.inner.clone()
    }
}

impl substrate_network::TransactionPool<Hash, Block> for TransactionPool {
    fn transactions(&self) -> Vec<(Hash, ExtrinsicFor<PoolApi>)> {
        let best_block_id = match self.best_block_id() {
            Some(id) => id,
            None => return vec![],
        };
        self.inner
            .cull_and_get_pending(&best_block_id, |pending| {
                pending
                    .map(|t| {
                        let hash = t.hash().clone();
                        let ex:ExtrinsicFor<PoolApi> = t.original.clone();
                        (hash, ex)
                    })
                    .collect()
            })
            .unwrap_or_else(|_e| {
                //warn!("Error retrieving pending set: {}", e);
                vec![]
            })
    }

    fn import(&self, transaction: &ExtrinsicFor<PoolApi>) -> Option<Hash> {
        match UncheckedExtrinsic::decode(&mut &transaction[..]) {
            Some(_) => {
                let best_block_id = self.best_block_id()?;
                match self.inner.submit_one(&best_block_id, transaction.clone()) {
                    Ok(xt) => Some(*xt.hash()),
                    Err(e) => {
                        match e.kind() {
                            ErrorKind::AlreadyImported(hash) => Some(
                                ::std::str::FromStr::from_str(&hash)
                                    .map_err(|_| {})
                                    .expect("Hash string is always valid"),
                            ),
                            _ => {
                                //debug!("Error adding transaction to the pool: {:?}", e);
                                None
                            }
                        }
                    }
                }
            },

            None => {
                //debug!("Error decoding transaction");
                None
            }
        }
    }

    fn on_broadcasted(&self, propagations: HashMap<Hash, Vec<String>>) {
        self.inner.on_broadcasted(propagations)
    }
}
