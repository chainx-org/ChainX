// Copyright 2019 Chainpool.
//! This module handles the internal state of spot, mainly the handicap and the quotation list.

use super::*;
use primitives::traits::{CheckedAdd, CheckedSub};

/// Internal mutables
impl<T: Trait> Module<T> {
    /// It's worth noting that the handicap is not always related to some real orders, i.e.,
    /// current lowest_offer(or highest_bid) is suprious.
    ///
    /// When there is no quotions at a certain price given the trading pair, we should check out
    /// whether the current handicap is true. If it's not true, adjust a tick accordingly.
    pub(super) fn update_handicap(pair: &TradingPair, price: T::Price, side: Side) {
        let tick_precision = pair.tick_precision;

        if <QuotationsOf<T>>::get(&(pair.index, price)).is_empty() {
            let mut handicap = <HandicapOf<T>>::get(pair.index);
            match side {
                Sell => {
                    if !handicap.lowest_offer.is_zero() {
                        if <QuotationsOf<T>>::get(&(pair.index, handicap.lowest_offer)).is_empty() {
                            handicap.tick_up_lowest_offer(tick_precision);
                            <HandicapOf<T>>::insert(pair.index, &handicap);

                            debug!(
                                "[update_handicap] pair_index: {:?}, lowest_offer: {:?}, side: {:?}",
                                pair.index,
                                handicap.lowest_offer,
                                Sell,
                            );
                        }
                    }
                }
                Buy => {
                    if !handicap.highest_bid.is_zero() {
                        if <QuotationsOf<T>>::get((pair.index, handicap.highest_bid)).is_empty() {
                            handicap.tick_down_highest_bid(tick_precision);
                            <HandicapOf<T>>::insert(pair.index, &handicap);

                            debug!(
                                "[update_handicap] pair_index: {:?}, highest_bid: {:?}, side: {:?}",
                                pair.index, handicap.highest_bid, Buy
                            );
                        }
                    }
                }
            };
        };
    }

    pub(super) fn update_handicap_after_matching_order(
        pair: &TradingPair,
        order: &mut OrderInfo<T>,
    ) {
        match order.side() {
            Buy => Self::update_handicap_of_buyers(pair, order),
            Sell => Self::update_handicap_of_sellers(pair, order),
        }
    }

    pub(super) fn tick_up(v: T::Price, tick: u64) -> T::Price {
        match v.checked_add(&tick.into()) {
            Some(x) => x,
            None => panic!("Fail to tick up"),
        }
    }

    /// This is only used for updating the handicap. Return zero when underflow.
    pub(super) fn tick_down(v: T::Price, tick: u64) -> T::Price {
        v.checked_sub(&tick.into()).unwrap_or_else(Zero::zero)
    }

    fn update_handicap_of_buyers(pair: &TradingPair, order: &mut OrderInfo<T>) {
        let mut handicap = <HandicapOf<T>>::get(pair.index);
        if order.price() > handicap.highest_bid || handicap.highest_bid == Default::default() {
            let highest_bid = order.price();

            if highest_bid >= handicap.lowest_offer {
                handicap.lowest_offer = Self::tick_up(highest_bid, pair.tick());

                debug!(
                    "[update_handicap] pair_index: {:?}, lowest_offer: {:?}, side: {:?}",
                    order.pair_index(),
                    handicap.lowest_offer,
                    Sell,
                );
            }

            handicap.highest_bid = highest_bid;
            <HandicapOf<T>>::insert(order.pair_index(), handicap);

            debug!(
                "[update_handicap] pair_index: {:?}, highest_bid: {:?}, side: {:?}",
                order.pair_index(),
                highest_bid,
                Buy
            );
        }
    }

    fn update_handicap_of_sellers(pair: &TradingPair, order: &mut OrderInfo<T>) {
        let mut handicap = <HandicapOf<T>>::get(pair.index);
        if order.price() < handicap.lowest_offer || handicap.lowest_offer == Default::default() {
            let lowest_offer = order.price();

            if lowest_offer <= handicap.highest_bid {
                handicap.highest_bid = Self::tick_down(lowest_offer, pair.tick());

                debug!(
                    "[update_handicap] pair_index: {:?}, highest_bid: {:?}, side: {:?}",
                    order.pair_index(),
                    handicap.highest_bid,
                    Buy
                );
            }

            handicap.lowest_offer = lowest_offer;
            <HandicapOf<T>>::insert(order.pair_index(), handicap);

            debug!(
                "[update_handicap] pair_index: {:?}, lowest_offer: {:?}, side: {:?}",
                order.pair_index(),
                lowest_offer,
                Sell,
            );
        }
    }

    fn blocks_per_hour() -> u64 {
        let period = <timestamp::Module<T>>::minimum_period();
        let seconds_per_hour = (60 * 60) as u64;
        seconds_per_hour / period.saturated_into::<u64>()
    }

    /// This happens when the maker orders have been full filled.
    pub(super) fn remove_orders_and_quotations(
        pair_index: TradingPairIndex,
        price: T::Price,
        fulfilled_orders: Vec<(T::AccountId, OrderIndex)>,
    ) {
        debug!(
            "[remove_orders_and_quotations] These fulfilled orders will be removed: {:?}",
            fulfilled_orders
        );
        for order_key in fulfilled_orders.iter() {
            <OrderInfoOf<T>>::remove(order_key);
        }

        <QuotationsOf<T>>::mutate(&(pair_index, price), |quotations| {
            quotations.retain(|i| !fulfilled_orders.contains(i));
        });
    }

    /// This happens when the order is killed.
    pub(super) fn remove_quotation(
        pair_index: TradingPairIndex,
        price: T::Price,
        order_key: (T::AccountId, OrderIndex),
    ) {
        <QuotationsOf<T>>::mutate(&(pair_index, price), |quotations| {
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
    pub(crate) fn update_latest_and_average_price(pair_index: TradingPairIndex, latest: T::Price) {
        let blocks_per_hour = Self::blocks_per_hour();
        let current_block = <system::Module<T>>::block_number();

        let aver = if let Some((_, aver, last_update)) = <TradingPairInfoOf<T>>::get(pair_index) {
            let elapsed = (current_block - last_update).saturated_into::<u64>();
            if elapsed < blocks_per_hour {
                let new_weight = latest.into() * elapsed;
                let old_weight = aver.into() * (blocks_per_hour - elapsed);
                ((new_weight + old_weight) / blocks_per_hour).into()
            } else {
                latest
            }
        } else {
            latest
        };

        debug!(
            "[update_latest_and_average_price] latest: {:?}, average: {:?}, current_block: {:?}",
            latest, aver, current_block
        );
        <TradingPairInfoOf<T>>::insert(pair_index, (latest, aver, current_block));
    }
}
