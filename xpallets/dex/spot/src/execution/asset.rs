//! This module handles all the asset related operations in Spot.
//!
//! Copyright 2020 Chainpool.

use super::*;
use xpallet_assets::AssetType::{self, ReservedDexSpot, Usable};

impl<T: Trait> Module<T> {
    /// Delivery the assets to maker and taker respectively when executing the order.
    pub(super) fn delivery_asset_to_each_other(
        maker_order_side: Side,
        pair: &TradingPairProfile,
        turnover: BalanceOf<T>,
        price: T::Price,
        maker_order: &mut OrderInfo<T>,
        taker_order: &mut OrderInfo<T>,
    ) -> result::Result<(BalanceOf<T>, BalanceOf<T>), DispatchError> {
        let maker = &maker_order.submitter();
        let taker = &taker_order.submitter();

        let base = pair.base();
        let quote = pair.quote();

        let turnover_in_quote =
            Self::convert_base_to_quote(turnover, price, pair).unwrap_or_else(|_| Zero::zero());

        match maker_order_side {
            Side::Sell => {
                // maker(seller): unreserve the base currency and move to the taker.
                // taker(buyer): unreserve the quote currency and move to the maker.
                let maker_turnover_amount = turnover;
                let taker_turnover_amount = turnover_in_quote;

                Self::apply_delivery(base, maker_turnover_amount, maker, taker)?;
                Self::apply_delivery(quote, taker_turnover_amount, taker, maker)?;

                Ok((maker_turnover_amount, taker_turnover_amount))
            }
            Side::Buy => {
                // maker(buyer): unreserve the quote currency and move to the taker.
                // taker(seller): unreserve the base currency and move to the maker.
                let maker_turnover_amount = turnover_in_quote;
                let taker_turnover_amount = turnover;

                Self::apply_delivery(base, taker_turnover_amount, taker, maker)?;
                Self::apply_delivery(quote, maker_turnover_amount, maker, taker)?;

                Ok((maker_turnover_amount, taker_turnover_amount))
            }
        }
    }

    /// Actually move someone's ReservedDexSpot asset_id to another one's Free.
    #[inline]
    fn apply_delivery(
        asset_id: AssetId,
        value: BalanceOf<T>,
        from: &T::AccountId,
        to: &T::AccountId,
    ) -> DispatchResult {
        if asset_id == xpallet_protocol::PCX {
            <T as xpallet_assets::Trait>::Currency::unreserve(from, value);
            <T as xpallet_assets::Trait>::Currency::transfer(
                from,
                to,
                value,
                ExistenceRequirement::KeepAlive,
            )?;
            NativeReserves::<T>::mutate(from, |reserved| *reserved -= value);
        } else {
            Self::move_asset(asset_id, from, ReservedDexSpot, to, Usable, value)?;
        }
        Ok(())
    }

    /// Actually reserve the asset locked by putting order.
    pub(crate) fn put_order_reserve(
        who: &T::AccountId,
        asset_id: AssetId,
        value: BalanceOf<T>,
    ) -> DispatchResult {
        if asset_id == xpallet_protocol::PCX {
            <T as xpallet_assets::Trait>::Currency::reserve(who, value)?;
            NativeReserves::<T>::mutate(who, |reserved| *reserved += value);
        } else {
            ensure!(
                <xpallet_assets::Module<T>>::usable_balance(who, &asset_id) >= value,
                Error::<T>::InsufficientBalance
            );
            Self::move_asset(asset_id, who, Usable, who, ReservedDexSpot, value)?;
        }
        Ok(())
    }

    /// Unreserve the locked balances in Spot in general.
    fn generic_unreserve(
        who: &T::AccountId,
        asset_id: AssetId,
        value: BalanceOf<T>,
    ) -> DispatchResult {
        if asset_id == xpallet_protocol::PCX {
            <T as xpallet_assets::Trait>::Currency::unreserve(who, value);
            NativeReserves::<T>::mutate(who, |reserved| *reserved -= value);
        } else {
            Self::move_asset(asset_id, who, ReservedDexSpot, who, Usable, value)?;
        }
        Ok(())
    }

    /// Unreserve the locked asset when the order is canceled.
    #[inline]
    pub(crate) fn cancel_order_unreserve(
        who: &T::AccountId,
        asset_id: AssetId,
        value: BalanceOf<T>,
    ) -> DispatchResult {
        Self::generic_unreserve(who, asset_id, value)
    }

    /// Refund the remaining reserved asset when the order is fulfilled.
    #[inline]
    pub(crate) fn refund_reserved_dex_spot(
        who: &T::AccountId,
        asset_id: AssetId,
        remaining: BalanceOf<T>,
    ) {
        let _ = Self::generic_unreserve(who, asset_id, remaining);
    }

    /// Wrap the move_balance function in xassets module.
    fn move_asset(
        asset_id: AssetId,
        from: &T::AccountId,
        from_ty: AssetType,
        to: &T::AccountId,
        to_ty: AssetType,
        value: BalanceOf<T>,
    ) -> DispatchResult {
        <xpallet_assets::Module<T>>::move_balance(&asset_id, from, from_ty, to, to_ty, value)
            .map_err(|_| DispatchError::Other("Unexpected asset error"))
    }
}
