// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

mod asset;
mod order;
mod state;

use xp_logging::debug;

use super::*;
use crate::types::*;

impl<T: Trait> Module<T> {
    fn check_bid_price(
        quote: T::Price,
        lowest_ask: T::Price,
        fluctuation: T::Price,
    ) -> result::Result<(), Error<T>> {
        debug!(
            "[check_bid_price] quote: {:?}, lowest_ask: {:?}, fluctuation: {:?}",
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

    fn check_ask_price(
        quote: T::Price,
        highest_bid: T::Price,
        fluctuation: T::Price,
    ) -> result::Result<(), Error<T>> {
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
    pub(crate) fn is_valid_quote(
        quote: T::Price,
        side: Side,
        pair_id: TradingPairId,
    ) -> result::Result<(), Error<T>> {
        let handicap = <HandicapOf<T>>::get(pair_id);
        let (lowest_ask, highest_bid) = (handicap.lowest_ask, handicap.highest_bid);

        let pair = Self::trading_pair(pair_id)?;
        let fluctuation = pair.calc_fluctuation::<T>().saturated_into();

        match side {
            Side::Buy => Self::check_bid_price(quote, lowest_ask, fluctuation),
            Side::Sell => Self::check_ask_price(quote, highest_bid, fluctuation),
        }
    }

    /// Returns true if there are already too many orders at the `price` and `side` for a trading pair.
    pub(crate) fn has_too_many_backlog_orders(
        pair_id: TradingPairId,
        price: T::Price,
        side: Side,
    ) -> result::Result<(), Error<T>> {
        let quotations = <QuotationsOf<T>>::get(pair_id, price);
        if quotations.len() >= MAX_BACKLOG_ORDER {
            let (who, order_id) = &quotations[0];
            if let Some(order) = <OrderInfoOf<T>>::get(who, order_id) {
                if order.side() == side {
                    return Err(Error::<T>::TooManyBacklogOrders);
                }
            }
        }

        Ok(())
    }

    fn currency_decimals_of(asset_id: AssetId) -> Option<u8> {
        <xpallet_assets_registrar::Module<T>>::asset_info_of(asset_id).map(|x| x.decimals())
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
    /// = amount * price * 10^(quote.decimals) / 10^(base.decimals) * 10^(price.decimals)
    /// = amount * price * 10^(quote.decimals - base.decimals - price.decimals)
    pub(crate) fn convert_base_to_quote(
        amount: BalanceOf<T>,
        price: T::Price,
        pair: &TradingPairProfile,
    ) -> result::Result<BalanceOf<T>, Error<T>> {
        if let (Some(base_p), Some(quote_p)) = (
            Self::currency_decimals_of(pair.base()),
            Self::currency_decimals_of(pair.quote()),
        ) {
            let (base_p, quote_p, pair_p) =
                (u32::from(base_p), u32::from(quote_p), pair.pip_decimals);

            let (mul, exp) = if quote_p >= (base_p + pair_p) {
                (true, 10_u128.pow(quote_p - base_p - pair_p))
            } else {
                (false, 10_u128.pow(base_p + pair_p - quote_p))
            };

            // Can overflow
            let ap = amount.saturated_into::<u128>() * price.saturated_into::<u128>();

            let volume = if mul {
                ap.checked_mul(exp)
                    .unwrap_or_else(|| panic!("amount * price * decimals overflow"))
            } else {
                ap / exp // exp can't be zero; qed
            };

            if !volume.is_zero() {
                if volume < u128::max_value() {
                    Ok(volume.saturated_into::<BalanceOf<T>>())
                } else {
                    panic!("the value of converted quote currency definitely less than u128::max_value()")
                }
            } else {
                Err(Error::<T>::VolumeTooSmall)
            }
        } else {
            Err(Error::<T>::InvalidTradingPairAsset)
        }
    }
}
