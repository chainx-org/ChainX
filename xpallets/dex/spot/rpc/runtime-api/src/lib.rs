//! Runtime API definition required by ChainX RPC extensions.

#![cfg_attr(not(feature = "std"), no_std)]

use codec::Codec;
use sp_std::prelude::*;
use xpallet_dex_spot::{Depth, Order, PairInfo, TradingPairId};
use xpallet_support::{RpcBalance, RpcPrice};

sp_api::decl_runtime_apis! {
    /// The API to query DEX Spot info.
    pub trait XSpotApi<AccountId, Balance, BlockNumber, Price> where
        AccountId: Codec,
        Balance: Codec,
        BlockNumber: Codec,
        Price: Codec,
    {
        /// Get the overall info of all trading pairs.
        fn trading_pairs() -> Vec<PairInfo<RpcPrice<Price>>>;

        /// Get the orders of an account.
        fn orders(who: AccountId) -> Vec<Order<TradingPairId, AccountId, RpcBalance<Balance>, RpcPrice<Price>, BlockNumber>>;

        /// Get the depth of a trading pair.
        fn depth(pair_id: TradingPairId) -> Vec<Depth<RpcPrice<Price>, RpcBalance<Balance>>>;
    }
}
