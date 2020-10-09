// Copyright 2018-2019 Chainpool.

use parity_codec::{Decode, Encode};
#[cfg(feature = "std")]
use serde_derive::{Deserialize, Serialize};

// Substrate
use rstd::prelude::Vec;

// chainx
use xbridge_common::traits::IntoVecu8;

// light-bitcoin
use btc_chain::{BlockHeader, Transaction};
use btc_keys::Address;
use btc_primitives::{Compact, H256};
use merkle::PartialMerkleTree;

use crate::traits::RelayTransaction;

#[derive(PartialEq, Clone, Copy, Eq, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub enum TxType {
    Withdrawal,
    Deposit,
    HotAndCold,
    TrusteeTransition,
    Lock,
    Unlock,
    Irrelevance,
}

impl Default for TxType {
    fn default() -> Self {
        TxType::Deposit
    }
}

#[cfg_attr(feature = "std", derive(Debug))]
#[derive(Clone, Encode, Decode)]
pub struct RelayTx {
    pub block_hash: H256,
    pub raw: Transaction,
    pub merkle_proof: PartialMerkleTree,
    pub previous_raw: Transaction,
}

impl RelayTransaction for RelayTx {
    fn block_hash(&self) -> &H256 {
        &self.block_hash
    }
    fn raw_tx(&self) -> &Transaction {
        &self.raw
    }
    fn merkle_proof(&self) -> &PartialMerkleTree {
        &self.merkle_proof
    }
    fn prev_tx(&self) -> Option<&Transaction> {
        Some(&self.previous_raw)
    }
}

#[derive(PartialEq, Clone, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct WithdrawalProposal<AccountId> {
    pub sig_state: VoteResult,
    pub withdrawal_id_list: Vec<u32>,
    pub tx: Transaction,
    pub trustee_list: Vec<(AccountId, bool)>,
}

impl<AccountId> WithdrawalProposal<AccountId> {
    pub fn new(
        sig_state: VoteResult,
        withdrawal_id_list: Vec<u32>,
        tx: Transaction,
        trustee_list: Vec<(AccountId, bool)>,
    ) -> Self {
        WithdrawalProposal {
            sig_state,
            withdrawal_id_list,
            tx,
            trustee_list,
        }
    }
}

#[derive(PartialEq, Clone, Copy, Eq, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub enum VoteResult {
    Unfinish,
    Finish,
}

#[derive(PartialEq, Clone, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct BlockHeaderInfo {
    pub header: BlockHeader,
    pub height: u32,
    pub confirmed: bool,
    pub txid_list: Vec<H256>,
}

#[derive(PartialEq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct TxInfo {
    pub raw_tx: Transaction,
    pub tx_type: TxType,
    pub height: u32,
    #[deprecated(
        since = "1.0.0",
        note = "Please not modify this property, use `TxMarkFor2` instead of it"
    )]
    pub done: bool,
}

pub enum DepositAccountInfo<AccountId> {
    AccountId(AccountId),
    Address(Address),
}

#[derive(PartialEq, Clone, Encode, Decode, Default)]
pub struct DepositCache {
    pub txid: H256,
    pub balance: u64,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct TrusteeAddrInfo {
    pub addr: Address,
    pub redeem_script: Vec<u8>,
}

impl IntoVecu8 for TrusteeAddrInfo {
    fn into_vecu8(self) -> Vec<u8> {
        self.encode()
    }
    fn from_vecu8(src: &[u8]) -> Option<Self> {
        #[allow(clippy::useless_asref)]
        Decode::decode(&mut src.as_ref())
    }
}

#[cfg(feature = "std")]
impl std::fmt::Debug for TrusteeAddrInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        use rustc_hex::ToHex;
        let hex: String = self.redeem_script.to_hex();
        write!(
            f,
            "TrusteeAddrInfo {{ addr: {:?}, redeem_script: {:?} }}",
            self.addr, hex
        )
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct Params {
    max_bits: u32,
    //Compact
    block_max_future: u32,

    target_timespan_seconds: u32,
    target_spacing_seconds: u32,
    retargeting_factor: u32,

    retargeting_interval: u32,
    min_timespan: u32,
    max_timespan: u32,
}

impl Params {
    pub fn new(
        max_bits: u32,
        block_max_future: u32,
        target_timespan_seconds: u32,
        target_spacing_seconds: u32,
        retargeting_factor: u32,
    ) -> Params {
        Params {
            max_bits,
            block_max_future,

            target_timespan_seconds,
            target_spacing_seconds,
            retargeting_factor,

            retargeting_interval: target_timespan_seconds / target_spacing_seconds,
            min_timespan: target_timespan_seconds / retargeting_factor,
            max_timespan: target_timespan_seconds * retargeting_factor,
        }
    }

    pub fn max_bits(&self) -> Compact {
        Compact::new(self.max_bits)
    }

    pub fn retargeting_interval(&self) -> u32 {
        self.retargeting_interval
    }

    pub fn block_max_future(&self) -> u32 {
        self.block_max_future
    }
    pub fn min_timespan(&self) -> u32 {
        self.min_timespan
    }

    pub fn max_timespan(&self) -> u32 {
        self.max_timespan
    }
    pub fn target_timespan_seconds(&self) -> u32 {
        self.target_timespan_seconds
    }
}
