// Copyright 2019 Chainpool.
//! This module handles all the asset related actions.

use super::*;
use xassets::AssetType::{self, Free, ReservedDexSpot};
use xsupport::error;

impl<T: Trait> Module<T> {
    /// Delivery asset to maker and taker respectively when execute the order.
    pub(super) fn delivery_asset_to_each_other(
        maker_order_side: Side,
        pair: &TradingPair,
        turnover: T::Balance,
        price: T::Price,
        maker_order: &mut OrderInfo<T>,
        taker_order: &mut OrderInfo<T>,
    ) -> result::Result<(T::Balance, T::Balance), &'static str> {
        let maker = &maker_order.submitter();
        let taker = &taker_order.submitter();

        let base = pair.base_as_ref();
        let quote = pair.quote_as_ref();

        let (maker_turnover_amount, taker_turnover_amount) = match maker_order_side {
            Sell => {
                // maker(seller): unserve the base currency and move to the taker.
                // taker(buyer): unserve the quote currency and move to the maker.
                let maker_turnover_amount = turnover;
                let taker_turnover_amount =
                    Self::convert_base_to_quote(turnover, price, pair).unwrap_or(Zero::zero());

                Self::apply_delivery(base, maker_turnover_amount, maker, taker)?;
                Self::apply_delivery(quote, taker_turnover_amount, taker, maker)?;

                (maker_turnover_amount, taker_turnover_amount)
            }
            Buy => {
                // maker(buyer): unserve the quote currency and move to the taker.
                // taker(seller): unserve the base currency and move to the maker.
                let maker_turnover_amount =
                    Self::convert_base_to_quote(turnover, price, pair).unwrap_or(Zero::zero());
                let taker_turnover_amount = turnover;

                Self::apply_delivery(quote, maker_turnover_amount, maker, taker)?;
                Self::apply_delivery(base, taker_turnover_amount, taker, maker)?;

                (maker_turnover_amount, taker_turnover_amount)
            }
        };

        Ok((maker_turnover_amount, taker_turnover_amount))
    }

    /// Actually move someone's ReservedDexSpot token to another one's Free
    #[inline]
    fn apply_delivery(
        token: &Token,
        value: T::Balance,
        from: &T::AccountId,
        to: &T::AccountId,
    ) -> Result {
        Self::move_balance(token, from, ReservedDexSpot, to, Free, value)
    }

    /// Actually reserve tokens required by putting order.
    pub(crate) fn put_order_reserve(
        who: &T::AccountId,
        token: &Token,
        value: T::Balance,
    ) -> Result {
        if <xassets::Module<T>>::free_balance_of(who, token) < value {
            return Err("Can not put order if transactor's free token too low");
        }

        Self::move_balance(token, who, Free, who, ReservedDexSpot, value)
    }

    #[inline]
    pub(crate) fn cancel_order_unreserve(
        who: &T::AccountId,
        token: &Token,
        value: T::Balance,
    ) -> Result {
        Self::move_balance(token, who, ReservedDexSpot, who, Free, value)
    }

    fn move_balance(
        token: &Token,
        from: &T::AccountId,
        from_type: AssetType,
        to: &T::AccountId,
        to_type: AssetType,
        value: T::Balance,
    ) -> Result {
        let _ = <xassets::Module<T>>::move_balance(token, from, from_type, to, to_type, value)
            .map_err(|e| {
                error!(
                    "[move_balance] Fail to move {:?} from {:?}'s {:?} to {:?}'s {:?}",
                    value, from, from_type, to, to_type
                );
                e.info()
            })?;
        Ok(())
    }
}
