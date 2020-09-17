// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use super::*;
use codec::{Decode, Encode};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::{traits::Saturating, RuntimeDebug};

#[derive(PartialEq, Eq, Clone, Default, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct FullPairInfo<Price, BlockNumber> {
    #[cfg_attr(feature = "std", serde(flatten))]
    pub profile: TradingPairProfile,
    #[cfg_attr(feature = "std", serde(flatten))]
    pub handicap: Handicap<Price>,
    #[cfg_attr(feature = "std", serde(flatten))]
    pub pair_info: TradingPairInfo<Price, BlockNumber>,
    /// The maximum valid bid price.
    pub max_valid_bid: Price,
    /// The minimum valid ask price.
    pub min_valid_ask: Price,
}

/// Details of an order.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct RpcOrder<PairId, AccountId, Balance, Price, BlockNumber> {
    /// Immutable details of the order.
    #[cfg_attr(feature = "std", serde(flatten))]
    pub props: OrderProperty<PairId, AccountId, Balance, Price, BlockNumber>,
    /// Status of the order.
    pub status: OrderStatus,
    /// The amount of unexecuted, measured by the **quote** currency.
    ///
    /// While (props.amount() - already_filled) can be expressed as
    /// the remaining amount as well, it's measured by the base currency.
    pub remaining: Balance,
    /// Indices of all executed transaction records.
    pub executed_indices: Vec<TradingHistoryIndex>,
    /// The amount of executed, measured by the **base** currency.
    pub already_filled: Balance,
    /// Current locked asset balance in this order.
    pub reserved_balance: Balance,
    /// Block number at which the order details updated.
    pub last_update_at: BlockNumber,
}

#[derive(PartialEq, Eq, Clone, Default, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct Depth<Price, Balance> {
    /// List of asks in pair of (price, quantity).
    pub asks: Vec<(Price, Balance)>,
    /// List of bids in pair of (price, quantity).
    pub bids: Vec<(Price, Balance)>,
}

impl<T: Trait> Module<T> {
    /// Returns the range of a valid quotation for a trading pair.
    fn get_quotation_range(profile: &TradingPairProfile) -> (T::Price, T::Price) {
        let handicap = Self::handicap_of(profile.id);
        let pair_fluctuation: T::Price = profile.calc_fluctuation::<T>().saturated_into();
        let max_valid_bid = if !handicap.lowest_ask.is_zero() {
            handicap.lowest_ask + pair_fluctuation
        } else {
            Zero::zero()
        };
        let min_valid_ask = if handicap.highest_bid > pair_fluctuation {
            handicap.highest_bid - pair_fluctuation
        } else {
            profile.tick().saturated_into()
        };
        (min_valid_ask, max_valid_bid)
    }

    /// Get the overall info of all trading pairs.
    pub fn trading_pairs() -> Vec<FullPairInfo<T::Price, T::BlockNumber>> {
        let pair_count = Self::trading_pair_count();
        let mut pairs = Vec::with_capacity(pair_count as usize);
        for pair_id in 0..pair_count {
            if let Some(profile) = Self::trading_pair_of(pair_id) {
                let (min_valid_ask, max_valid_bid) = Self::get_quotation_range(&profile);
                let handicap = Self::handicap_of(pair_id);
                let pair_info: TradingPairInfo<T::Price, T::BlockNumber> =
                    Self::trading_pair_info_of(pair_id).unwrap_or_default();
                pairs.push(FullPairInfo {
                    profile,
                    handicap,
                    pair_info,
                    max_valid_bid,
                    min_valid_ask,
                });
            }
        }
        pairs
    }

