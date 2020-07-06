// Copyright 2019 Chainpool.

mod asset;
mod order;
mod state;
// pub mod types;

use super::*;
use crate::types::*;
use xpallet_support::debug;

impl<T: Trait> Module<T> {
    fn check_bid_price(
        quote: T::Price,
        lowest_offer: T::Price,
        fluctuation: T::Price,
    ) -> Result<T> {
        debug!(
            "[check_bid_price]quote: {:?}, lowest_offer: {:?}, fluctuation: {:?}",
            quote, lowest_offer, fluctuation
        );

        if lowest_offer.is_zero() {
            return Ok(());
        }

        if quote > lowest_offer && quote - lowest_offer > fluctuation {
            return Err(Error::<T>::TooHighBidPrice);
        }

        Ok(())
    }

    fn check_ask_price(quote: T::Price, highest_bid: T::Price, fluctuation: T::Price) -> Result<T> {
        debug!(
            "[check_ask_price] Sell: quote: {:?}, highest_bid: {:?}, fluctuation: {:?}",
            quote, highest_bid, fluctuation
        );

        if highest_bid.is_zero() {
            return Ok(());
        }

        if quote < highest_bid && highest_bid - quote > fluctuation {
            return Err(Error::<T>::TooLowAskPrice);
        }

        Ok(())
    }

    /// Given the price volatility is 10%, a valid quote range should be:
    /// - sell: [highest_bid - 10% * highest_bid, ~)
    /// - buy:  (~, lowest_offer + 10% * lowest_offer]
    pub(crate) fn is_within_quotation_range(
        quote: T::Price,
        side: Side,
        pair_id: TradingPairId,
    ) -> Result<T> {
        let handicap = <HandicapOf<T>>::get(pair_id);
        let (lowest_offer, highest_bid) = (handicap.lowest_offer, handicap.highest_bid);

        let pair = Self::trading_pair(pair_id)?;
        let fluctuation = pair.fluctuation().saturated_into();

        match side {
            Side::Buy => Self::check_bid_price(quote, lowest_offer, fluctuation),
            Side::Sell => Self::check_ask_price(quote, highest_bid, fluctuation),
        }
    }

    pub(crate) fn has_too_many_backlog_orders(
        pair_id: TradingPairId,
        price: T::Price,
        side: Side,
    ) -> Result<T> {
        let quotations = <QuotationsOf<T>>::get(pair_id, price);
        if quotations.len() >= MAX_BACKLOG_ORDER {
            let (who, order_index) = &quotations[0];
            if let Some(order) = <OrderInfoOf<T>>::get(who, order_index) {
                if order.side() == side {
                    return Err(Error::<T>::TooManyBacklogOrders);
                }
            }
        }

        Ok(())
    }

    /// Convert the base currency to the quote currency given the trading pair.
    ///
    /// NOTE: There is a loss of accuracy here.
    ///
    /// PCX/BTC
    /// amount: measured by the base currency, e.g., PCX.
    /// price: measured by the quote currency, e.g., BTC.
    ///
    /// converted
    /// = amount * price * 10^(quote.precision) / 10^(base.precision) * 10^(price.precision)
    /// = amount * price * 10^(quote.precision - base.precision - price.precision)
    pub(crate) fn convert_base_to_quote(
        amount: T::Balance,
        price: T::Price,
        pair: &TradingPairProfile,
    ) -> result::Result<T::Balance, Error<T>> {
        if let (Some(base), Some(quote)) = (
            <xpallet_assets::Module<T>>::asset_info_of(pair.base()),
            <xpallet_assets::Module<T>>::asset_info_of(pair.quote()),
        ) {
            let (base_p, quote_p, pair_p) = (
                u32::from(base.precision()),
                u32::from(quote.precision()),
                pair.pip_precision,
            );

            let (mul, s) = if quote_p >= (base_p + pair_p) {
                (true, 10_u128.pow(quote_p - base_p - pair_p))
            } else {
                (false, 10_u128.pow(base_p + pair_p - quote_p))
            };

            // Can overflow
            let ap = amount.saturated_into::<u128>() * price.saturated_into::<u128>();

            let converted = if mul {
                match ap.checked_mul(s) {
                    Some(r) => r,
                    None => panic!("amount * price * precision overflow"),
                }
            } else {
                ap / s
            };

            if !converted.is_zero() {
                if converted < u128::from(u64::max_value()) {
                    return Ok((converted as u64).saturated_into());
                } else {
                    panic!("converted quote currency value definitely less than u64::max_value()")
                }
            }
        }

        Err(Error::<T>::VolumeTooSmall)
    }

    pub(crate) fn update_order_event(order: &OrderInfo<T>) {
        // Self::deposit_event(RawEvent::UpdateOrder(
        // order.submitter(),
        // order.index(),
        // order.already_filled,
        // order.last_update_at,
        // order.status,
        // order.remaining,
        // order.executed_indices.clone(),
        // ));
    }

    pub(crate) fn update_order_pair_event(pair: &TradingPairProfile) {
        // Self::deposit_event(RawEvent::UpdateOrderPair(
        // pair.index,
        // pair.currency_pair.clone(),
        // pair.pip_precision,
        // pair.tick_precision,
        // pair.online,
        // ));
    }
}
