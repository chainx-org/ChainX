// Copyright 2019 Chainpool.
//! This module takes care of the order processing.

#[cfg(feature = "std")]
use chrono::prelude::*;

use super::*;
use sp_runtime::traits::CheckedAdd;

impl<T: Trait> Module<T> {
    /// When the price is far from the current handicap, i.e.,
    /// - buy: less than the lowest_ask
    /// - sell: larger than the highest_bid
    /// what we only need to do is to check if the handicap should be updated.
    /// Or else we should match the order.
    pub(crate) fn try_match_order(
        pair: &TradingPairProfile,
        order: &mut OrderInfo<T>,
        pair_index: TradingPairId,
        side: Side,
        price: T::Price,
    ) {
        let handicap = <HandicapOf<T>>::get(pair_index);
        let (lowest_ask, highest_bid) = (handicap.lowest_ask, handicap.highest_bid);

        // If the price is too low or too high, we only need to check if the handicap should be updated,
        // otherwise we should match this order.
        let skip_match_order = match side {
            Side::Buy => lowest_ask.is_zero() || price < lowest_ask,
            Side::Sell => highest_bid.is_zero() || price > highest_bid,
        };

        // If there is no chance to match order, we only have to insert this quote and update handicap.
        if skip_match_order {
            <QuotationsOf<T>>::mutate(order.pair_id(), order.price(), |quotations| {
                quotations.push((order.submitter(), order.id()))
            });

            match side {
                Side::Buy if price > highest_bid => {
                    <HandicapOf<T>>::mutate(pair_index, |handicap| handicap.highest_bid = price);
                }
                Side::Sell if lowest_ask.is_zero() || price < lowest_ask => {
                    <HandicapOf<T>>::mutate(pair_index, |handicap| handicap.lowest_ask = price);
                }
                _ => (),
            }
        } else {
            Self::match_order(&pair, order, &handicap);
        }

        Self::deposit_event(RawEvent::UpdateOrder(order.clone()));
    }

    /// Insert a fresh order and return the inserted result.
    pub(crate) fn inject_order(
        who: T::AccountId,
        pair_id: TradingPairId,
        price: T::Price,
        order_type: OrderType,
        side: Side,
        amount: BalanceOf<T>,
        remaining: BalanceOf<T>,
    ) -> Order<TradingPairId, T::AccountId, BalanceOf<T>, T::Price, T::BlockNumber> {
        let order_id = Self::order_count_of(&who);

        let submitter = who.clone();
        let order = Self::new_fresh_order(
            pair_id, price, order_id, submitter, order_type, side, amount, remaining,
        );

        debug!("[inject_order]a new order injected:{:?}", order);
        <OrderInfoOf<T>>::insert(&who, order_id, &order);

        // The order count of user should be increased after a new order is created.
        <OrderCountOf<T>>::insert(&who, order_id + 1);

        Self::deposit_event(RawEvent::PutOrder(order.clone()));

        order
    }

    /// Create a brand new order with some defaults.
    #[allow(clippy::too_many_arguments)]
    fn new_fresh_order(
        pair_id: TradingPairId,
        price: T::Price,
        order_id: OrderId,
        submitter: T::AccountId,
        class: OrderType,
        side: Side,
        amount: BalanceOf<T>,
        remaining: BalanceOf<T>,
    ) -> Order<TradingPairId, T::AccountId, BalanceOf<T>, T::Price, T::BlockNumber> {
        let current_block = <frame_system::Module<T>>::block_number();
        let props = OrderProperty {
            pair_id,
            side,
            submitter,
            amount,
            price,
            id: order_id,
            order_type: class,
            created_at: current_block,
        };

        Order::new(
            props,
            Zero::zero(),
            current_block,
            OrderStatus::Created,
            Default::default(),
            remaining,
        )
    }

    /// Match the new putted order. When the matching is complete, we should check
    /// if the order has been fulfilled and update the handicap.
    fn match_order(
        pair: &TradingPairProfile,
        order: &mut OrderInfo<T>,
        handicap: &HandicapInfo<T>,
    ) {
        #[cfg(feature = "std")]
        let begin = Local::now().timestamp_millis();

        Self::apply_match_order(order, pair, handicap);

        #[cfg(feature = "std")]
        let end = Local::now().timestamp_millis();
        #[cfg(feature = "std")]
        debug!("[match order] elasped time: {:}ms", end - begin);

        // Remove the full filled order, otherwise the quotations, order status and handicap
        // should be updated.
        if order.is_fulfilled() {
            order.status = OrderStatus::Filled;
            <OrderInfoOf<T>>::remove(order.submitter(), order.id());
        } else {
            <QuotationsOf<T>>::mutate(order.pair_id(), order.price(), |quotations| {
                quotations.push((order.submitter(), order.id()))
            });

            // NOTE: Since the handicap is not always related to a real order,
            // this guard statement is neccessary!
            if order.already_filled > Zero::zero() {
                order.status = OrderStatus::ParitialFill;
            }

            <OrderInfoOf<T>>::insert(order.submitter(), order.id(), order.clone());

            Self::update_handicap_after_matching_order(pair, order);
        }
    }

