// Copyright 2019-2023 ChainX Project Authors. Licensed under GPL-3.0.

//! RPC interface for the DEX Spot module.

#![allow(clippy::type_complexity)]

use std::fmt::{Debug, Display};
use std::str::FromStr;
use std::sync::Arc;

use codec::Codec;
use jsonrpc_derive::rpc;
use serde::{Deserialize, Serialize};

use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{generic::BlockId, traits::Block as BlockT};

use xp_rpc::{runtime_error_into_rpc_err, Result, RpcBalance, RpcPrice};

use xpallet_dex_spot_rpc_runtime_api::{
    Depth, FullPairInfo, Handicap, OrderProperty, RpcOrder, TradingPairId, TradingPairInfo,
    XSpotApi as XSpotRuntimeApi,
};

/// XSpot RPC methods.
#[rpc]
pub trait XSpotApi<BlockHash, AccountId, Balance, BlockNumber, Price>
where
    Balance: Display + FromStr,
    Price: Display + FromStr,
{
    /// Get the overall info of all trading pairs.
    #[rpc(name = "xspot_getTradingPairs")]
    fn trading_pairs(
        &self,
        at: Option<BlockHash>,
    ) -> Result<Vec<FullPairInfo<RpcPrice<Price>, BlockNumber>>>;

    /// Get the orders of an account.
    #[rpc(name = "xspot_getOrdersByAccount")]
    fn orders(
        &self,
        who: AccountId,
        page_index: u32,
        page_size: u32,
        at: Option<BlockHash>,
    ) -> Result<
        Page<
            Vec<
                RpcOrder<
                    TradingPairId,
                    AccountId,
                    RpcBalance<Balance>,
                    RpcPrice<Price>,
                    BlockNumber,
                >,
            >,
        >,
    >;

    /// Get the depth of a trading pair.
    #[rpc(name = "xspot_getDepth")]
    fn depth(
        &self,
        pair_id: TradingPairId,
        depth_size: u32,
        at: Option<BlockHash>,
    ) -> Result<Option<Depth<RpcPrice<Price>, RpcBalance<Balance>>>>;
}

/// A struct that implements the [`XSpotApi`].
pub struct XSpot<C, B> {
    client: Arc<C>,
    _marker: std::marker::PhantomData<B>,
}

impl<C, B> XSpot<C, B> {
    /// Create new `Contracts` with the given reference to the client.
    pub fn new(client: Arc<C>) -> Self {
        Self {
            client,
            _marker: Default::default(),
        }
    }
}

impl<C, Block, AccountId, Balance, BlockNumber, Price>
    XSpotApi<<Block as BlockT>::Hash, AccountId, Balance, BlockNumber, Price> for XSpot<C, Block>
where
    Block: BlockT,
    C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
    C::Api: XSpotRuntimeApi<Block, AccountId, Balance, BlockNumber, Price>,
    AccountId: Codec,
    Balance: Codec + Display + FromStr,
    BlockNumber: Codec,
    Price: Codec + Display + FromStr,
{
    fn trading_pairs(
        &self,
        at: Option<<Block as BlockT>::Hash>,
    ) -> Result<Vec<FullPairInfo<RpcPrice<Price>, BlockNumber>>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));
        api.trading_pairs(&at)
            .map(|trading_pairs| {
                trading_pairs
                    .into_iter()
                    .map(
                        |trading_pairs| FullPairInfo::<RpcPrice<Price>, BlockNumber> {
                            profile: trading_pairs.profile,
                            handicap: Handicap {
                                highest_bid: trading_pairs.handicap.highest_bid.into(),
                                lowest_ask: trading_pairs.handicap.lowest_ask.into(),
                            },
                            pair_info: TradingPairInfo {
                                latest_price: trading_pairs.pair_info.latest_price.into(),
                                last_updated: trading_pairs.pair_info.last_updated,
                            },
                            max_valid_bid: trading_pairs.max_valid_bid.into(),
                            min_valid_ask: trading_pairs.min_valid_ask.into(),
                        },
                    )
                    .collect::<Vec<_>>()
            })
            .map_err(runtime_error_into_rpc_err)
    }

    fn orders(
        &self,
        who: AccountId,
        page_index: u32,
        page_size: u32,
        at: Option<<Block as BlockT>::Hash>,
    ) -> Result<
        Page<
            Vec<
                RpcOrder<
                    TradingPairId,
                    AccountId,
                    RpcBalance<Balance>,
                    RpcPrice<Price>,
                    BlockNumber,
                >,
            >,
        >,
    > {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));
        let data = api
            .orders(&at, who, page_index, page_size)
            .map(|orders| {
                orders
                    .into_iter()
                    .map(|order| RpcOrder {
                        props: OrderProperty {
                            id: order.props.id,
                            side: order.props.side,
                            price: order.props.price.into(),
                            amount: order.props.amount.into(),
                            pair_id: order.props.pair_id,
                            submitter: order.props.submitter,
                            order_type: order.props.order_type,
                            created_at: order.props.created_at,
                        },
                        status: order.status,
                        remaining: order.remaining.into(),
                        executed_indices: order.executed_indices,
                        already_filled: order.already_filled.into(),
                        reserved_balance: order.reserved_balance.into(),
                        last_update_at: order.last_update_at,
                    })
                    .collect::<Vec<_>>()
            })
            .map_err(runtime_error_into_rpc_err)?;
        Ok(Page {
            page_index,
            page_size,
            data,
        })
    }

    fn depth(
        &self,
        pair_id: TradingPairId,
        depth_size: u32,
        at: Option<<Block as BlockT>::Hash>,
    ) -> Result<Option<Depth<RpcPrice<Price>, RpcBalance<Balance>>>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));
        match api.depth(&at, pair_id, depth_size) {
            Ok(Some(depth)) => {
                let asks = depth
                    .asks
                    .into_iter()
                    .map(|(price, quantity)| (price.into(), quantity.into()))
                    .collect::<Vec<_>>();
                let bids = depth
                    .bids
                    .into_iter()
                    .map(|(price, quantity)| (price.into(), quantity.into()))
                    .collect::<Vec<_>>();
                Ok(Some(Depth { asks, bids }))
            }
            Ok(None) => Ok(None),
            Err(err) => Err(runtime_error_into_rpc_err(err)),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Page<T> {
    pub page_index: u32,
    pub page_size: u32,
    pub data: T,
}
