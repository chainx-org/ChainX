// Copyright 2019-2023 ChainX Project Authors. Licensed under GPL-3.0.

//! Some primitives and utils about ChainX gateway bitcoin.

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(missing_docs)]
#![allow(clippy::type_complexity)]

mod detector;
mod extractor;
mod types;
mod utils;

pub use self::detector::BtcTxTypeDetector;
pub use self::extractor::{AccountExtractor, OpReturnExtractor};
pub use self::types::{BtcDepositInfo, BtcTxMetaType, BtcTxType, OpReturnAccount};
pub use self::utils::*;
