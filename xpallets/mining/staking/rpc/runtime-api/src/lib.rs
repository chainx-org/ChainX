//! Runtime API definition required by ChainX RPC extensions.

#![cfg_attr(not(feature = "std"), no_std)]

use codec::Codec;
use sp_std::prelude::*;
use xpallet_mining_staking::ValidatorInfo;

sp_api::decl_runtime_apis! {
    /// The API to query account nonce (aka transaction index).
    pub trait XStakingApi<AccountId, Balance, BlockNumber> where
        AccountId: Codec,
        Balance: Codec,
        BlockNumber: Codec,
    {
        /// Get overall information about all potential validators.
        fn validators() -> Vec<ValidatorInfo<AccountId, Balance, BlockNumber>>;
    }
}
