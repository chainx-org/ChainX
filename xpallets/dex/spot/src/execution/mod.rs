// Copyright 2020 Chainpool.

mod asset;
mod order;
mod state;

use super::*;
use crate::types::*;
use xpallet_support::debug;

impl<T: Trait> Module<T> {
    fn check_bid_price(quote: T::Price, lowest_ask: T::Price, fluctuation: T::Price) -> Result<T> {
        debug!(
            "[check_bid_price]quote: {:?}, lowest_ask: {:?}, fluctuation: {:?}",
            quote, lowest_ask, fluctuation
        );

        // There is no offer yet, this is the first one.
        if lowest_ask.is_zero() {
            return Ok(());
        }

        if quote > lowest_ask && quote - lowest_ask > fluctuation {
            return Err(Error::<T>::TooHighBidPrice);
        }

        Ok(())
    }

    fn check_ask_price(quote: T::Price, highest_bid: T::Price, fluctuation: T::Price) -> Result<T> {
        debug!(
            "[check_ask_price] Sell: quote: {:?}, highest_bid: {:?}, fluctuation: {:?}",
            quote, highest_bid, fluctuation
        );

        // There is no bid yet, this is the first one.
        if highest_bid.is_zero() {
            return Ok(());
        }

        if quote < highest_bid && highest_bid - quote > fluctuation {
            return Err(Error::<T>::TooLowAskPrice);
        }

        Ok(())
    }

    /// Assume the price volatility is 10%, a valid quote range should be:
    ///
    /// - sell: [highest_bid - 10% * highest_bid, ~)
    /// - buy:  (~, lowest_ask + 10% * lowest_ask]
    pub(crate) fn is_valid_quote(quote: T::Price, side: Side, pair_id: TradingPairId) -> Result<T> {
        let handicap = <HandicapOf<T>>::get(pair_id);
        let (lowest_ask, highest_bid) = (handicap.lowest_ask, handicap.highest_bid);

        let pair = Self::trading_pair(pair_id)?;
        let fluctuation = pair.calc_fluctuation::<T>().saturated_into();

        match side {
            Side::Buy => Self::check_bid_price(quote, lowest_ask, fluctuation),
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

    fn currency_precision_of(asset_id: AssetId) -> Option<u8> {
        <xpallet_assets::Module<T>>::asset_info_of(asset_id).map(|x| x.precision())
    }

    /// Converts the base currency to the quote currency given the trading pair.
    ///
    /// NOTE: There is possibly a loss of accuracy here.
    ///
    /// PCX/BTC
    /// amount: measured by the base currency, e.g., PCX.
    /// price: measured by the quote currency, e.g., BTC.
    ///
    /// volume
    /// = amount * price * 10^(quote.precision) / 10^(base.precision) * 10^(price.precision)
    /// = amount * price * 10^(quote.precision - base.precision - price.precision)
    pub(crate) fn convert_base_to_quote(
        amount: BalanceOf<T>,
        price: T::Price,
        pair: &TradingPairProfile,
    ) -> result::Result<BalanceOf<T>, Error<T>> {
        if let (Some(base_p), Some(quote_p)) = (
            Self::currency_precision_of(pair.base()),
            Self::currency_precision_of(pair.quote()),
        ) {
            let (base_p, quote_p, pair_p) =
                (u32::from(base_p), u32::from(quote_p), pair.pip_precision);

            let (mul, s) = if quote_p >= (base_p + pair_p) {
                (true, 10_u128.pow(quote_p - base_p - pair_p))
            } else {
                (false, 10_u128.pow(base_p + pair_p - quote_p))
            };

            // Can overflow
            let ap = amount.saturated_into::<u128>() * price.saturated_into::<u128>();

            let volume = if mul {
                match ap.checked_mul(s) {
                    Some(r) => r,
                    None => panic!("amount * price * precision overflow"),
                }
            } else {
                ap / s
            };

            if !volume.is_zero() {
                if volume < u128::from(u64::max_value()) {
                    return Ok((volume as u64).saturated_into());
                } else {
                    panic!("the value of converted quote currency definitely less than u64::max_value()")
                }
            }
        } else {
            return Err(Error::<T>::InvalidTradingPairAsset);
        }

        Err(Error::<T>::VolumeTooSmall)
    }
}
