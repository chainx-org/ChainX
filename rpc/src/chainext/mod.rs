use jsonrpc_macros::Trailing;
use primitives::Blake2Hasher;
use runtime_primitives::generic::{BlockId, SignedBlock};
use runtime_primitives::traits::{Block as BlockT, Header, NumberFor};

use client::{self, Client};
use std::sync::Arc;
use tokio::runtime::TaskExecutor;

mod error;

use self::error::Result;

build_rpc_trait! {
    pub trait ChainApiExt<Hash, Header, Number, Extrinsic> {

        #[rpc(name = "chainext_getBlockByNumber")]
        fn block_info(&self, Trailing<Number>) -> Result<Option<SignedBlock<Header, Extrinsic, Hash>>>;
    }
}

pub struct ChainExt<B, E, Block: BlockT> {
    client: Arc<Client<B, E, Block>>,
}

impl<B, E, Block: BlockT> ChainExt<B, E, Block> {
    pub fn new(client: Arc<Client<B, E, Block>>, _executor: TaskExecutor) -> Self {
        Self { client }
    }
}

impl<B, E, Block> ChainApiExt<Block::Hash, Block::Header, NumberFor<Block>, Block::Extrinsic>
    for ChainExt<B, E, Block>
where
    Block: BlockT + 'static,
    B: client::backend::Backend<Block, Blake2Hasher> + Send + Sync + 'static,
    E: client::CallExecutor<Block, Blake2Hasher> + Send + Sync + 'static,
{
    fn block_info(
        &self,
        number: Trailing<NumberFor<Block>>,
    ) -> Result<Option<SignedBlock<Block::Header, Block::Extrinsic, Block::Hash>>> {
        let hash = match number.into() {
            None => Some(self.client.info()?.chain.best_hash),
            Some(number) => self
                .client
                .header(&BlockId::number(number))?
                .map(|h| h.hash()),
        };
        let block_hash = match hash {
            None => self.client.info()?.chain.best_hash,
            Some(h) => h,
        };

        Ok(self.client.block(&BlockId::Hash(block_hash))?)
    }
}