    /// Get the orders of an account.
    ///
    /// The returned data will be empty if `page_index` is invalid.
    ///
    /// FIXME: page_size should be limited.
    #[allow(clippy::type_complexity)]
    pub fn orders(
        who: T::AccountId,
        page_index: u32,
        page_size: u32,
    ) -> Vec<RpcOrder<TradingPairId, T::AccountId, BalanceOf<T>, T::Price, T::BlockNumber>> {
        OrderInfoOf::<T>::iter_prefix_values(who)
            .flat_map(|order| {
                Self::trading_pair(order.pair_id())
                    .ok()
                    .and_then(|pair| match order.props.side {
                        Side::Buy => Self::convert_base_to_quote(
                            order.remaining_in_base(),
                            order.props.price,
                            &pair,
                        )
                        .ok(),
                        Side::Sell => Some(order.remaining),
                    })
                    .map(|reserved_balance| RpcOrder {
                        props: order.props,
                        status: order.status,
                        remaining: order.remaining,
                        executed_indices: order.executed_indices,
                        already_filled: order.already_filled,
                        reserved_balance,
                        last_update_at: order.last_update_at,
                    })
            })
            .skip((page_index * page_size) as usize)
            .take(page_size as usize)
            .collect()
    }

    /// Returns the sum of unfilled quantities at `price` of a trading pair `pair_id`.
    fn get_commulative_qty(pair_id: TradingPairId, price: T::Price) -> u128 {
        QuotationsOf::<T>::get(pair_id, price)
            .iter()
            .filter_map(|(trader, order_id)| OrderInfoOf::<T>::get(trader, order_id))
            .map(|order| {
                order
                    .amount()
                    .saturating_sub(order.already_filled)
                    .saturated_into::<u128>()
            })
            .sum()
    }

    /// Get the depth of a trading pair around the handicap given the depth size.
    #[allow(clippy::type_complexity)]
    pub fn depth(pair_id: TradingPairId, depth_size: u32) -> Option<Depth<T::Price, BalanceOf<T>>> {
        Self::trading_pair_of(pair_id).map(|pair| {
            let Handicap {
                lowest_ask,
                highest_bid,
            } = Self::handicap_of(pair_id);

            let (min_valid_ask, max_valid_bid) = Self::get_quotation_range(&pair);

            let step = pair.tick().saturated_into::<u128>();

            let generic_depth = |start: T::Price, end: T::Price| {
                let start = start.saturated_into::<u128>();
                let end = end.saturated_into::<u128>();
                (0..)
                    .map(|x| start + step * x)
                    .take_while(|&x| x <= end)
                    .filter_map(|price| {
                        let cummulative_qty =
                            Self::get_commulative_qty(pair_id, price.saturated_into());
                        if cummulative_qty.is_zero() {
                            None
                        } else {
                            let price: T::Price = price.saturated_into();
                            let cummulative_qty: BalanceOf<T> = cummulative_qty.saturated_into();
                            Some((price, cummulative_qty))
                        }
                    })
                    .take(depth_size as usize)
                    .collect::<Vec<_>>()
            };

            let asks = generic_depth(lowest_ask, max_valid_bid);
            let bids = generic_depth(min_valid_ask, highest_bid);

            Depth { asks, bids }
        })
    }
}

#[cfg(test)]
mod rpc_tests {
    use super::*;
    use crate::mock::*;
    use crate::tests::{t_issue_pcx, t_put_order_sell, t_set_handicap};
    use frame_support::assert_ok;

    #[test]
    fn rpc_depth_should_work() {
        ExtBuilder::default().build_and_execute(|| {
            let pair_id = 0;
            let who = 1;

            t_set_handicap(pair_id, 1_000_000, 1_100_000);

            t_issue_pcx(who, 1000);
            // The depth does not count this order in.
            assert_ok!(t_put_order_sell(who, pair_id, 100, 1_210_000));
            assert_ok!(t_put_order_sell(who, pair_id, 100, 1_109_000));
            assert_ok!(t_put_order_sell(who, pair_id, 200, 1_108_000));

            assert_eq!(XSpot::depth(pair_id, 100).unwrap(), {
                Depth {
                    asks: vec![(1_108_000, 200), (1_109_000, 100)],
                    bids: vec![],
                }
            });
        });
    }
}
