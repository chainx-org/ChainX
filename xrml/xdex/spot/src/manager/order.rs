// Copyright 2019 Chainpool.
//! This module takes care of the order processing.

use super::*;
use primitives::traits::CheckedAdd;

impl<T: Trait> Module<T> {
    /// When the price is far from the current handicap, i.e.,
    /// - buy: less than the lowest_offer
    /// - sell: larger than the highest_bid
    /// what we only need to do is to check if the handicap should be updated.
    /// Or else we should match the order.
    pub(crate) fn try_match_order(
        pair: &TradingPair,
        order: &mut OrderInfo<T>,
        pair_index: TradingPairIndex,
        side: Side,
        price: T::Price,
    ) {
        let handicap = <HandicapOf<T>>::get(pair_index);
        let (lowest_offer, highest_bid) = (handicap.lowest_offer, handicap.highest_bid);

        // If the price is too low or too high, we only need to check if the handicap should be updated,
        // otherwise we should match this order.
        let skip_match_order = match side {
            Buy => lowest_offer.is_zero() || price < lowest_offer,
            Sell => highest_bid.is_zero() || price > highest_bid,
        };

        // If there is no chance to match order, we only have to insert this quote and update handicap.
        if skip_match_order {
            <QuotationsOf<T>>::mutate(&(order.pair_index(), order.price()), |quotations| {
                quotations.push((order.submitter(), order.index()))
            });

            match side {
                Buy if price > highest_bid => {
                    <HandicapOf<T>>::mutate(pair_index, |handicap| handicap.highest_bid = price);
                }
                Sell if lowest_offer.is_zero() || price < lowest_offer => {
                    <HandicapOf<T>>::mutate(pair_index, |handicap| handicap.lowest_offer = price);
                }
                _ => (),
            }
        } else {
            Self::match_order(&pair, order, &handicap);
        }

        Self::update_order_event(&order);
    }

    /// Insert a fresh order and return the inserted result.
    pub(crate) fn inject_order(
        who: T::AccountId,
        pair_index: TradingPairIndex,
        price: T::Price,
        order_type: OrderType,
        side: Side,
        amount: T::Balance,
        remaining: T::Balance,
    ) -> Order<TradingPairIndex, T::AccountId, T::Balance, T::Price, T::BlockNumber> {
        // The order count of user should be increased as well.
        let order_index = Self::order_count_of(&who);
        <OrderCountOf<T>>::insert(&who, order_index + 1);

        let order = Self::new_fresh_order(
            pair_index,
            price,
            order_index,
            who,
            order_type,
            side,
            amount,
            remaining,
        );

        debug!("[inject_order] {:?}", order);
        <OrderInfoOf<T>>::insert(&(order.submitter(), order.index()), &order);

        Self::deposit_event(RawEvent::PutOrder(
            order.submitter(),
            order.index(),
            order.pair_index(),
            order.order_type(),
            order.price(),
            order.side(),
            order.amount(),
            order.created_at(),
        ));

        order
    }

    /// Create a brand new order with some defaults.
    fn new_fresh_order(
        pair_index: TradingPairIndex,
        price: T::Price,
        order_index: OrderIndex,
        submitter: T::AccountId,
        class: OrderType,
        side: Side,
        amount: T::Balance,
        remaining: T::Balance,
    ) -> Order<TradingPairIndex, T::AccountId, T::Balance, T::Price, T::BlockNumber> {
        let current_block = <system::Module<T>>::block_number();
        let props = OrderProperty::new(
            pair_index,
            order_index,
            class,
            side,
            submitter,
            amount,
            price,
            current_block,
        );

        Order::new(
            props,
            Zero::zero(),
            current_block,
            OrderStatus::ZeroFill,
            Default::default(),
            remaining,
        )
    }

