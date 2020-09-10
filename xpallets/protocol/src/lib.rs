// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

//! Some protocol details in the ChainX.

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(missing_docs)]

mod asset;
mod network;

pub use self::asset::*;
pub use self::network::*;

/// The maximum length of asset symbol
pub const ASSET_SYMBOL_MAX_LEN: usize = 24;
/// The maximum length of asset name
pub const ASSET_NAME_MAX_LEN: usize = 48;
/// The maximum length of asset description
pub const ASSET_DESC_MAX_LEN: usize = 128;
/// The maximum length of memo
pub const MEMO_MAX_LEN: usize = 80;
