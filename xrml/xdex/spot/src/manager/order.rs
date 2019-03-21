// Copyright 2019 Chainpool.
//! This module takes care of the order processing.

use super::*;
use primitives::traits::CheckedAdd;

impl<T: Trait> Module<T> {
    /// Create a brand new order with some defaults.
    pub(crate) fn new_fresh_order(
        pair: TradingPairIndex,
        price: T::Price,
        order_index: ID,
        submitter: T::AccountId,
        class: OrderType,
        direction: OrderDirection,
        amount: T::Balance,
        remaining: T::Balance,
    ) -> Order<TradingPairIndex, T::AccountId, T::Balance, T::Price, T::BlockNumber> {
        let current_block = <system::Module<T>>::block_number();
        let props = OrderProperty::new(
            pair,
            order_index,
            class,
            direction,
            submitter,
            amount,
            price,
            current_block,
        );

        Order::new(
            props,
            Zero::zero(),
            current_block,
            OrderStatus::ZeroExecuted,
            Default::default(),
            remaining,
        )
    }

    fn update_order(order: &mut OrderDetails<T>, amount: &T::Balance, new_fill_index: ID) {
        order.fill_index.push(new_fill_index);
        order.already_filled = order.already_filled.checked_add(amount).unwrap_or_default();

        order.status = if order.already_filled == order.amount() {
            OrderStatus::AllExecuted
        } else if order.already_filled < order.amount() {
            OrderStatus::ParitialExecuted
        } else {
            panic!("maker order has not enough amount");
        };

        order.last_update_at = <system::Module<T>>::block_number();
    }

    pub(crate) fn fill_order(
        pairid: TradingPairIndex,
        maker_order: &mut OrderDetails<T>,
        taker_order: &mut OrderDetails<T>,
        price: T::Price,
        amount: T::Balance,
    ) -> Result {
        let pair = Self::trading_pair(&pairid)?;

        // 更新挂单、成交历史、资产转移
        let new_fill_index = Self::trade_history_index_of(pairid) + 1;

        // 更新 maker, taker 对应的订单
        Self::update_order(maker_order, &amount, new_fill_index);
        Self::update_order(maker_order, &amount, new_fill_index);

        Self::delivery_asset_to_each_other(
            maker_order.direction(),
            &pair,
            amount,
            price,
            maker_order,
            taker_order,
        )?;

        //插入新的成交记录
        let fill = Fill {
            pair: pairid,
            price,
            index: new_fill_index,
            maker: Maker(maker_order.submitter(), maker_order.index()),
            taker: Taker(taker_order.submitter(), taker_order.index()),
            amount,
            time: <system::Module<T>>::block_number(),
        };

        <TradeHistoryIndexOf<T>>::insert(pairid, new_fill_index);

        //插入更新后的订单
        Self::event_order(&maker_order.clone());
        <OrderInfoOf<T>>::insert(
            (maker_order.submitter(), maker_order.index()),
            &maker_order.clone(),
        );

        Self::event_order(&taker_order.clone());
        <OrderInfoOf<T>>::insert(
            (taker_order.submitter(), taker_order.index()),
            &taker_order.clone(),
        );

        // 记录日志
        Self::deposit_event(RawEvent::FillOrder(
            fill.index,
            fill.pair,
            fill.price,
            fill.maker.0,
            fill.taker.0,
            fill.maker.1,
            fill.taker.1,
            fill.amount,
            fill.time.as_(),
        ));

        Ok(())
    }

