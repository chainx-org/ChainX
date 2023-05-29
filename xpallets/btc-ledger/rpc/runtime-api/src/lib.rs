// Copyright 2019-2023 ChainX Project Authors. Licensed under GPL-3.0.

#![cfg_attr(not(feature = "std"), no_std)]

use codec::Codec;

sp_api::decl_runtime_apis! {
    pub trait BtcLedgerApi<AccountId, Balance>
    where
        AccountId: Codec,
        Balance: Codec,
    {
        fn get_balance(who: AccountId) -> Balance;
        fn get_total() -> Balance;
    }
}
