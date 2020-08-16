// Copyright 2018-2019 Chainpool.
//! System manager: Handles all of the top-level stuff; executing block/transaction, setting code
//! and depositing logs.

#![allow(clippy::type_complexity)]
#![allow(clippy::match_overlapping_arm)]
#![allow(clippy::block_in_if_condition_stmt)]
#![cfg_attr(not(feature = "std"), no_std)]

pub mod generic;
pub mod traits;

use parity_codec::{Decode, Encode};
use rstd::prelude::Vec;

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

pub type XString = Vec<u8>;
pub type Name = XString;
pub type URL = XString;
pub type AddrStr = XString;
pub type Memo = XString;
pub type Token = XString;
pub type Desc = XString;

/// A result of execution of a contract.
#[derive(Eq, PartialEq, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub enum ContractExecResult {
    /// The contract returned successfully.
    ///
    /// There is a status code and, optionally, some data returned by the contract.
    Success {
        /// Status code returned by the contract.
        status: u16,
        /// Output data returned by the contract.
        ///
        /// Can be empty.
        data: Vec<u8>,
    },
    /// The contract execution either trapped or returned an error.
    Error(Vec<u8>),
}

/// A result type of the get storage call.
///
/// See [`ContractsApi::get_storage`] for more info.
pub type GetStorageResult = Result<Option<Vec<u8>>, GetStorageError>;
/// The possible errors that can happen querying the storage of a contract.
#[derive(Eq, PartialEq, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub enum GetStorageError {
    /// The given address doesn't point on a contract.
    ContractDoesntExist,
    /// The specified contract is a tombstone and thus cannot have any storage.
    IsTombstone,
}

#[cfg(feature = "std")]
impl std::fmt::Display for GetStorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "{:?}", self)
    }
}

#[derive(PartialEq, PartialOrd, Ord, Eq, Clone, Copy, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub enum XRC20Selector {
    BalanceOf,
    TotalSupply,
    Name,
    Symbol,
    Decimals,
    Issue,
    Destroy,
}
