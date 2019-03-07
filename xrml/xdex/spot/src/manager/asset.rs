// Copyright 2019 Chainpool.
//! This module handles all the asset related actions.

use super::*;

impl<T: Trait> Module<T> {
    /// 转移 maker和taker中的资产
    /// Delivery asset to maker and taker respectively.
    pub(crate) fn delivery_asset_to_each_other(
        maker_order_direction: OrderDirection,
        pair: &TradingPair,
        amount: T::Balance,
        price: T::Price,
        maker_order: &mut OrderDetails<T>,
        taker_order: &mut OrderDetails<T>,
    ) -> Result {
        let maker = maker_order.submitter();
        let taker = taker_order.submitter();

        match maker_order_direction {
            OrderDirection::Sell => {
                //卖家先解锁first token 并move给买家，
                let maker_back_token: &Token = &pair.currency_pair.0;
                let maker_back_amount: T::Balance = amount;
                maker_order.remaining = maker_order
                    .remaining
                    .checked_sub(&maker_back_amount)
                    .unwrap_or_default();

                Self::apply_delivery(&maker_back_token, maker_back_amount, &maker, &taker)?;

                //计算买家的数量，解锁second,并move 给卖家
                let taker_back_token: &Token = &pair.currency_pair.1;
                let taker_back_amount: T::Balance =
                    Self::convert_to_counter_currency(amount, price, &pair).unwrap_or(Zero::zero());
                taker_order.remaining = taker_order
                    .remaining
                    .checked_sub(&taker_back_amount)
                    .unwrap_or_default();

                Self::apply_delivery(&taker_back_token, taker_back_amount, &taker, &maker)?;
            }
            OrderDirection::Buy => {
                //买先解锁second token 并move给卖家，和手续费账户
                let maker_back_token: &Token = &pair.currency_pair.1;
                let maker_back_amount: T::Balance =
                    Self::convert_to_counter_currency(amount, price, &pair).unwrap_or(Zero::zero());
                maker_order.remaining = maker_order
                    .remaining
                    .checked_sub(&maker_back_amount)
                    .unwrap_or_default();

                Self::apply_delivery(&maker_back_token, maker_back_amount, &maker, &taker)?;
                //计算卖家的数量，解锁second,并move 给买家,和手续费账户
                let taker_back_token: &Token = &pair.currency_pair.0;
                let taker_back_amount: T::Balance = As::sa(amount.as_());
                taker_order.remaining = taker_order
                    .remaining
                    .checked_sub(&taker_back_amount)
                    .unwrap_or_default();

                Self::apply_delivery(&taker_back_token, taker_back_amount, &taker, &maker)?;
            }
        }

        Ok(())
    }

    /// Actually move someone's ReservedDexSpot token to another one's Free
    pub(crate) fn apply_delivery(
        token: &Token,
        value: T::Balance,
        from: &T::AccountId,
        to: &T::AccountId,
    ) -> Result {
        <xassets::Module<T>>::move_balance(
            token,
            from,
            xassets::AssetType::ReservedDexSpot,
            to,
            xassets::AssetType::Free,
            value,
        )
        .map_err(|e| e.info())
    }

    /// Reserve the token for putting order if account has enough balance.
    pub(crate) fn try_put_order_reserve(
        who: &T::AccountId,
        pair: &TradingPair,
        direction: &OrderDirection,
        amount: T::Balance,
        price: T::Price,
    ) -> result::Result<T::Balance, &'static str> {
        let (token, remaining) =
            if let Some(sum) = Self::convert_to_counter_currency(amount, price, pair) {
                match *direction {
                    OrderDirection::Buy => {
                        Self::has_enough_balance(who, pair.currency_pair.counter_as_ref(), sum)?
                    }
                    OrderDirection::Sell => {
                        Self::has_enough_balance(who, pair.currency_pair.base_as_ref(), amount)?
                    }
                }
            } else {
                return Err("amount*price too small");
            };

        Self::apply_put_order_reserve(who, token, remaining)?;

        Ok(remaining)
    }

    /// Actually reserve tokens required by putting order.
    fn apply_put_order_reserve(who: &T::AccountId, token: &Token, value: T::Balance) -> Result {
        <xassets::Module<T>>::move_balance(
            token,
            who,
            xassets::AssetType::Free,
            who,
            xassets::AssetType::ReservedDexSpot,
            value,
        )
        .map_err(|e| e.info())
    }

    pub(crate) fn cancel_order_unreserve(
        who: &T::AccountId,
        token: &Token,
        value: T::Balance,
    ) -> Result {
        <xassets::Module<T>>::move_balance(
            token,
            who,
            xassets::AssetType::ReservedDexSpot,
            who,
            xassets::AssetType::Free,
            value,
        )
        .map_err(|e| e.info())
    }

    /// See if the account has enough required token.
    pub(crate) fn has_enough_balance<'a>(
        who: &T::AccountId,
        token: &'a Token,
        required: T::Balance,
    ) -> result::Result<(&'a Token, T::Balance), &'static str> {
        if <xassets::Module<T>>::free_balance(who, token) < required {
            return Err("Can't apply put order if transactor's free token too low");
        }

        Ok((token, required))
    }
}
