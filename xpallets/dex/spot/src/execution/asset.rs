// Copyright 2019 Chainpool.
//! This module handles all the asset related operations.

use super::*;
use xpallet_assets::AssetType::{self, Free, ReservedDexSpot};

impl<T: Trait> Module<T> {
    /// Delivery asset to maker and taker respectively when execute the order.
    pub(super) fn delivery_asset_to_each_other(
        maker_order_side: Side,
        pair: &TradingPairProfile,
        turnover: T::Balance,
        price: T::Price,
        maker_order: &mut OrderInfo<T>,
        taker_order: &mut OrderInfo<T>,
    ) -> result::Result<(T::Balance, T::Balance), Error<T>> {
        let maker = &maker_order.submitter();
        let taker = &taker_order.submitter();

        let base = pair.base();
        let quote = pair.quote();

        let (maker_turnover_amount, taker_turnover_amount) = match maker_order_side {
            Side::Sell => {
                // maker(seller): unserve the base currency and move to the taker.
                // taker(buyer): unserve the quote currency and move to the maker.
                let maker_turnover_amount = turnover;
                let taker_turnover_amount = Self::convert_base_to_quote(turnover, price, pair)
                    .unwrap_or_else(|_| Zero::zero());

                Self::apply_delivery(&base, maker_turnover_amount, maker, taker)?;
                Self::apply_delivery(&quote, taker_turnover_amount, taker, maker)?;

                (maker_turnover_amount, taker_turnover_amount)
            }
            Side::Buy => {
                // maker(buyer): unserve the quote currency and move to the taker.
                // taker(seller): unserve the base currency and move to the maker.
                let maker_turnover_amount = Self::convert_base_to_quote(turnover, price, pair)
                    .unwrap_or_else(|_| Zero::zero());
                let taker_turnover_amount = turnover;

                Self::apply_delivery(&quote, maker_turnover_amount, maker, taker)?;
                Self::apply_delivery(&base, taker_turnover_amount, taker, maker)?;

                (maker_turnover_amount, taker_turnover_amount)
            }
        };

        Ok((maker_turnover_amount, taker_turnover_amount))
    }

    /// Actually move someone's ReservedDexSpot token to another one's Free
    #[inline]
    fn apply_delivery(
        token: &AssetId,
        value: T::Balance,
        from: &T::AccountId,
        to: &T::AccountId,
    ) -> Result<T> {
        Self::move_balance(token, from, ReservedDexSpot, to, Free, value)
    }

    /// Actually reserve tokens required by putting order.
    pub(crate) fn put_order_reserve(
        who: &T::AccountId,
        token: &AssetId,
        value: T::Balance,
    ) -> Result<T> {
        if <xpallet_assets::Module<T>>::free_balance_of(who, token) < value {
            return Err(Error::<T>::InsufficientBalance);
        }

        Self::move_balance(token, who, Free, who, ReservedDexSpot, value)
    }

    /// Unreserve the locked asset when the order is canceled.
    #[inline]
    pub(crate) fn cancel_order_unreserve(
        who: &T::AccountId,
        token: &AssetId,
        value: T::Balance,
    ) -> Result<T> {
        Self::move_balance(token, who, ReservedDexSpot, who, Free, value)
    }

    /// Refund the remaining reserved asset when the order is fulfilled.
    #[inline]
    pub(crate) fn refund_reserved_dex_spot(
        who: &T::AccountId,
        token: &AssetId,
        remaining: T::Balance,
    ) {
        let _ = Self::move_balance(token, who, ReservedDexSpot, who, Free, remaining);
    }

    /// Wrap the move_balance function in xassets module.
    fn move_balance(
        token: &AssetId,
        from: &T::AccountId,
        from_type: AssetType,
        to: &T::AccountId,
        to_type: AssetType,
        value: T::Balance,
    ) -> Result<T> {
        <xpallet_assets::Module<T>>::move_balance(
            token, from, from_type, to, to_type, value, true,
        )?;
        Ok(())
    }
}
