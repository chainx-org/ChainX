// Copyright 2018-2019 Chainpool.

use parity_codec::{Codec, Decode, Encode};
#[cfg(feature = "std")]
use serde_derive::{Deserialize, Serialize};

// Substrate
use rstd::vec::Vec;
use xassets::{Memo, Token};
use xr_primitives::XString;
use xsupport::storage::linked_node::{LinkedNodeCollection, NodeT};

use super::{ApplicationMHeader, ApplicationMTail, ApplicationMap, Trait};

pub type AddrStr = XString;

#[derive(PartialEq, Eq, Clone, Encode, Decode)]
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

#[derive(PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct RecordInfo<AccountId, Balance, Moment> {
    pub who: AccountId,
    pub token: Token,
    pub balance: Balance,
    // txhash
    pub txid: Vec<u8>,
    /// withdrawal addr or deposit from which addr
    pub addr: AddrStr,
    /// memo or ext info
    pub ext: Memo,
    /// tx time
    pub time: Moment,
    /// only for withdrawal, mark which id for application
    pub withdrawal_id: u32, // only for withdrawal
    /// tx state
    pub state: TxState,
}

/// application for withdrawal
#[derive(PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct Application<AccountId, Balance, Moment> {
    pub id: u32,
    pub applicant: AccountId,
    pub token: Token,
    pub balance: Balance,
    pub addr: AddrStr,
    pub ext: Memo,
    pub time: Moment,
}

impl<AccountId, Balance, Moment> Application<AccountId, Balance, Moment>
where
    AccountId: Codec + Clone,
    Balance: Codec + Copy + Clone,
    Moment: Codec + Clone,
{
    pub fn new(
        id: u32,
        applicant: AccountId,
        token: Token,
        balance: Balance,
        addr: AddrStr,
        ext: Memo,
        time: Moment,
    ) -> Self {
        Application::<AccountId, Balance, Moment> {
            id,
            applicant,
            token,
            balance,
            addr,
            ext,
            time,
        }
    }
    pub fn id(&self) -> u32 {
        self.id
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
    pub fn time(&self) -> Moment {
        self.time.clone()
    }
}

impl<AccountId, Balance, Moment> NodeT for Application<AccountId, Balance, Moment> {
    type Index = u32;
    fn index(&self) -> Self::Index {
        self.id
    }
}

pub struct LinkedMultiKey<T: Trait>(support::storage::generator::PhantomData<T>);

impl<T: Trait> LinkedNodeCollection for LinkedMultiKey<T> {
    type Header = ApplicationMHeader<T>;
    type NodeMap = ApplicationMap<T>;
    type Tail = ApplicationMTail<T>;
}
