// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

//! This module handles the internal state of spot, mainly the handicap and the quotation list.

use super::*;
use crate::types::Side::{Buy, Sell};
use sp_runtime::traits::{CheckedAdd, CheckedSub};

/// Internal mutables
impl<T: Config> Module<T> {
    /// It's worth noting that the handicap is not always related to some real orders, i.e.,
    /// current lowest_ask(or highest_bid) is suprious.
    ///
    /// When there is no quotions at a certain price given the trading pair, we should check out
    /// whether the current handicap is true. If it's not true, adjust a tick accordingly.
    pub(super) fn update_handicap(pair: &TradingPairProfile, price: T::Price, side: Side) {
        let tick_decimals = pair.tick_decimals;

        if <QuotationsOf<T>>::get(pair.id, price).is_empty() {
            let mut handicap = <HandicapOf<T>>::get(pair.id);
            match side {
                Side::Sell => {
                    if !handicap.lowest_ask.is_zero()
                        && <QuotationsOf<T>>::get(pair.id, handicap.lowest_ask).is_empty()
                    {
                        handicap.tick_up_lowest_ask(tick_decimals);
                        <HandicapOf<T>>::insert(pair.id, &handicap);

                        debug!(
                            target: "runtime::dex::spot",
                            "[update_handicap] pair_index: {:?}, lowest_ask: {:?}, side: {:?}",
                            pair.id, handicap.lowest_ask, Sell,
                        );
                    }
                }
                Side::Buy => {
                    if !handicap.highest_bid.is_zero()
                        && <QuotationsOf<T>>::get(pair.id, handicap.highest_bid).is_empty()
                    {
                        handicap.tick_down_highest_bid(tick_decimals);
                        <HandicapOf<T>>::insert(pair.id, &handicap);

                        debug!(
                            target: "runtime::dex::spot",
                            "[update_handicap] pair_index: {:?}, highest_bid: {:?}, side: {:?}",
                            pair.id, handicap.highest_bid, Buy
                        );
                    }
                }
            };
        };
    }

    pub(super) fn update_handicap_after_matching_order(
        pair: &TradingPairProfile,
        order: &mut OrderInfo<T>,
    ) {
        match order.side() {
            Side::Buy => Self::update_handicap_of_buyers(pair, order),
            Side::Sell => Self::update_handicap_of_sellers(pair, order),
        }
    }

    pub(super) fn tick_up(v: T::Price, tick: Tick) -> T::Price {
        match v.checked_add(&tick.saturated_into()) {
            Some(x) => x,
            None => panic!("Fail to tick up"),
        }
    }

    /// This is only used for updating the handicap. Return zero when underflow.
    pub(super) fn tick_down(v: T::Price, tick: Tick) -> T::Price {
        v.checked_sub(&tick.saturated_into())
            .unwrap_or_else(Zero::zero)
    }

    fn update_handicap_of_buyers(pair: &TradingPairProfile, order: &mut OrderInfo<T>) {
        HandicapOf::<T>::mutate(pair.id, |handicap| {
            let order_price = order.price();

            if order_price > handicap.highest_bid || handicap.highest_bid.is_zero() {
                let new_highest_bid = order_price;

                if new_highest_bid >= handicap.lowest_ask {
                    handicap.lowest_ask = Self::tick_up(new_highest_bid, pair.tick());
                    debug!(
                        target: "runtime::dex::spot",
                        "[update_handicap] pair_id: {:?}, lowest_ask: {:?}, side: {:?}",
                        order.pair_id(),
                        handicap.lowest_ask,
                        Side::Sell,
                    );
                }

                handicap.highest_bid = new_highest_bid;
                debug!(
                    target: "runtime::dex::spot",
                    "[update_handicap] pair_id: {:?}, highest_bid: {:?}, side: {:?}",
                    order.pair_id(),
                    new_highest_bid,
                    Side::Buy
                );
            }
        });
    }

    fn update_handicap_of_sellers(pair: &TradingPairProfile, order: &mut OrderInfo<T>) {
        HandicapOf::<T>::mutate(pair.id, |handicap| {
            let order_price = order.price();

            if order_price < handicap.lowest_ask || handicap.lowest_ask.is_zero() {
                let new_lowest_ask = order_price;

                if new_lowest_ask <= handicap.highest_bid {
                    handicap.highest_bid = Self::tick_down(new_lowest_ask, pair.tick());
                    debug!(
                        target: "runtime::dex::spot",
                        "[update_handicap] pair_id: {:?}, highest_bid: {:?}, side: {:?}",
                        order.pair_id(),
                        handicap.highest_bid,
                        Side::Buy
                    );
                }

                handicap.lowest_ask = new_lowest_ask;
                debug!(
                    target: "runtime::dex::spot",
                    "[update_handicap] pair_id: {:?}, lowest_ask: {:?}, side: {:?}",
                    order.pair_id(),
                    new_lowest_ask,
                    Side::Sell,
                );
            }
        });
    }

    /// Removes the order as well as the quotations from the order list.
    ///
    /// This happens when the maker orders have been completely filled.
    pub(super) fn remove_orders_and_quotations(
        pair_id: TradingPairId,
        price: T::Price,
        fulfilled_orders: Vec<(T::AccountId, OrderId)>,
    ) {
        debug!(
            target: "runtime::dex::spot",
            "[remove_orders_and_quotations] These fulfilled orders will be removed: {:?}",
            fulfilled_orders
        );
        for (who, order_idx) in fulfilled_orders.iter() {
            <OrderInfoOf<T>>::remove(who, order_idx);
        }

        <QuotationsOf<T>>::mutate(pair_id, price, |quotations| {
            quotations.retain(|i| !fulfilled_orders.contains(i));
        });
    }

    /// Removes the quotation only.
    ///
    /// This happens when the order is killed.
    pub(super) fn remove_quotation(
        pair_id: TradingPairId,
        price: T::Price,
        order_key: (T::AccountId, OrderId),
    ) {
        <QuotationsOf<T>>::mutate(pair_id, price, |quotations| {
            if let Some(idx) = quotations.iter().position(|i| i == &order_key) {
                // NOTE: Can't use swap_remove since the original order must be preserved.
                let _removed = quotations.remove(idx);
                debug!(
                    target: "runtime::dex::spot",
                    "[remove_quotation] (who, order_index): {:?}, removed order: {:?}",
                    order_key, _removed
                );
            }
        });
    }

    /// Updates the latest price of a trading pair.
    ///
    /// This happens after an order is executed every time.
    pub(crate) fn update_latest_price(pair_index: TradingPairId, latest: T::Price) {
        let current_block = <frame_system::Pallet<T>>::block_number();

        <TradingPairInfoOf<T>>::insert(
            pair_index,
            TradingPairInfo {
                latest_price: latest,
                last_updated: current_block,
            },
        );
    }
}
