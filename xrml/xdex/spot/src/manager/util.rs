// Copyright 2019 Chainpool.
//! This module provides some common utilities for internal uses.

use super::*;
use primitives::traits::CheckedAdd;

/// Internal mutables
impl<T: Trait> Module<T> {
    /// 更新盘口
    pub(crate) fn update_handicap(pair: &TradingPair, price: T::Price, direction: OrderDirection) {
        let min_unit = 10_u64.pow(pair.unit_precision);

        if <QuotationsOf<T>>::get((pair.id, price)).is_empty() {
            match direction {
                OrderDirection::Sell => {
                    //更新卖一
                    if let Some(mut handicap) = <HandicapOf<T>>::get(pair.id) {
                        if <QuotationsOf<T>>::get((pair.id, handicap.sell)).is_empty() {
                            handicap.sell = handicap
                                .sell
                                .checked_add(&As::sa(min_unit))
                                .unwrap_or_default();
                            Self::event_handicap(pair.id, handicap.sell, OrderDirection::Sell);
                            <HandicapOf<T>>::insert(pair.id, handicap);
                        }
                    }
                }
                OrderDirection::Buy => {
                    //更新买一
                    if let Some(mut handicap) = <HandicapOf<T>>::get(pair.id) {
                        if <QuotationsOf<T>>::get((pair.id, handicap.buy)).is_empty() {
                            handicap.buy = handicap
                                .buy
                                .checked_sub(&As::sa(min_unit))
                                .unwrap_or_default();
                            Self::event_handicap(pair.id, handicap.buy, OrderDirection::Buy);
                            <HandicapOf<T>>::insert(pair.id, handicap);
                        }
                    }
                }
            };
        };
    }

    fn blocks_per_hour() -> u64 {
        let period = <timestamp::Module<T>>::block_period();
        let seconds_for_hour = (60 * 60) as u64;
        seconds_for_hour / period.as_()
    }

    pub(crate) fn update_last_average_price(pairid: TradingPairIndex, price: T::Price) {
        let blocks_per_hour: u64 = Self::blocks_per_hour();
        let number = <system::Module<T>>::block_number();

        if let Some((_, aver, time)) = <TradingPairInfoOf<T>>::get(pairid) {
            let aver = if number - time < As::sa(blocks_per_hour) {
                let new_weight = price.as_() * (number - time).as_();
                let old_weight = aver.as_() * (blocks_per_hour + time.as_() - number.as_());
                As::sa((new_weight + old_weight) / blocks_per_hour)
            } else {
                price
            };
            <TradingPairInfoOf<T>>::insert(pairid, (price, aver, number));
        } else {
            <TradingPairInfoOf<T>>::insert(pairid, (price, price, number));
        }
    }

    /// BTC/PCX
    /// Convert the base currency to the counter currency given the currency pair in trading pair
    /// amount: first(BTC) 的数量单位, price: 以 second(PCX) 计价的价格
    /// 公式 =（ amount * price * 10^second精度 ）/（ first精度 * price精度）
    /// amount * price * 10^(second.precision) / 10^(first.precision) * 10^(price.precision)
    /// = amount * price * 10^(second.precision - first.precision - price.precision)
    pub(crate) fn convert_to_counter_currency(
        amount: T::Balance,
        price: T::Price,
        pair: &TradingPair,
    ) -> Option<T::Balance> {
        if let (Some((first, _, _)), Some((second, _, _))) = (
            <xassets::Module<T>>::asset_info(&pair.currency_pair.0),
            <xassets::Module<T>>::asset_info(&pair.currency_pair.1),
        ) {
            let (first_p, second_p, pair_p) = (
                first.precision() as u32,
                second.precision() as u32,
                pair.precision,
            );
            let (mul, s) = if second_p >= (first_p + pair_p) {
                (true, 10_u128.pow(second_p - first_p - pair_p))
            } else {
                (false, 10_u128.pow(first_p + pair_p - second_p))
            };
            // Can overflow
            let ap = amount.as_() as u128 * price.as_() as u128;
            let transformed = if mul {
                match ap.checked_mul(s) {
                    Some(r) => r,
                    None => panic!("amount * price * precision overflow"),
                }
            } else {
                ap / s
            };

            if !transformed.is_zero() {
                return Some(T::Balance::sa(transformed as u64));
            }
        }

        None
    }

    /// 检查和更新报价
    pub(crate) fn check_and_delete_quotations(id: TradingPairIndex, price: T::Price) {
        let quotations = <QuotationsOf<T>>::get(&(id, price));
        if quotations.is_empty() {
            return;
        }

        let mut new_list: Vec<(T::AccountId, ID)> = Vec::new();
        for quotation in quotations.into_iter() {
            if let Some(order) = <OrderInfoOf<T>>::get(&quotation) {
                if order.already_filled >= order.amount()
                    || OrderStatus::ParitialExecutedAndCanceled == order.status
                    || OrderStatus::Canceled == order.status
                {
                    //Event 记录挂单详情状态变更
                    Self::event_order(&order);

                    //删除挂单详情
                    <OrderInfoOf<T>>::remove(&quotation);
                    Self::deposit_event(RawEvent::RemoveUserQuotations(
                        order.submitter(),
                        order.index(),
                    ));
                } else {
                    new_list.push(quotation);
                }
            }
        }

        // 空了就删除
        <QuotationsOf<T>>::insert(&(id, price), new_list);
    }

    pub(crate) fn event_order(order: &OrderDetails<T>) {
        Self::deposit_event(RawEvent::UpdateOrder(
            order.submitter(),
            order.index(),
            order.pair(),
            order.price(),
            order.order_type(),
            order.direction(),
            order.amount(),
            order.already_filled,
            order.created_at(),
            order.last_update_at,
            order.status,
            order.remaining,
            order.fill_index.clone(),
        ));
    }

    pub(crate) fn event_pair(pair: &TradingPair) {
        Self::deposit_event(RawEvent::UpdateOrderPair(
            pair.id,
            pair.currency_pair.clone(),
            pair.precision,
            pair.unit_precision,
            pair.online,
        ));
    }

    pub(crate) fn event_handicap(id: TradingPairIndex, price: T::Price, direction: OrderDirection) {
        Self::deposit_event(RawEvent::Handicap(id, price, direction));
    }
}
