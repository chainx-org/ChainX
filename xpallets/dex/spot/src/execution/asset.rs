//! This module handles all the asset related operations in Spot.
//!
//! Copyright 2020 Chainpool.

use super::*;
use xpallet_assets::AssetType::{self, Free, ReservedDexSpot};

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
            <T as Trait>::Currency::transfer(from, to, value, ExistenceRequirement::KeepAlive)?;
            NativeReserves::<T>::mutate(from, |reserved| *reserved -= value);
        } else {
            Self::move_asset(asset_id, from, ReservedDexSpot, to, Free, value)?;
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
            <T as Trait>::Currency::reserve(who, value)?;
            NativeReserves::<T>::mutate(who, |reserved| *reserved += value);
        } else {
            ensure!(
                <xpallet_assets::Module<T>>::free_balance_of(who, &asset_id).saturated_into()
                    >= value.saturated_into::<u128>(),
                Error::<T>::InsufficientBalance
            );
            Self::move_asset(asset_id, who, Free, who, ReservedDexSpot, value)?;
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
        if asset_id == xpallet_protocol::PCX {
            <T as Trait>::Currency::unreserve(who, value);
            NativeReserves::<T>::mutate(who, |reserved| *reserved -= value);
            Ok(())
        } else {
            Self::move_asset(asset_id, who, ReservedDexSpot, who, Free, value)
        }
    }

    /// Refund the remaining reserved asset when the order is fulfilled.
    #[inline]
    pub(crate) fn refund_reserved_dex_spot(
        who: &T::AccountId,
        asset_id: AssetId,
        remaining: BalanceOf<T>,
    ) {
        if asset_id == xpallet_protocol::PCX {
            <T as Trait>::Currency::unreserve(who, remaining);
            NativeReserves::<T>::mutate(who, |reserved| *reserved -= remaining);
        } else {
            let _ = Self::move_asset(asset_id, who, ReservedDexSpot, who, Free, remaining);
        }
    }

    /// Wrap the move_asset function in xassets module.
    fn move_asset(
        asset_id: AssetId,
        from: &T::AccountId,
        from_ty: AssetType,
        to: &T::AccountId,
        to_ty: AssetType,
        amount: BalanceOf<T>,
    ) -> DispatchResult {
        let value = amount.saturated_into::<u128>();
        <xpallet_assets::Module<T>>::move_balance(
            &asset_id,
            from,
            from_ty,
            to,
            to_ty,
            value.saturated_into(),
        )
        .map_err(|_| DispatchError::Other("Unexpected asset error"))
    }
}
