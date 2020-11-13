// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use codec::{Decode, Encode};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

use sp_runtime::RuntimeDebug;
use sp_std::prelude::*;

use light_bitcoin::{
    chain::{BlockHeader as BtcHeader, Transaction as BtcTransaction},
    keys::Address,
    merkle::PartialMerkleTree,
    primitives::{Compact, H256},
};

use chainx_primitives::ReferralId;
use xp_gateway_bitcoin::BtcTxType;

/// BtcAddress is an bitcoin address encoded in base58
/// like: "1Nekoo5VTe7yQQ8WFqrva2UbdyRMVYCP1t" or "3J98t1WpEZ73CNmQviecrnyiWrnqRhWNLy"
/// not layout state or public or else.
pub type BtcAddress = Vec<u8>;

#[derive(Clone, RuntimeDebug)]
pub struct BtcRelayedTx {
    pub block_hash: H256,
    pub raw: BtcTransaction,
    pub merkle_proof: PartialMerkleTree,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct BtcRelayedTxInfo {
    pub block_hash: H256,
    pub merkle_proof: PartialMerkleTree,
}

impl BtcRelayedTxInfo {
    pub fn into_relayed_tx(self, tx: BtcTransaction) -> BtcRelayedTx {
        BtcRelayedTx {
            block_hash: self.block_hash,
            raw: tx,
            merkle_proof: self.merkle_proof,
        }
    }
}

#[derive(PartialEq, Eq, Clone, Default, Encode, Decode, RuntimeDebug)]
pub struct BtcHeaderInfo {
    pub header: BtcHeader,
    pub height: u32,
}

#[derive(PartialEq, Eq, Clone, Copy, Default, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct BtcHeaderIndex {
    pub hash: H256,
    pub height: u32,
}

#[derive(PartialEq, Clone, Copy, Eq, Encode, Decode, RuntimeDebug)]
pub struct BtcTxState {
    pub tx_type: BtcTxType,
    pub result: BtcTxResult,
}

#[derive(PartialEq, Clone, Copy, Eq, Encode, Decode, RuntimeDebug)]
pub enum BtcTxResult {
    Success,
    Failure,
}

pub enum AccountInfo<AccountId> {
    /// A value of type `L`.
    Account((AccountId, Option<ReferralId>)),
    /// A value of type `R`.
    Address(Address),
}

#[derive(PartialEq, Clone, Encode, Decode, Default, RuntimeDebug)]
pub struct BtcDepositCache {
    pub txid: H256,
    pub balance: u64,
}

#[derive(PartialEq, Clone, Encode, Decode, RuntimeDebug)]
pub struct BtcWithdrawalProposal<AccountId> {
    pub sig_state: VoteResult,
    pub withdrawal_id_list: Vec<u32>,
    pub tx: BtcTransaction,
    pub trustee_list: Vec<(AccountId, bool)>,
}

impl<AccountId> BtcWithdrawalProposal<AccountId> {
    pub fn new(
        sig_state: VoteResult,
        withdrawal_id_list: Vec<u32>,
        tx: BtcTransaction,
        trustee_list: Vec<(AccountId, bool)>,
    ) -> Self {
        Self {
            sig_state,
            withdrawal_id_list,
            tx,
            trustee_list,
        }
    }
}

#[derive(PartialEq, Clone, Copy, Eq, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum VoteResult {
    Unfinish,
    Finish,
}

#[derive(PartialEq, Eq, Clone, Copy, Encode, Decode, Default, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct BtcParams {
    max_bits: u32,
    block_max_future: u32,

    target_timespan_seconds: u32,
    target_spacing_seconds: u32,
    retargeting_factor: u32,

    retargeting_interval: u32,
    min_timespan: u32,
    max_timespan: u32,
}

impl BtcParams {
    pub fn new(
        max_bits: u32,
        block_max_future: u32,
        target_timespan_seconds: u32,
        target_spacing_seconds: u32,
        retargeting_factor: u32,
    ) -> BtcParams {
        Self {
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
    pub fn block_max_future(&self) -> u32 {
        self.block_max_future
    }
    pub fn target_timespan_seconds(&self) -> u32 {
        self.target_timespan_seconds
    }

    pub fn retargeting_interval(&self) -> u32 {
        self.retargeting_interval
    }
    pub fn min_timespan(&self) -> u32 {
        self.min_timespan
    }
    pub fn max_timespan(&self) -> u32 {
        self.max_timespan
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum BtcTxVerifier {
    Recover,
    RuntimeInterface,
    #[cfg(any(feature = "runtime-benchmarks", test))]
    /// Test would ignore sign check and always return true
    Test,
}

impl Default for BtcTxVerifier {
    fn default() -> Self {
        BtcTxVerifier::Recover
    }
}
