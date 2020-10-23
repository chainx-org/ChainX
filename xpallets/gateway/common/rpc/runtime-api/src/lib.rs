// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

//! Runtime API definition required by ChainX RPC extensions.

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::too_many_arguments, clippy::unnecessary_mut_passed)]

use sp_std::{collections::btree_map::BTreeMap, prelude::*};

use sp_runtime::DispatchError;

pub use chainx_primitives::{AddrStr, AssetId, ChainAddress};
pub use xp_runtime::Memo;

pub use xpallet_assets::{Chain, WithdrawalLimit};
pub use xpallet_gateway_common::{
    trustees,
    types::{GenericTrusteeIntentionProps, GenericTrusteeSessionInfo},
};

sp_api::decl_runtime_apis! {
    /// The API to query account nonce (aka transaction index).
    pub trait XGatewayCommonApi<AccountId, Balance>
    where
        AccountId: codec::Codec,
        Balance: codec::Codec,
    {
        fn bound_addrs(who: AccountId) -> BTreeMap<Chain, Vec<ChainAddress>>;

        fn withdrawal_limit(asset_id: AssetId) -> Result<WithdrawalLimit<Balance>, DispatchError>;

        fn verify_withdrawal(asset_id: AssetId, value: Balance, addr: AddrStr, memo: Memo) -> Result<(), DispatchError>;

        /// Get all trustee multisig.
        fn trustee_multisigs() -> BTreeMap<Chain, AccountId>;

        fn trustee_properties(chain: Chain, who: AccountId) -> Option<GenericTrusteeIntentionProps>;

        fn trustee_session_info(chain: Chain) -> Option<GenericTrusteeSessionInfo<AccountId>>;

        fn generate_trustee_session_info(chain: Chain, Vec<AccountId>) -> Result<GenericTrusteeSessionInfo<AccountId>, DispatchError>;
    }
}
