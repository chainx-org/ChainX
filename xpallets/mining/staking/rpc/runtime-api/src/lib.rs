//! Runtime API definition required by ChainX RPC extensions.

#![cfg_attr(not(feature = "std"), no_std)]

use codec::Codec;
use sp_std::collections::btree_map::BTreeMap;
use sp_std::prelude::*;
use xpallet_mining_staking::{NominatorInfo, RpcNominatorLedger, ValidatorInfo};
use xpallet_support::RpcBalance;

sp_api::decl_runtime_apis! {
    /// The API to query Staking info.
    pub trait XStakingApi<AccountId, Balance, BlockNumber> where
        AccountId: Codec + Ord,
        Balance: Codec,
        BlockNumber: Codec,
    {
        /// Get overall information about all potential validators.
        fn validators() -> Vec<ValidatorInfo<AccountId, RpcBalance<Balance>, BlockNumber>>;

        /// Get overall information given the validator AccountId.
        fn validator_info_of(who: AccountId) -> ValidatorInfo<AccountId, RpcBalance<Balance>, BlockNumber>;

        /// Get the staking dividends info given the staker AccountId.
        fn staking_dividend_of(who: AccountId) -> BTreeMap<AccountId, RpcBalance<Balance>>;

        /// Get the nomination details given the staker AccountId.
        fn nomination_details_of(who: AccountId) -> BTreeMap<AccountId, RpcNominatorLedger<RpcBalance<Balance>, BlockNumber>>;

        /// Get individual nominator information given the nominator AccountId.
        fn nominator_info_of(who: AccountId) -> NominatorInfo<RpcBalance<Balance>, BlockNumber>;
    }
}
