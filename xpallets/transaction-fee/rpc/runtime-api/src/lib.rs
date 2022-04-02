// Copyright 2019-2022 ChainX Project Authors. Licensed under GPL-3.0.

//! Runtime API definition for transaction fee module.

#![cfg_attr(not(feature = "std"), no_std)]

use codec::Codec;
use sp_runtime::traits::{MaybeDisplay, MaybeFromStr};

pub use xpallet_transaction_fee::{FeeDetails, InclusionFee};

sp_api::decl_runtime_apis! {
    pub trait XTransactionFeeApi<Balance> where
        Balance: Codec + MaybeDisplay + MaybeFromStr,
    {
        fn query_fee_details(uxt: Block::Extrinsic, len: u32) -> FeeDetails<Balance>;
    }
}