    /// Match the new putted order. When the matching is complete, we should check
    /// if the order has been fulfilled and update the handicap.
    fn match_order(pair: &TradingPair, order: &mut OrderInfo<T>, handicap: &HandicapInfo<T>) {
        #[cfg(feature = "std")]
        let begin = Local::now().timestamp_millis();

        Self::apply_match_order(order, pair, handicap);

        #[cfg(feature = "std")]
        let end = Local::now().timestamp_millis();
        debug!("[match order] elasped time: {:}ms", end - begin);

        // Remove the full filled order, otherwise the quotations, order status and handicap
        // should be updated.
        if order.is_fulfilled() {
            order.status = OrderStatus::Filled;
            <OrderInfoOf<T>>::remove(&(order.submitter(), order.index()));
        } else {
            <QuotationsOf<T>>::mutate(&(order.pair_index(), order.price()), |quotations| {
                quotations.push((order.submitter(), order.index()))
            });

            // Since the handicap is not always related to a real order, this guard statement is neccessary!
            if order.already_filled > Zero::zero() {
                order.status = OrderStatus::ParitialFill;
            }

            <OrderInfoOf<T>>::insert(&(order.submitter(), order.index()), order.clone());

            Self::update_handicap_after_matching_order(pair, order);
        }
    }

    fn apply_match_order_given_opponent_price(
        taker_order: &mut OrderInfo<T>,
        pair: &TradingPair,
        opponent_price: T::Price,
        opponent_side: Side,
    ) {
        let quotations = <QuotationsOf<T>>::get(&(pair.index, opponent_price));
        for quotation in quotations.iter() {
            if taker_order.is_fulfilled() {
                return;
            }
            // Find the matched order.
            if let Some(mut maker_order) = <OrderInfoOf<T>>::get(quotation) {
                if opponent_side != maker_order.side() {
                    panic!("opponent side error");
                }

                let turnover = cmp::min(
                    taker_order.remaining_in_base(),
                    maker_order.remaining_in_base(),
                );

                // Execute the order at the opponent price when they match.
                let _ = Self::execute_order(
                    pair.index,
                    &mut maker_order,
                    taker_order,
                    opponent_price,
                    turnover,
                );

                // Remove maker_order if it has been full filled.
                if maker_order.is_fulfilled() {
                    <OrderInfoOf<T>>::remove(&(maker_order.submitter(), maker_order.index()));

                    Self::remove_quotation(
                        pair.index,
                        opponent_price,
                        maker_order.submitter(),
                        maker_order.index(),
                    );

                    Self::update_handicap(&pair, opponent_price, maker_order.side());
                }

                Self::update_latest_and_average_price(pair.index, opponent_price);
            }
        }
    }

    fn apply_match_order(
        taker_order: &mut OrderInfo<T>,
        pair: &TradingPair,
        handicap: &HandicapInfo<T>,
    ) {
        let (lowest_offer, highest_bid) = (handicap.lowest_offer, handicap.highest_bid);
        let tick = 10_u64.pow(pair.tick_precision);

        let my_quote = taker_order.price();

        //  Buy: [ lowest_offer  , my_quote ]
        // Sell: [ my_quote , highest_bid   ]
        // FIXME refine later
        match taker_order.side() {
            Buy => {
                let (opponent_side, floor, ceiling) = (Sell, lowest_offer, my_quote);

                let mut opponent_price = floor;

                while !opponent_price.is_zero() && opponent_price <= ceiling {
                    if taker_order.is_fulfilled() {
                        return;
                    }
                    Self::apply_match_order_given_opponent_price(
                        taker_order,
                        pair,
                        opponent_price,
                        opponent_side,
                    );
                    opponent_price = Self::tick_up(opponent_price, tick);
                }
            }
            Sell => {
                let (opponent_side, floor, ceiling) = (Buy, my_quote, highest_bid);

                let mut opponent_price = ceiling;

                while !opponent_price.is_zero() && opponent_price >= floor {
                    if taker_order.is_fulfilled() {
                        return;
                    }
                    Self::apply_match_order_given_opponent_price(
                        taker_order,
                        pair,
                        opponent_price,
                        opponent_side,
                    );
                    opponent_price = Self::tick_down(opponent_price, tick);
                }
            }
        }
    }

