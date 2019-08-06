// Copyright 2018-2019 Chainpool.

#![cfg_attr(not(feature = "std"), no_std)]

use rstd::collections::btree_map::BTreeMap;
use rstd::prelude::Vec;
use sr_primitives::traits::AuthorityIdFor;

use client::decl_runtime_apis;

use chainx_primitives::{AccountIdForApi, Balance, BlockNumber, Timestamp};

pub mod xassets_api {
    use super::*;
    use rstd::collections::btree_map::BTreeMap;
    use xassets::{Asset, AssetType, Memo, Token};
    use xprocess::WithdrawalLimit;
    use xr_primitives::AddrStr;

    decl_runtime_apis! {
        pub trait XAssetsApi {
            fn valid_assets() -> Vec<Token>;
            fn all_assets() -> Vec<(Asset, bool)>;
            fn valid_assets_of(who: AccountIdForApi) -> Vec<(Token, BTreeMap<AssetType, Balance>)>;
            fn withdrawal_list_of(chain: xassets::Chain) -> Vec<xrecords::RecordInfo<AccountIdForApi, Balance, BlockNumber, Timestamp>>;
            fn deposit_list_of(chain: xassets::Chain) -> Vec<xrecords::RecordInfo<AccountIdForApi, Balance, BlockNumber, Timestamp>>;
            fn verify_address(token: Token, addr: AddrStr, ext: Memo) -> Result<(), Vec<u8>>;
            fn withdrawal_limit(token: Token) -> Option<WithdrawalLimit<Balance>>;
        }
    }
}

pub mod xmining_api {
    use super::*;
    use xassets::Token;

    decl_runtime_apis! {
        pub trait XMiningApi {
            fn jackpot_accountid_for_unsafe(who: AccountIdForApi) -> AccountIdForApi;
            fn multi_jackpot_accountid_for_unsafe(who: Vec<AccountIdForApi>) -> Vec<AccountIdForApi>;
            fn token_jackpot_accountid_for_unsafe(token: Token) -> AccountIdForApi;
            fn multi_token_jackpot_accountid_for_unsafe(token: Vec<Token>) -> Vec<AccountIdForApi>;
            fn asset_power(token: Token) -> Option<Balance>;
        }
    }
}

pub mod xspot_api {
    use super::*;
    use xassets::Token;

    decl_runtime_apis! {
        pub trait XSpotApi {
            fn aver_asset_price(token: Token) -> Option<Balance>;
        }
    }
}

pub mod xfee_api {
    use super::*;

    decl_runtime_apis! {
        pub trait XFeeApi {
            fn transaction_fee(call: Vec<u8>, encoded_len: u64) -> Option<u64>;

            fn fee_weight_map() -> BTreeMap<Vec<u8>, u64>;
        }
    }
}

pub mod xsession_api {
    use super::*;

    decl_runtime_apis! {
        pub trait XSessionApi {
            fn pubkeys_for_validator_name(name: Vec<u8>) -> Option<(AccountIdForApi, Option<AuthorityIdFor<Block>>)>;
        }
    }
}

pub mod xstaking_api {
    use super::*;

    decl_runtime_apis! {
        pub trait XStakingApi {
            fn intention_set() -> Vec<AccountIdForApi>;
        }
    }
}

pub mod xbridge_api {
    use super::*;
    use xassets::Chain;
    use xbridge_common::types::{GenericAllSessionInfo, GenericTrusteeIntentionProps};
    decl_runtime_apis! {
        pub trait XBridgeApi {
            /// generate a mock trustee info
            fn mock_new_trustees(chain: Chain, candidates: Vec<AccountIdForApi>) -> Result<GenericAllSessionInfo<AccountIdForApi>, Vec<u8>>;

            fn trustee_props_for(who: AccountIdForApi) ->  BTreeMap<xassets::Chain, GenericTrusteeIntentionProps>;

            fn trustee_session_info() -> BTreeMap<xassets::Chain, GenericAllSessionInfo<AccountIdForApi>>;

            fn trustee_session_info_for(chain: Chain, number: Option<u32>) -> Option<(u32, GenericAllSessionInfo<AccountIdForApi>)>;
        }
    }
}