    fn apply_match_order_given_counterparty(
        taker_order: &mut OrderInfo<T>,
        pair: &TradingPairProfile,
        counterparty_price: T::Price,
        counterparty_side: Side,
    ) {
        let quotations = <QuotationsOf<T>>::get(pair.id, counterparty_price);
        let mut fulfilled_orders = Vec::new();

        for (who, order_index) in quotations.iter() {
            if taker_order.is_fulfilled() {
                break;
            }
            // Find the matched order.
            if let Some(mut maker_order) = <OrderInfoOf<T>>::get(who, order_index) {
                assert!(
                    counterparty_side == maker_order.side(),
                    "Opponent side should match the side of maker order."
                );

                let turnover = cmp::min(
                    taker_order.remaining_in_base(),
                    maker_order.remaining_in_base(),
                );

                // Execute the order at the opponent price when they match.
                let execution_result = Self::execute_order(
                    pair.id,
                    &mut maker_order,
                    taker_order,
                    counterparty_price,
                    turnover,
                );

                assert!(execution_result.is_ok(), "Match order execution paniced");

                // Remove maker_order if it has been full filled.
                if maker_order.is_fulfilled() {
                    fulfilled_orders.push((maker_order.submitter(), maker_order.id()));
                    Self::update_handicap(&pair, counterparty_price, maker_order.side());
                }

                Self::update_latest_price(pair.id, counterparty_price);
            }
        }

        // Remove the fulfilled orders as well as the quotations.
        if !fulfilled_orders.is_empty() {
            Self::remove_orders_and_quotations(pair.id, counterparty_price, fulfilled_orders);
        }
    }

    fn match_taker_order_buy(
        taker_order: &mut OrderInfo<T>,
        pair: &TradingPairProfile,
        lowest_ask: T::Price,
    ) {
        let tick = pair.tick();
        let my_quote = taker_order.price();

        let counterparty_side = Side::Sell;
        let (floor, ceiling) = (lowest_ask, my_quote);

        let mut counterparty_price = floor;

        while !counterparty_price.is_zero() && counterparty_price <= ceiling {
            if taker_order.is_fulfilled() {
                return;
            }
            Self::apply_match_order_given_counterparty(
                taker_order,
                pair,
                counterparty_price,
                counterparty_side,
            );
            counterparty_price = Self::tick_up(counterparty_price, tick);
        }
    }

    fn match_taker_order_sell(
        taker_order: &mut OrderInfo<T>,
        pair: &TradingPairProfile,
        highest_bid: T::Price,
    ) {
        let tick = pair.tick();
        let my_quote = taker_order.price();

        let counterparty_side = Side::Buy;
        let (floor, ceiling) = (my_quote, highest_bid);

        let mut counterparty_price = ceiling;

        while !counterparty_price.is_zero() && counterparty_price >= floor {
            if taker_order.is_fulfilled() {
                return;
            }
            Self::apply_match_order_given_counterparty(
                taker_order,
                pair,
                counterparty_price,
                counterparty_side,
            );
            counterparty_price = Self::tick_down(counterparty_price, tick);
        }
    }

    /// TODO: optimize the matching order.
    ///
    /// Currently the matching is processed by iterating the tick one by one.
    fn apply_match_order(
        taker_order: &mut OrderInfo<T>,
        pair: &TradingPairProfile,
        handicap: &HandicapInfo<T>,
    ) {
        let (lowest_ask, highest_bid) = (handicap.lowest_ask, handicap.highest_bid);

        //  Buy: [ lowest_ask  , my_quote ]
        // Sell: [ my_quote , highest_bid   ]
        match taker_order.side() {
            Side::Buy => Self::match_taker_order_buy(taker_order, pair, lowest_ask),
            Side::Sell => Self::match_taker_order_sell(taker_order, pair, highest_bid),
        }
    }

    /// Remove the order from quotations and clear the order info when it's canceled.
    pub(crate) fn kill_order(
        pair_id: TradingPairId,
        price: T::Price,
        who: T::AccountId,
        order_index: OrderId,
        pair: TradingPairProfile,
        order_side: Side,
    ) {
        <OrderInfoOf<T>>::remove(&who, order_index);

        let order_key = (who, order_index);
        Self::remove_quotation(pair_id, price, order_key);

        Self::update_handicap(&pair, price, order_side);
    }

