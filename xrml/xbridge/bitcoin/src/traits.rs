use btc_chain::Transaction;
use btc_primitives::H256;
use merkle::PartialMerkleTree;

pub trait RelayTransaction {
    fn block_hash(&self) -> &H256;
    fn tx_hash(&self) -> H256 {
        self.raw_tx().hash()
    }
    fn raw_tx(&self) -> &Transaction;
    fn merkle_proof(&self) -> &PartialMerkleTree;
    fn prev_tx(&self) -> Option<&Transaction>;
}
