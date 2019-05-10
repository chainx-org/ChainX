// Copyright 2018-2019 Chainpool.

use parity_codec::{Codec, Decode, Encode};
#[cfg(feature = "std")]
use serde_derive::{Deserialize, Serialize};

// Substrate
use rstd::vec::Vec;
use xassets::{Memo, Token};
use xr_primitives::AddrStr;
use xsupport::storage::linked_node::{LinkedNodeCollection, NodeT};

use super::{ApplicationMHeader, ApplicationMTail, ApplicationMap, Trait};

/// state machine for state is:
/// Applying(lock token) => Processing(can't cancel) =>
///        destroy token => NormalFinish|RootFinish (final state)
///        release token => NormalCancel(can from Applying directly)|RootCancel (final state)
#[derive(PartialEq, Eq, Clone, Copy, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub enum ApplicationState {
    Applying,
    Processing,
    NormalFinish,
    RootFinish,
    NormalCancel,
    RootCancel,
}

impl Default for ApplicationState {
    fn default() -> Self {
        ApplicationState::Applying
    }
}

/// application for withdrawal
#[derive(PartialEq, Eq, Clone, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct Application<AccountId, Balance, BlockNumber> {
    pub id: u32,
    pub state: ApplicationState,
    pub applicant: AccountId,
    pub token: Token,
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
        id: u32,
        applicant: AccountId,
        token: Token,
        balance: Balance,
        addr: AddrStr,
        ext: Memo,
        height: BlockNumber,
    ) -> Self {
        Application::<AccountId, Balance, BlockNumber> {
            id,
            state: ApplicationState::Applying, // init state,
            applicant,
            token,
            balance,
            addr,
            ext,
            height,
        }
    }
    pub fn id(&self) -> u32 {
        self.id
    }
    pub fn state(&self) -> ApplicationState {
        self.state
    }
    pub fn applicant(&self) -> AccountId {
        self.applicant.clone()
    }
    pub fn token(&self) -> Token {
        self.token.clone()
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

impl<AccountId, Balance, BlockNumber> NodeT for Application<AccountId, Balance, BlockNumber> {
    type Index = u32;
    fn index(&self) -> Self::Index {
        self.id
    }
}

pub struct LinkedMultiKey<T: Trait>(rstd::marker::PhantomData<T>);

impl<T: Trait> LinkedNodeCollection for LinkedMultiKey<T> {
    type Header = ApplicationMHeader<T>;
    type NodeMap = ApplicationMap<T>;
    type Tail = ApplicationMTail<T>;
}

// for rpc
#[derive(PartialEq, Eq, Clone, Copy, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
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
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
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
}
