//! Runtime API definition required by ChainX RPC extensions.

#![cfg_attr(not(feature = "std"), no_std)]

use sp_std::collections::btree_map::BTreeMap;

pub use xpallet_assets::Chain;
pub use xpallet_gateway_common::{
    trustees,
    types::{GenericTrusteeIntentionProps, GenericTrusteeSessionInfo},
};

sp_api::decl_runtime_apis! {
    /// The API to query account nonce (aka transaction index).
    pub trait XGatewayCommonApi<AccountId> where
        AccountId: codec::Codec,
    {
        /// Get all trustee multisig.
        fn trustee_multisigs() -> BTreeMap<Chain, AccountId>;

        fn trustee_properties(chain: Chain, who: AccountId) -> Option<GenericTrusteeIntentionProps>;

        fn trustee_session_info(chain: Chain) -> Option<GenericTrusteeSessionInfo<AccountId>>;
    }
}
