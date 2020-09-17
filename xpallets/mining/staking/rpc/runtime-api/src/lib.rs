// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

//! Runtime API definition required by ChainX RPC extensions.

#![cfg_attr(not(feature = "std"), no_std)]

use sp_std::{collections::btree_map::BTreeMap, prelude::*};

use codec::Codec;

pub use xpallet_mining_staking::{NominatorInfo, NominatorLedger, ValidatorInfo};

sp_api::decl_runtime_apis! {
    /// The API to query Staking info.
    pub trait XStakingApi<AccountId, Balance, BlockNumber>
    where
        AccountId: Codec + Ord,
        Balance: Codec,
        BlockNumber: Codec,
    {
        /// Get overall information about all potential validators.
        fn validators() -> Vec<ValidatorInfo<AccountId, Balance, BlockNumber>>;

        /// Get overall information given the validator AccountId.
        fn validator_info_of(who: AccountId) -> ValidatorInfo<AccountId, Balance, BlockNumber>;

        /// Get the staking dividends info given the staker AccountId.
        fn staking_dividend_of(who: AccountId) -> BTreeMap<AccountId, Balance>;

        /// Get the nomination details given the staker AccountId.
        fn nomination_details_of(who: AccountId) -> BTreeMap<AccountId, NominatorLedger<Balance, BlockNumber>>;

        /// Get individual nominator information given the nominator AccountId.
        fn nominator_info_of(who: AccountId) -> NominatorInfo<BlockNumber>;
    }
}