    /// Update the status of order after the turnover is calculated.
    fn update_order_on_execute(
        order: &mut OrderInfo<T>,
        turnover: &BalanceOf<T>,
        trade_history_index: TradingHistoryIndex,
    ) {
        order.executed_indices.push(trade_history_index);

        // Unwrap or default?
        order.already_filled = match order.already_filled.checked_add(turnover) {
            Some(x) => x,
            None => panic!("add order.already_filled overflow"),
        };

        order.status = if order.already_filled == order.amount() {
            OrderStatus::Filled
        } else if order.already_filled < order.amount() {
            OrderStatus::ParitialFill
        } else {
            panic!("Already filled of an order can't greater than the order's amount.");
        };

        order.last_update_at = <frame_system::Module<T>>::block_number();
    }

    /// Writes the `order` to the storage.
    #[inline]
    fn insert_executed_order(order: &OrderInfo<T>) {
        <OrderInfoOf<T>>::insert(order.submitter(), order.id(), order);
    }

    /// Refund the remaining asset to the order submitter.
    ///
    /// Due to the loss of decimals in `Self::convert_base_to_quote()`,
    /// the remaining could still be non-zero when the order is full filled,
    /// which must be refunded.
    fn try_refund_remaining(order: &mut OrderInfo<T>, asset_id: AssetId) {
        // NOTE: Refund the remaining reserved asset when the order is fulfilled.
        if order.is_fulfilled() && !order.remaining.is_zero() {
            let unreserve_result =
                Self::generic_unreserve(&order.submitter(), asset_id, order.remaining);
            assert!(
                unreserve_result.is_ok(),
                "Unreserve the remaining asset can not fail"
            );
            order.remaining = Zero::zero();
        }
    }

    /// 1. update the taker and maker order based on the turnover
    /// 2. delivery asset to each other
    /// 3. update the remaining field of orders
    /// 4. try refunding the non-zero remaining asset if order is fulfilled
    fn execute_order(
        pair_id: TradingPairId,
        maker_order: &mut OrderInfo<T>,
        taker_order: &mut OrderInfo<T>,
        price: T::Price,
        turnover: BalanceOf<T>,
    ) -> DispatchResult {
        let pair = Self::trading_pair(pair_id)?;

        let trading_history_idx = Self::trading_history_index_of(pair_id);
        TradingHistoryIndexOf::insert(pair_id, trading_history_idx + 1);

        Self::update_order_on_execute(maker_order, &turnover, trading_history_idx);
        Self::update_order_on_execute(taker_order, &turnover, trading_history_idx);

        let (maker_turnover_amount, taker_turnover_amount) = Self::delivery_asset_to_each_other(
            maker_order.side(),
            &pair,
            turnover,
            price,
            maker_order,
            taker_order,
        )?;

        maker_order.decrease_remaining_on_execute(maker_turnover_amount);
        taker_order.decrease_remaining_on_execute(taker_turnover_amount);

        let refund_remaining_asset = |order: &OrderInfo<T>| match order.side() {
            Side::Buy => pair.quote(),
            Side::Sell => pair.base(),
        };

        Self::try_refund_remaining(maker_order, refund_remaining_asset(maker_order));
        Self::try_refund_remaining(taker_order, refund_remaining_asset(taker_order));

        Self::insert_executed_order(maker_order);
        Self::insert_executed_order(taker_order);

        Self::deposit_event(RawEvent::UpdateOrder(maker_order.clone()));
        Self::deposit_event(RawEvent::UpdateOrder(taker_order.clone()));
        Self::deposit_event(RawEvent::OrderExecuted(OrderExecutedInfo::new(
            trading_history_idx,
            pair_id,
            price,
            turnover,
            maker_order,
            taker_order,
            <frame_system::Module<T>>::block_number(),
        )));

        Ok(())
    }

    pub(crate) fn update_order_and_unreserve_on_cancel(
        order: &mut OrderInfo<T>,
        pair: &TradingPairProfile,
        who: &T::AccountId,
    ) -> DispatchResult {
        // Unreserve the remaining asset.
        let (refund_asset, refund_amount) = match order.side() {
            Side::Sell => (pair.base(), order.remaining_in_base()),
            Side::Buy => (pair.quote(), order.remaining),
        };

        Self::generic_unreserve(who, refund_asset, refund_amount)?;

        order.update_status_on_cancel();
        order.decrease_remaining_on_cancel(refund_amount);
        order.last_update_at = <frame_system::Module<T>>::block_number();

        OrderInfoOf::<T>::insert(order.submitter(), order.id(), order.clone());

        Self::deposit_event(RawEvent::UpdateOrder(order.clone()));

        Ok(())
    }
}