    /// Remove the order from quotations and clear the order info when it's canceled.
    pub(crate) fn kill_order(
        pair_index: TradingPairIndex,
        price: T::Price,
        who: T::AccountId,
        order_index: OrderIndex,
        pair: TradingPair,
        order_side: Side,
    ) {
        <OrderInfoOf<T>>::remove(&(who.clone(), order_index));

        Self::remove_quotation(pair_index, price, who, order_index);

        Self::update_handicap(&pair, price, order_side);
    }

    /// Update the status of order after the turnover is calculated.
    fn update_order_on_execute(
        order: &mut OrderInfo<T>,
        turnover: &T::Balance,
        trade_history_index: TradeHistoryIndex,
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

        order.last_update_at = <system::Module<T>>::block_number();
    }

    fn insert_refreshed_order(order: &OrderInfo<T>) {
        <OrderInfoOf<T>>::insert(&(order.submitter(), order.index()), order);
    }

    /// Due to the loss of precision in Self::convert_base_to_quote(),
    /// the remaining could still be non-zero when the order is full filled, which must be refunded.
    fn try_refund_remaining(order: &mut OrderInfo<T>, token: &Token) {
        if order.is_fulfilled() && !order.remaining.is_zero() {
            Self::refund_reserved_dex_spot(&order.submitter(), token, order.remaining);
            order.remaining = Zero::zero();
        }
    }

    /// 1. update the taker and maker order based on the turnover
    /// 2. delivery asset to each other
    /// 3. update the remaining field of orders
    /// 4. try refunding the non-zero remaining asset if order is fulfilled
    fn execute_order(
        pair_index: TradingPairIndex,
        maker_order: &mut OrderInfo<T>,
        taker_order: &mut OrderInfo<T>,
        price: T::Price,
        turnover: T::Balance,
    ) -> Result {
        let pair = Self::trading_pair(pair_index)?;

        let trade_history_index = Self::trade_history_index_of(pair_index);
        <TradeHistoryIndexOf<T>>::insert(pair_index, trade_history_index + 1);

        Self::update_order_on_execute(maker_order, &turnover, trade_history_index);
        Self::update_order_on_execute(taker_order, &turnover, trade_history_index);

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

        let refunding_token_type = |order: &OrderInfo<T>| match order.side() {
            Buy => pair.quote(),
            Sell => pair.base(),
        };

        Self::try_refund_remaining(maker_order, &refunding_token_type(maker_order));
        Self::try_refund_remaining(taker_order, &refunding_token_type(taker_order));

        Self::insert_refreshed_order(maker_order);
        Self::insert_refreshed_order(taker_order);

        Self::update_order_event(&maker_order.clone());
        Self::update_order_event(&taker_order.clone());
        Self::deposit_event(RawEvent::FillOrder(
            trade_history_index,
            pair_index,
            price,
            maker_order.submitter(),
            taker_order.submitter(),
            maker_order.index(),
            taker_order.index(),
            turnover,
            <system::Module<T>>::block_number().as_(),
        ));

        Ok(())
    }

    pub(crate) fn update_order_and_unreserve_on_cancel(
        order: &mut OrderInfo<T>,
        pair: &TradingPair,
        who: &T::AccountId,
    ) -> Result {
        // Unreserve the remaining asset.
        let (refund_token, refund_amount) = match order.side() {
            Sell => (pair.base(), order.remaining_in_base()),
            Buy => (pair.quote(), order.remaining),
        };

        Self::cancel_order_unreserve(who, &refund_token, refund_amount)?;

        order.update_status_on_cancel();
        order.decrease_remaining_on_cancel(refund_amount);
        order.last_update_at = <system::Module<T>>::block_number();

        Self::update_order_event(&order);
        <OrderInfoOf<T>>::insert(&(order.submitter(), order.index()), order);

        Ok(())
    }
}
