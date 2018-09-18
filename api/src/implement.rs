// Copyright 2018 chainpool

use super::*;

impl BlockBuilder for TClientBlockBuilder {
    fn push_extrinsic(&mut self, extrinsic: UncheckedExtrinsic) -> Result<()> {
        self.push(extrinsic).map_err(Into::into)
    }

    fn bake(self) -> Result<Block> {
        TClientBlockBuilder::bake(self).map_err(Into::into)
    }
}

impl ChainXApi for TClient {
    type BlockBuilder = TClientBlockBuilder;

    fn session_keys(&self, at: &BlockId) -> Result<Vec<SessionKey>> {
        Ok(self.authorities_at(at)?)
    }

    fn validators(&self, at: &BlockId) -> Result<Vec<AccountId>> {
        self.call_api_at(at, "validators", &())
    }

    fn random_seed(&self, at: &BlockId) -> Result<Hash> {
        self.call_api_at(at, "random_seed", &())
    }

    fn timestamp(&self, at: &BlockId) -> Result<Timestamp> {
        self.call_api_at(at, "timestamp", &())
    }

    fn evaluate_block(&self, at: &BlockId, block: Block) -> Result<bool> {
        let res: Result<()> = self.call_api_at(at, "execute_block", &block);
        match res {
            Ok(_) => Ok(true),
            Err(err) => {
                match err.kind() {
                    &ErrorKind::Execution(_) => Ok(false),
                    _ => Err(err),
                }
            }
        }
    }

    fn index(&self, at: &BlockId, account: AccountId) -> Result<Index> {
       self.call_api_at(at, "account_nonce", &account)
    }

    fn lookup(&self, at: &BlockId, address: Address) -> Result<Option<AccountId>> {
        self.call_api_at(at, "lookup_address", &address)
    }

    fn build_block(&self, at: &BlockId, inherent_data: InherentData) -> Result<Self::BlockBuilder> {
//        let mut block_builder = self.new_block_at(at)?;
//        for inherent in self.inherent_extrinsics(at, inherent_data)? {
//            block_builder.push(inherent)?;
//        }
//
//        Ok(block_builder)
        let runtime_version = self.runtime_version_at(at)?;

        let mut block_builder = self.new_block_at(at)?;
        if runtime_version.has_api(*b"inherent", 1) {
            for inherent in self.inherent_extrinsics(at, inherent_data)? {
                block_builder.push(inherent)?;
            }
        }
        Ok(block_builder)
    }

    fn inherent_extrinsics(
        &self,
        at: &BlockId,
        inherent_data: InherentData,
    ) -> Result<Vec<UncheckedExtrinsic>> {
//        let runtime_version = self.runtime_version_at(at)?;
        // TODO change for api
        self.call_api_at(at, "inherent_extrinsics", &inherent_data)
//        self.call_api_at(at, "inherent_extrinsics", &(inherent_data, runtime_version.spec_version))
    }
}
