//! Runtime API definition required by ChainX RPC extensions.

#![cfg_attr(not(feature = "std"), no_std)]

use sp_std::prelude::*;

sp_api::decl_runtime_apis! {
    /// The API to query account nonce (aka transaction index).
    pub trait XStakingApi<AccountId> where
        AccountId: codec::Codec,
    {
        /// Get all potential validators.
        fn validators() -> Vec<AccountId>;
    }
}
