//! Runtime API definition for transaction fee module.

#![cfg_attr(not(feature = "std"), no_std)]

use codec::Codec;
use sp_runtime::traits::{MaybeDisplay, MaybeFromStr};

pub use pallet_transaction_payment_rpc_runtime_api::RuntimeDispatchInfo;

sp_api::decl_runtime_apis! {
    pub trait TransactionFeeApi<Balance> where
        Balance: Codec + MaybeDisplay + MaybeFromStr,
    {
        fn query_detailed_info(uxt: Block::Extrinsic, len: u32) -> RuntimeDispatchInfo<Balance>;
    }
}
