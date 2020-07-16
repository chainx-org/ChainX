// Copyright 2018-2019 Chainpool.

use codec::{Codec, Decode, Encode};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

// Substrate
use chainx_primitives::{AddrStr, AssetId, Memo, Token};
use sp_std::vec::Vec;

/// state machine for state is:
/// Applying(lock token) => Processing(can't cancel) =>
///        destroy token => NormalFinish|RootFinish (final state)
///        release token => NormalCancel(can from Applying directly)|RootCancel (final state)
#[derive(PartialEq, Eq, Clone, Copy, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub enum WithdrawalState {
    Applying,
    Processing,
    NormalFinish,
    RootFinish,
    NormalCancel,
    RootCancel,
}

impl Default for WithdrawalState {
    fn default() -> Self {
        WithdrawalState::Applying
    }
}

/// application for withdrawal
#[derive(PartialEq, Eq, Clone, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct Application<AccountId, Balance, BlockNumber> {
    pub asset_id: AssetId,
    pub applicant: AccountId,
    pub balance: Balance,
    pub addr: AddrStr,
    pub ext: Memo,
    pub height: BlockNumber,
}

impl<AccountId, Balance, BlockNumber> Application<AccountId, Balance, BlockNumber>
where
    AccountId: Codec + Clone,
    Balance: Codec + Copy + Clone,
    BlockNumber: Codec + Clone,
{
    pub fn new(
        applicant: AccountId,
        asset_id: AssetId,
        balance: Balance,
        addr: AddrStr,
        ext: Memo,
        height: BlockNumber,
    ) -> Self {
        Application::<AccountId, Balance, BlockNumber> {
            asset_id,
            applicant,
            balance,
            addr,
            ext,
            height,
        }
    }
    pub fn applicant(&self) -> AccountId {
        self.applicant.clone()
    }
    pub fn asset_id(&self) -> AssetId {
        self.asset_id
    }
    pub fn balance(&self) -> Balance {
        self.balance
    }
    pub fn addr(&self) -> AddrStr {
        self.addr.clone()
    }
    pub fn ext(&self) -> Memo {
        self.ext.clone()
    }
    pub fn height(&self) -> BlockNumber {
        self.height.clone()
    }
}

// for rpc
#[derive(PartialEq, Eq, Clone, Copy, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub enum TxState {
    NotApplying,
    Applying,
    Signing,
    Broadcasting,
    Processing,
    Confirming(u32, u32),
    Confirmed,
    Unknown,
}

impl Default for TxState {
    fn default() -> Self {
        TxState::NotApplying
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub enum HeightOrTime<BlockNumber, Timestamp> {
    Height(BlockNumber),
    Timestamp(Timestamp),
}

#[derive(PartialEq, Eq, Clone, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct RecordInfo<AccountId, Balance, BlockNumber: Default, Timestamp> {
    pub who: AccountId,
    pub token: Token,
    pub balance: Balance,
    // txhash
    pub txid: Vec<u8>,
    /// withdrawal addr or deposit from which addr
    pub addr: AddrStr,
    /// memo or ext info
    pub ext: Memo,
    /// tx height
    pub height_or_time: HeightOrTime<BlockNumber, Timestamp>,
    /// only for withdrawal, mark which id for application
    pub withdrawal_id: u32, // only for withdrawal
    /// tx state
    pub state: TxState,
    /// application state
    pub application_state: Option<WithdrawalState>,
}