    pub(crate) fn update_quotations_and_handicap(pair: &TradingPair, order: &mut OrderDetails<T>) {
        if order.amount() > order.already_filled {
            if order.already_filled > Zero::zero() {
                order.status = OrderStatus::ParitialExecuted;
            }

            <OrderInfoOf<T>>::insert((order.submitter(), order.index()), &order.clone());
            Self::event_order(&order);

            // 更新报价
            let quotation_key = (order.pair(), order.price());
            let mut quotations = <QuotationsOf<T>>::get(&quotation_key);
            quotations.push((order.submitter(), order.index()));
            <QuotationsOf<T>>::insert(&quotation_key, quotations);

            // 更新盘口
            match order.direction() {
                OrderDirection::Buy => {
                    if let Some(mut handicap) = <HandicapOf<T>>::get(order.pair()) {
                        if order.price() > handicap.buy || handicap.buy == Default::default() {
                            handicap.buy = order.price();
                            Self::event_handicap(order.pair(), handicap.buy, OrderDirection::Buy);

                            if handicap.buy >= handicap.sell {
                                handicap.sell = handicap
                                    .buy
                                    .checked_add(&As::sa(10_u64.pow(pair.unit_precision)))
                                    .unwrap_or_default();
                                Self::event_handicap(
                                    order.pair(),
                                    handicap.sell,
                                    OrderDirection::Sell,
                                );
                            }
                            <HandicapOf<T>>::insert(order.pair(), handicap);
                        }
                    } else {
                        let mut handicap: HandicapT<T> = Default::default();
                        handicap.buy = order.price();
                        Self::event_handicap(order.pair(), handicap.buy, OrderDirection::Buy);
                        <HandicapOf<T>>::insert(order.pair(), handicap);
                    }
                }
                OrderDirection::Sell => {
                    if let Some(mut handicap) = <HandicapOf<T>>::get(order.pair()) {
                        if order.price() < handicap.sell || handicap.sell == Default::default() {
                            handicap.sell = order.price();
                            Self::event_handicap(order.pair(), handicap.sell, OrderDirection::Sell);

                            if handicap.sell <= handicap.buy {
                                handicap.buy = handicap
                                    .sell
                                    .checked_sub(&As::sa(10_u64.pow(pair.unit_precision)))
                                    .unwrap_or_default();
                                Self::event_handicap(
                                    order.pair(),
                                    handicap.buy,
                                    OrderDirection::Buy,
                                );
                            }
                            <HandicapOf<T>>::insert(order.pair(), handicap);
                        }
                    } else {
                        let mut handicap: HandicapT<T> = Default::default();
                        handicap.sell = order.price();
                        Self::event_handicap(order.pair(), handicap.sell, OrderDirection::Sell);
                        <HandicapOf<T>>::insert(order.pair(), handicap);
                    }
                }
            }
        } else {
            // 更新状态 删除
            order.status = OrderStatus::AllExecuted;
            Self::event_order(&order);
            <OrderInfoOf<T>>::remove((order.submitter(), order.index()));
        }
    }

    pub(crate) fn try_match_order(
        order: &mut OrderDetails<T>,
        pair: &TradingPair,
        handicap: &HandicapT<T>,
    ) {
        let (opponent_direction, mut opponent_price) = match order.direction() {
            OrderDirection::Buy => (OrderDirection::Sell, handicap.sell),
            OrderDirection::Sell => (OrderDirection::Buy, handicap.buy),
        };
        let min_unit = 10_u64.pow(pair.unit_precision);
        let safe_checked_sub = |x: T::Balance, y: T::Balance| x.checked_sub(&y).unwrap_or_default();

        loop {
            if opponent_price.is_zero() {
                return;
            }

            if order.already_filled >= order.amount() {
                order.status = OrderStatus::AllExecuted;
                return;
            }

            let found = match order.direction() {
                OrderDirection::Buy => {
                    if order.price() >= opponent_price {
                        true
                    } else {
                        return;
                    }
                }
                OrderDirection::Sell => {
                    if order.price() <= opponent_price {
                        true
                    } else {
                        return;
                    }
                }
            };

            if found {
                let quotations = <QuotationsOf<T>>::get(&(pair.id, opponent_price));
                for quotation in quotations.iter() {
                    if order.already_filled >= order.amount() {
                        order.status = OrderStatus::AllExecuted;
                        break;
                    }
                    // 找到匹配的单
                    if let Some(mut maker_order) = <OrderInfoOf<T>>::get(quotation) {
                        if opponent_direction != maker_order.direction() {
                            panic!("opponent direction error");
                        }
                        let v1 = safe_checked_sub(order.amount(), order.already_filled);
                        let v2 = safe_checked_sub(maker_order.amount(), maker_order.already_filled);
                        let amount = cmp::min(v1, v2);

                        //填充成交
                        let _ = Self::fill_order(
                            pair.id,
                            &mut maker_order,
                            order,
                            opponent_price,
                            amount,
                        );

                        //更新最新价、平均价
                        Self::update_last_average_price(pair.id, opponent_price);
                    }
                }
            }

            //移动对手价
            opponent_price =
                Self::alter_opponent_price(order.direction(), opponent_price, min_unit);
        }
    }

    fn alter_opponent_price(
        direction: OrderDirection,
        opponent_price: T::Price,
        min_unit: u64,
    ) -> T::Price {
        match direction {
            OrderDirection::Buy => opponent_price
                .checked_add(&As::sa(min_unit))
                .unwrap_or_default(),
            OrderDirection::Sell => opponent_price
                .checked_sub(&As::sa(min_unit))
                .unwrap_or_default(),
        }
    }
}
