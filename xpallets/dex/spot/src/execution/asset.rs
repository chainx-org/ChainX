// Copyright 2019-2023 ChainX Project Authors. Licensed under GPL-3.0.

//! This module handles all the asset related operations in Spot.

use super::*;
use xpallet_assets::AssetType::{self, ReservedDexSpot, Usable};

impl<T: Config> Pallet<T> {
    /// Delivery the assets to maker and taker respectively when executing the order.
    pub(super) fn delivery_asset_to_each_other(
        maker_order_side: Side,
        pair: &TradingPairProfile,
        turnover: BalanceOf<T>,
        price: T::Price,
        maker_order: &mut OrderInfo<T>,
        taker_order: &mut OrderInfo<T>,
    ) -> Result<(BalanceOf<T>, BalanceOf<T>), DispatchError> {
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

    /// Returns true if the `asset_id` is native token.
    #[inline]
    fn is_native_asset(asset_id: AssetId) -> bool {
        asset_id == T::NativeAssetId::get()
    }

    /// Move the locked balance in Spot of account `from` to another account's Free.
    #[inline]
    fn apply_delivery(
        asset_id: AssetId,
        value: BalanceOf<T>,
        from: &T::AccountId,
        to: &T::AccountId,
    ) -> DispatchResult {
        if Self::is_native_asset(asset_id) {
            Self::transfer_native_asset(from, to, value)
        } else {
            Self::move_foreign_asset(asset_id, from, ReservedDexSpot, to, Usable, value)
        }
    }

    /// Unreserve the locked balances in Spot in general.
    pub(crate) fn generic_unreserve(
        who: &T::AccountId,
        asset_id: AssetId,
        value: BalanceOf<T>,
    ) -> DispatchResult {
        if Self::is_native_asset(asset_id) {
            <T as xpallet_assets::Config>::Currency::unreserve(who, value);
            NativeReserves::<T>::mutate(who, |reserved| *reserved -= value);
        } else {
            Self::unreserve_foreign_asset(who, asset_id, value)?;
        }
        Ok(())
    }

    /// Actually reserve the asset locked by putting order.
    pub(crate) fn put_order_reserve(
        who: &T::AccountId,
        asset_id: AssetId,
        value: BalanceOf<T>,
    ) -> DispatchResult {
        if Self::is_native_asset(asset_id) {
            <T as xpallet_assets::Config>::Currency::reserve(who, value)?;
            NativeReserves::<T>::mutate(who, |reserved| *reserved += value);
        } else {
            ensure!(
                <xpallet_assets::Pallet<T>>::usable_balance(who, &asset_id) >= value,
                Error::<T>::InsufficientBalance
            );
            Self::move_foreign_asset(asset_id, who, Usable, who, ReservedDexSpot, value)?;
        }
        Ok(())
    }

    /// Transfer some locked native token balance of `from` to another account.
    fn transfer_native_asset(
        from: &T::AccountId,
        to: &T::AccountId,
        value: BalanceOf<T>,
    ) -> DispatchResult {
        // FIXME: https://github.com/paritytech/substrate/issues/7992
        //
        // The account `to` definitely exists so this should always succeed.
        // This is equivalent to unreserve(from, value) + transfer(from, to, value)
        //
        // <T as xpallet_assets::Config>::Currency::repatriate_reserved(
        // from,
        // to,
        // value,
        // frame_support::traits::BalanceStatus::Free,
        // )?;

        <T as xpallet_assets::Config>::Currency::unreserve(from, value);
        <T as xpallet_assets::Config>::Currency::transfer(
            from,
            to,
            value,
            frame_support::traits::ExistenceRequirement::KeepAlive,
        )?;

        NativeReserves::<T>::mutate(from, |reserved| *reserved -= value);
        Ok(())
    }

    /// Move one's foreign asset from the state of `ReservedDexSpot` to `Usable`.
    fn unreserve_foreign_asset(
        who: &T::AccountId,
        asset_id: AssetId,
        value: BalanceOf<T>,
    ) -> DispatchResult {
        Self::move_foreign_asset(asset_id, who, ReservedDexSpot, who, Usable, value)
    }

    /// Wrap the move_balance function in xassets module.
    fn move_foreign_asset(
        asset_id: AssetId,
        from: &T::AccountId,
        from_ty: AssetType,
        to: &T::AccountId,
        to_ty: AssetType,
        value: BalanceOf<T>,
    ) -> DispatchResult {
        <xpallet_assets::Pallet<T>>::move_balance(&asset_id, from, from_ty, to, to_ty, value)
            .map_err(|_| DispatchError::Other("Unexpected error from assets Pallet"))
    }
}
