// Copyright 2018 Chainpool.

use extrinsic_pool::{Pool, ChainApi, VerifiedFor, ExtrinsicFor, scoring,
                     Readiness, VerifiedTransaction, Transaction, Options, scoring::Choice};
use runtime_primitives::traits::{Hash as HashT, Bounded, Checkable, BlakeTwo256, Lookup, CurrentHeight, BlockNumberToHash};
use std::{cmp::Ordering, collections::HashMap, sync::Arc};
use chainx_primitives::{Block, Hash, BlockId, AccountId, Index, BlockNumber};
use chainx_runtime::{UncheckedExtrinsic, Address};
use substrate_executor::NativeExecutor;
use substrate_client::{self, Client};
use extrinsic_pool::IntoPoolError;
use codec::{Encode, Decode};
use chainx_api::ChainXApi;
use substrate_client_db;
use substrate_network;
use chainx_executor;
use extrinsic_pool;
use error::{Error, ErrorKind, Result};

type Executor = substrate_client::LocalCallExecutor<Backend, NativeExecutor<chainx_executor::Executor>>;
type Backend = substrate_client_db::Backend<Block>;

const MAX_TRANSACTION_SIZE: usize = 4 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct VerifiedExtrinsic {
    /// Transaction hash.
    pub hash: Hash,
    /// Transaction sender.
    pub sender: AccountId,
    /// Transaction index.
    pub index: Index,
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
    pub fn index(&self) -> Index {
        self.index
    }
}

impl VerifiedTransaction for VerifiedExtrinsic {
    type Hash = Hash;
    type Sender = AccountId;

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

pub struct LocalContext<'a, A: 'a>(&'a Arc<A>);
impl<'a, A: 'a + ChainXApi> CurrentHeight for LocalContext<'a, A> {
    type BlockNumber = BlockNumber;
    fn current_height(&self) -> BlockNumber {
        self.0.current_height()
    }
}
impl<'a, A: 'a + ChainXApi> BlockNumberToHash for LocalContext<'a, A> {
    type BlockNumber = BlockNumber;
    type Hash = Hash;
    fn block_number_to_hash(&self, n: BlockNumber) -> Option<Hash> {
        self.0.block_number_to_hash(n)
    }
}
impl<'a, A: 'a + ChainXApi> Lookup for LocalContext<'a, A> {
    type Source = Address;
    type Target = AccountId;
    fn lookup(&self, a: Address) -> ::std::result::Result<AccountId, &'static str> {
        self.0.lookup(&BlockId::number(self.current_height()), a).unwrap_or(None).ok_or("error with lookup")
    }
}

pub struct PoolApi<A>{
    api:Arc<A>,
}

impl<A> PoolApi<A> where
    A: ChainXApi,
{
    /// Create a new instance.
    pub fn new(api: Arc<A>) -> Self {
        PoolApi {
            api,
        }
    }
}

