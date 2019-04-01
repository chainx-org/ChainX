// Copyright 2018-2019 Chainpool.

use parity_codec::{Decode, Encode};
#[cfg(feature = "std")]
use serde_derive::{Deserialize, Serialize};

// Substrate
use rstd::prelude::*;

#[derive(PartialEq, Eq, Clone, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub enum AddrType {
    Normal,
    Root,
    Trustee,
}

impl Default for AddrType {
    fn default() -> Self {
        AddrType::Normal
    }
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub struct AddrInfo<AccountId> {
    pub addr_type: AddrType,
    pub required_num: u32,
    pub owner_list: Vec<(AccountId, bool)>,
}

// struct for the status of a pending operation.
#[derive(PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub struct PendingState<Proposal> {
    pub yet_needed: u32,
    pub owners_done: u32,
    pub proposal: Box<Proposal>,
}
