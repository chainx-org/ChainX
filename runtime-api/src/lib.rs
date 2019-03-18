// Copyright 2018 Chainpool.
#![cfg_attr(not(feature = "std"), no_std)]

use rstd::prelude::Vec;
use sr_primitives::traits::AuthorityIdFor;

use client::decl_runtime_apis;

use chainx_primitives::{AccountIdImpl, Balance, Timestamp};

pub mod xassets_api {
    use super::*;
    use xassets::{Asset, AssetType, Memo, Token};
    use xrecords::AddrStr;
    use xsupport::storage::btree_map::CodecBTreeMap;

    decl_runtime_apis! {
        pub trait XAssetsApi {
            fn valid_assets() -> Vec<Token>;
            fn all_assets() -> Vec<(Asset, bool)>;
            fn valid_assets_of(who: AccountIdImpl) -> Vec<(Token, CodecBTreeMap<AssetType, Balance>)>;
            fn withdrawal_list_of(chain: xassets::Chain) -> Vec<xrecords::RecordInfo<AccountIdImpl, Balance, Timestamp>>;
            fn deposit_list_of(chain: xassets::Chain) -> Vec<xrecords::RecordInfo<AccountIdImpl, Balance, Timestamp>>;
            fn verify_address(token: Token, addr: AddrStr, ext: Memo) -> Result<(), Vec<u8>>;
            fn minimal_withdrawal_value(token: Token) -> Option<Balance>;
        }
    }
}

pub mod xmining_api {
    use super::*;
    use xassets::Token;

    decl_runtime_apis! {
        pub trait XMiningApi {
            fn jackpot_accountid_for(who: AccountIdImpl) -> AccountIdImpl;
            fn multi_jackpot_accountid_for(who: Vec<AccountIdImpl>) -> Vec<AccountIdImpl>;
            fn token_jackpot_accountid_for(token: Token) -> AccountIdImpl;
            fn multi_token_jackpot_accountid_for(token: Vec<Token>) -> Vec<AccountIdImpl>;
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
        }
    }
}

pub mod xsession_api {
    use super::*;

    decl_runtime_apis! {
        pub trait XSessionApi {
            fn pubkeys_for_validator_name(name: Vec<u8>) -> Option<(AccountIdImpl, Option<AuthorityIdFor<Block>>)>;
        }
    }
}
