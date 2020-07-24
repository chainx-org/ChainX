//! Runtime API definition required by ChainX RPC extensions.

#![cfg_attr(not(feature = "std"), no_std)]

use sp_runtime::DispatchError;

pub use xpallet_gateway_common::trustees::bitcoin::BTCTrusteeSessionInfo;

sp_api::decl_runtime_apis! {
    pub trait XGatewayBitcoinApi<AccountId> where
        AccountId: codec::Codec,
    {
        fn generate_trustee_info(candidates: Vec<AccountId>) -> Result<BTCTrusteeSessionInfo<AccountId>, DispatchError>;
    }
}
