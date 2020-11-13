// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

//! Some primitives and utils about ChainX gateway bitcoin.

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(missing_docs)]

mod detector;
mod extractor;
mod types;
mod utils;

pub use self::detector::BtcTxTypeDetector;
pub use self::extractor::OpReturnExtractor;
pub use self::types::{BtcTxType, DepositInfo, MetaTxType};
pub use self::utils::*;
