// Copyright 2018 chainpool

use super::*;
use state_machine::ExecutionManager;
use codec::{Encode, Decode};
use client::CallExecutor;

impl BlockBuilder for TClientBlockBuilder {
    fn push_extrinsic(&mut self, extrinsic: UncheckedExtrinsic) -> Result<()> {
          self.push(extrinsic).map_err(Into::into)
    }

    fn bake(self) -> Result<Block> {
          TClientBlockBuilder::bake(self).map_err(Into::into)
    }
}

fn call<R>(
	client: &TClient,
	at: &BlockId,
	function: &'static str,
	input: &[u8])
-> Result<R>
where
    R: Decode,
{
	let parent = at;
	let header = Header {
		parent_hash: client.block_hash_from_id(&parent)?
			.ok_or_else(|| ErrorKind::UnknownBlock(format!("{:?}", parent)))?,
			number: client.block_number_from_id(&parent)?
				.ok_or_else(|| ErrorKind::UnknownBlock(format!("{:?}", parent)))? + 1,
				state_root: Default::default(),
				extrinsics_root: Default::default(),
				digest: Default::default(),
	};
	client.state_at(&parent).map_err(Error::from).and_then(|state| {
		let mut overlay = Default::default();
		let execution_manager = || ExecutionManager::Both(|wasm_result, native_result| {
			warn!("Consensus error between wasm and native runtime execution at block {:?}", at);
			warn!("   Function {:?}", function);
			warn!("   Native result {:?}", native_result);
			warn!("   Wasm result {:?}", wasm_result);
			wasm_result
		});
		client.executor().call_at_state(
			&state,
			&mut overlay,
			"initialise_block",
			&header.encode(),
			execution_manager()
		)?;
		let (r, _) = client.executor().call_at_state(
			&state,
			&mut overlay,
			function,
			input,
			execution_manager()
		)?;
		Ok(Decode::decode(&mut &r[..])
		   .ok_or_else(|| client::error::Error::from(client::error::ErrorKind::CallResultDecode(function)))?)
	})
}

impl ChainXApi for TClient {
    type BlockBuilder = TClientBlockBuilder;

	fn session_keys(&self, at: &BlockId) -> Result<Vec<SessionKey>> {
		Ok(self.authorities_at(at)?)
	}

	fn validators(&self, at: &BlockId) -> Result<Vec<AccountId>> {
		call(self, at, "validators", &[])
	}

	fn random_seed(&self, at: &BlockId) -> Result<Hash> {
		call(self, at, "random_seed", &[])
	}

	fn timestamp(&self, at: &BlockId) -> Result<Timestamp> {
		call(self, at, "timestamp", &[])
	}

	fn evaluate_block(&self, at: &BlockId, block: Block) -> Result<bool> {
		let encoded = block.encode();
		let res: Result<()> = call(self, at, "execute_block", &encoded);
		match res {
			Ok(_) => Ok(true),
			Err(err) => match err.kind() {
				&ErrorKind::Execution(_) => Ok(false),
				_ => Err(err)
			}
		}
	}

	fn index(&self, at: &BlockId, account: AccountId) -> Result<Index> {
		account.using_encoded(|encoded| {
			call(self, at, "account_nonce", encoded)
		})
	}

	fn lookup(&self, at: &BlockId, address: Address) -> Result<Option<AccountId>> {
		address.using_encoded(|encoded| {
			call(self, at, "lookup_address", encoded)
		})
	}

	fn build_block(&self, at: &BlockId, inherent_data: InherentData) -> Result<Self::BlockBuilder> {
		let mut block_builder = self.new_block_at(at)?;
		for inherent in self.inherent_extrinsics(at, inherent_data)? {
			block_builder.push(inherent)?;
		}

		Ok(block_builder)
	}

	fn inherent_extrinsics(&self, at: &BlockId, inherent_data: InherentData) -> Result<Vec<UncheckedExtrinsic>> {
		let runtime_version = self.runtime_version_at(at)?;
		(inherent_data, runtime_version.spec_version).using_encoded(|encoded| {
			call(self, at, "inherent_extrinsics", encoded)
		})
	}
}


