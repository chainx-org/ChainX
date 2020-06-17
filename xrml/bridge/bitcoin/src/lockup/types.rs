use btc_chain::Transaction;
use btc_primitives::H256;
use merkle::PartialMerkleTree;
use parity_codec::{Decode, Encode};

use crate::traits::RelayTransaction;

#[derive(PartialEq, Clone, Default, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct LockupRelayTx {
    pub block_hash: H256,
    pub merkle_proof: PartialMerkleTree,
    pub raw_tx: Transaction,
}

impl RelayTransaction for LockupRelayTx {
    fn block_hash(&self) -> &H256 {
        &self.block_hash
    }
    fn raw_tx(&self) -> &Transaction {
        &self.raw_tx
    }
    fn merkle_proof(&self) -> &PartialMerkleTree {
        &self.merkle_proof
    }
    fn prev_tx(&self) -> Option<&Transaction> {
        None
    }
}
