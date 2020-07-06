// Copyright 2019 Chainpool.
//! This module handles the internal state of spot, mainly the handicap and the quotation list.

use super::*;
use crate::types::Side::{Buy, Sell};
use sp_runtime::traits::{CheckedAdd, CheckedSub};

/// Internal mutables
impl<T: Trait> Module<T> {
    /// It's worth noting that the handicap is not always related to some real orders, i.e.,
    /// current lowest_offer(or highest_bid) is suprious.
    ///
    /// When there is no quotions at a certain price given the trading pair, we should check out
    /// whether the current handicap is true. If it's not true, adjust a tick accordingly.
    pub(super) fn update_handicap(pair: &TradingPairProfile, price: T::Price, side: Side) {
        let tick_precision = pair.tick_precision;

        if <QuotationsOf<T>>::get(pair.id, price).is_empty() {
            let mut handicap = <HandicapOf<T>>::get(pair.id);
            match side {
                Side::Sell => {
                    if !handicap.lowest_offer.is_zero()
                        && <QuotationsOf<T>>::get(pair.id, handicap.lowest_offer).is_empty()
                    {
                        handicap.tick_up_lowest_offer(tick_precision);
                        <HandicapOf<T>>::insert(pair.id, &handicap);

                        debug!(
                            "[update_handicap] pair_index: {:?}, lowest_offer: {:?}, side: {:?}",
                            pair.id, handicap.lowest_offer, Sell,
                        );
                    }
                }
                Side::Buy => {
                    if !handicap.highest_bid.is_zero()
                        && <QuotationsOf<T>>::get(pair.id, handicap.highest_bid).is_empty()
                    {
                        handicap.tick_down_highest_bid(tick_precision);
                        <HandicapOf<T>>::insert(pair.id, &handicap);

                        debug!(
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

    pub(super) fn tick_up(v: T::Price, tick: u64) -> T::Price {
        match v.checked_add(&tick.saturated_into()) {
            Some(x) => x,
            None => panic!("Fail to tick up"),
        }
    }

    /// This is only used for updating the handicap. Return zero when underflow.
    pub(super) fn tick_down(v: T::Price, tick: u64) -> T::Price {
        v.checked_sub(&tick.saturated_into())
            .unwrap_or_else(Zero::zero)
    }

    fn update_handicap_of_buyers(pair: &TradingPairProfile, order: &mut OrderInfo<T>) {
        let mut handicap = <HandicapOf<T>>::get(pair.id);
        if order.price() > handicap.highest_bid || handicap.highest_bid == Default::default() {
            let highest_bid = order.price();

            if highest_bid >= handicap.lowest_offer {
                handicap.lowest_offer = Self::tick_up(highest_bid, pair.tick());

                debug!(
                    "[update_handicap] pair_index: {:?}, lowest_offer: {:?}, side: {:?}",
                    order.pair_id(),
                    handicap.lowest_offer,
                    Side::Sell,
                );
            }

            handicap.highest_bid = highest_bid;
            <HandicapOf<T>>::insert(order.pair_id(), handicap);

            debug!(
                "[update_handicap] pair_index: {:?}, highest_bid: {:?}, side: {:?}",
                order.pair_id(),
                highest_bid,
                Side::Buy
            );
        }
    }

    fn update_handicap_of_sellers(pair: &TradingPairProfile, order: &mut OrderInfo<T>) {
        let mut handicap = <HandicapOf<T>>::get(pair.id);
        if order.price() < handicap.lowest_offer || handicap.lowest_offer == Default::default() {
            let lowest_offer = order.price();

            if lowest_offer <= handicap.highest_bid {
                handicap.highest_bid = Self::tick_down(lowest_offer, pair.tick());

                debug!(
                    "[update_handicap] pair_index: {:?}, highest_bid: {:?}, side: {:?}",
                    order.pair_id(),
                    handicap.highest_bid,
                    Side::Buy
                );
            }

            handicap.lowest_offer = lowest_offer;
            <HandicapOf<T>>::insert(order.pair_id(), handicap);

            debug!(
                "[update_handicap] pair_index: {:?}, lowest_offer: {:?}, side: {:?}",
                order.pair_id(),
                lowest_offer,
                Side::Sell,
            );
        }
    }

    /// This happens when the maker orders have been full filled.
    pub(super) fn remove_orders_and_quotations(
        pair_id: TradingPairId,
        price: T::Price,
        fulfilled_orders: Vec<(T::AccountId, OrderId)>,
    ) {
        debug!(
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
                    "[remove_quotation] (who, order_index): {:?}, removed order: {:?}",
                    order_key, _removed
                );
            }
        });
    }

    /// This happens after an order has been executed.
    pub(crate) fn update_latest_price(pair_index: TradingPairId, latest: T::Price) {
        let current_block = <system::Module<T>>::block_number();

        <TradingPairInfoOf<T>>::insert(
            pair_index,
            TradingPairInfo {
                latest_price: latest,
                last_updated: current_block,
            },
        );
    }
}