impl<A> ChainApi for PoolApi<A> where
    A: ChainXApi + Send + Sync,
{
    type Ready = HashMap<AccountId, u64>;
    type Sender = AccountId;
    type VEx = VerifiedExtrinsic;
    type Block = Block;
    type Error = Error;
    type Hash = Hash;
    type Score = u64;
    type Event = ();

    fn verify_transaction(
        &self,
        _at: &BlockId,
        xt: &ExtrinsicFor<Self>,
    ) -> Result<Self::VEx> {

        let encoded = xt.encode();
        let uxt = UncheckedExtrinsic::decode(&mut encoded.as_slice()).ok_or_else(|| ErrorKind::InvalidExtrinsicFormat)?;

        if !uxt.is_signed() {
            bail!(ErrorKind::IsInherent(uxt))
        }

        let (encoded_size, hash) = (encoded.len(), BlakeTwo256::hash(&encoded));
        if encoded_size > MAX_TRANSACTION_SIZE {
            bail!(ErrorKind::TooLarge(encoded_size, MAX_TRANSACTION_SIZE));
        }

       debug!(target: "transaction-pool", "Transaction submitted: {}", ::substrate_primitives::hexdisplay::HexDisplay::from(&encoded));
        let checked = uxt.clone().check(&LocalContext(&self.api))?;
        let (sender, index) = checked.signed.expect("function previously bailed unless uxt.is_signed(); qed");

        if encoded_size < 1024 {
            debug!(target: "transaction-pool", "Transaction verified: {} => {:?}", hash, uxt);
        } else {
            debug!(target: "transaction-pool", "Transaction verified: {} ({} bytes is too large to display)", hash, encoded_size);
        }

        Ok(VerifiedExtrinsic {
            index,
            sender,
            hash,
            encoded_size,
        })
    }

    fn ready(&self) -> Self::Ready {
        HashMap::default()
    }

    fn is_ready(
        &self,
        at: &BlockId,
        known_nonces: &mut Self::Ready,
        xt: &VerifiedFor<Self>,
    ) -> Readiness {
        let sender = xt.verified.sender().clone();
        trace!(target: "transaction-pool", "Checking readiness of {} (from {})", xt.verified.hash, sender);

        // TODO: find a way to handle index error properly -- will need changes to
        // transaction-pool trait.
        let api = &self.api;
        let next_index = known_nonces.entry(sender)
            .or_insert_with(|| api.index(at, sender).ok().unwrap_or_else(Bounded::max_value));

        let result = match xt.verified.index.cmp(&next_index) {
            Ordering::Greater => Readiness::Future,
            Ordering::Equal => Readiness::Ready,
            // TODO [ToDr] Should mark transactions referencing too old blockhash as `Stale` as well.
            Ordering::Less => Readiness::Stale,
        };

        // remember to increment `next_index`
        *next_index = next_index.saturating_add(1);
        result
    }

    fn compare(old: &VerifiedFor<Self>, other: &VerifiedFor<Self>) -> Ordering {
        old.verified.index().cmp(&other.verified.index())
    }

    fn choose(old: &VerifiedFor<Self>, new: &VerifiedFor<Self>) -> scoring::Choice {
        if old.verified.index() == new.verified.index() {
            return Choice::ReplaceOld;
        }
        Choice::InsertNew
    }

    fn update_scores(
        xts: &[Transaction<VerifiedFor<Self>>],
        scores: &mut [Self::Score],
        _change: scoring::Change<()>,
    ) {
        for i in 0..xts.len() {
           scores[i] = 1;
        }
    }

    fn should_replace(_old: &VerifiedFor<Self>, _new: &VerifiedFor<Self>) -> scoring::Choice {
        Choice::RejectNew
    }

    fn latest_hash(&self) -> Hash {
        self.api.block_number_to_hash(self.api.current_height()).expect("Latest block number always has a hash; qed")
    }
}

pub struct TransactionPool<A> where
    A: ChainXApi + Send + Sync,
{
    inner: Arc<Pool<PoolApi<A>>>,
    client: Arc<Client<Backend, Executor, Block>>,
}

impl<A> TransactionPool<A> where
    A: ChainXApi + Send + Sync,
{
    /// Create a new transaction pool.
    pub fn new(
        options: Options,
        api: PoolApi<A>,
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

    pub fn inner(&self) -> Arc<Pool<PoolApi<A>>> {
        self.inner.clone()
    }
}

impl<A> substrate_network::TransactionPool<Hash, Block> for TransactionPool<A> where
    A: ChainXApi + Send + Sync,
{
    fn transactions(&self) -> Vec<(Hash, ExtrinsicFor<PoolApi<A>>)> {
        let best_block_id = match self.best_block_id() {
            Some(id) => id,
            None => return vec![],
        };
        self.inner
            .cull_and_get_pending(&best_block_id, |pending| {
                pending
                    .map(|t| {
                        let hash = t.hash().clone();
                        let ex:ExtrinsicFor<PoolApi<A>> = t.original.clone();
                        (hash, ex)
                    })
                    .collect()
            })
            .unwrap_or_else(|e| {
                warn!("Error retrieving pending set: {}", e);
                vec![]
            })
    }

    fn import(&self, transaction: &ExtrinsicFor<PoolApi<A>>) -> Option<Hash> {
        let encoded = transaction.encode();
        if let Some(uxt) = Decode::decode(&mut &encoded[..]) {
            let best_block_id = self.best_block_id()?;
            match self.inner.submit_one(&best_block_id, uxt) {
                Ok(xt) => Some(*xt.hash()),
                Err(e) => match e.into_pool_error() {
                    Ok(e) => match e.kind() {
                        extrinsic_pool::ErrorKind::AlreadyImported(hash) =>
                            Some(::std::str::FromStr::from_str(&hash).map_err(|_| {})
                                .expect("Hash string is always valid")),
                        _ => {
                            debug!("Error adding transaction to the pool: {:?}", e);
                            None
                        },
                    },
                    Err(e) => {
                        debug!("Error converting pool error: {:?}", e);
                        None
                    }
                }
            }
        } else {
            debug!("Error decoding transaction");
            None
        }
    }

    fn on_broadcasted(&self, propagations: HashMap<Hash, Vec<String>>) {
        self.inner.on_broadcasted(propagations)
    }
}
