// Copyright 2018 Chainpool.

#![allow(clippy::ptr_arg)]
// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

mod manager;
mod mock;
mod tests;

#[cfg(feature = "std")]
use chrono::prelude::*;
use parity_codec::Codec;

// Substrate
use primitives::traits::{
    MaybeSerializeDebug, Member, SaturatedConversion, SimpleArithmetic, Zero,
};
use rstd::{
    cmp,
    convert::{TryFrom, TryInto},
    prelude::*,
    result,
};
use support::{
    decl_event, decl_module, decl_storage, dispatch::Result, ensure, Parameter, StorageMap,
    StorageValue,
};
use system::ensure_signed;

// ChainX
use xassets::{AssetType::ReservedDexSpot, ChainT, OnAssetRegisterOrRevoke, Token};
use xsupport::info;
use Side::{Buy, Sell};

pub use self::manager::types::*;

const MAX_BACKLOG_ORDER: usize = 1000;

pub type OrderInfo<T> = Order<
    TradingPairIndex,
    <T as system::Trait>::AccountId,
    <T as xassets::Trait>::Balance,
    <T as Trait>::Price,
    <T as system::Trait>::BlockNumber,
>;

pub type HandicapInfo<T> = Handicap<<T as Trait>::Price>;

pub trait Trait: xassets::Trait + xbitcoin::Trait + xsdot::Trait {
    type Price: Parameter
        + Member
        + SimpleArithmetic
        + From<u64>
        + Into<u64>
        + TryInto<u64>
        + TryFrom<u64>
        + From<Self::Balance>
        + Codec
        + Default
        + Copy
        + MaybeSerializeDebug
        + Zero;
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event<T>() = default;

        pub fn put_order(
            origin,
            pair_index: TradingPairIndex,
            order_type: OrderType,
            side: Side,
            amount: T::Balance,
            price: T::Price
        ) -> Result {
            let who = ensure_signed(origin)?;

            ensure!(!price.is_zero(), "Price can't be zero");
            ensure!(!amount.is_zero(), "Amount can't be zero");
            ensure!(order_type == OrderType::Limit, "Only support Limit order for now");

            let pair = Self::trading_pair(pair_index)?;

            ensure!(pair.online, "The trading pair must be online");
            ensure!(
                (price.into() % 10_u64.pow(pair.tick_precision)).is_zero(),
                "Price must be an integer multiple of the tick precision"
            );

            Self::is_within_quotation_range(price, side, pair_index)?;
            Self::has_too_many_backlog_orders(pair_index, price, side)?;

            // Reserve the token according to the order side.
            let (reserve_token, reserve_amount) = match side {
                Buy => (pair.quote_as_ref(), Self::convert_base_to_quote(amount, price, &pair)?),
                Sell => (pair.base_as_ref(), amount),
            };

            Self::put_order_reserve(&who, reserve_token, reserve_amount)?;

            Self::apply_put_order(who, pair_index, order_type, side, amount, price, reserve_amount)
        }

        pub fn cancel_order(origin, pair_index: TradingPairIndex, order_index: OrderIndex) -> Result {
            let who = ensure_signed(origin)?;

            Self::check_cancel_order(&who, pair_index, order_index)?;
            Self::apply_cancel_order(&who, pair_index, order_index)
        }

        fn set_cancel_order(who: T::AccountId, pair_index: TradingPairIndex, order_index: OrderIndex) -> Result {
            Self::check_cancel_order(&who, pair_index, order_index)?;
            Self::apply_cancel_order(&who, pair_index, order_index)
        }

        fn set_handicap(pair_index: TradingPairIndex, highest_bid: T::Price, lowest_offer: T::Price) {
            <HandicapOf<T>>::insert(pair_index, HandicapInfo::<T>::new(highest_bid, lowest_offer));
            info!(
                    "[set_handicap] pair_index: {:?}, highest_bid: {:?}, lowest_offer: {:?}",
                    pair_index,
                    highest_bid,
                    lowest_offer,
                );
        }

        /// Refund the accumulative locked asset due to the loss of accuracy of Self::convert_base_to_quote().
        fn refund_locked(who: T::AccountId, token: Token) {
            let btc = <xbitcoin::Module<T> as ChainT>::TOKEN;
            let sdot = <xsdot::Module<T> as ChainT>::TOKEN;
            ensure!(token == btc.to_vec() || token == sdot.to_vec(), "Only BTC and SDOT could have this refunding issue.");

            // The max order count won't exceed 10000 when this issue is resolved.
            let order_count = cmp::min(10000, Self::order_count_of(&who));
            for index in 0..order_count {
                if Self::order_info_of((who.clone(), index)).is_some() {
                    return Err("Cannot refund if the user's open orders are not empty.");
                }
            }
            let value = <xassets::Module<T>>::asset_balance_of(&who, &token, ReservedDexSpot);
            if !value.is_zero() {
                Self::refund_reserved_dex_spot(&who, &token, value);
                info!("[refund_locked] who: {:?}, token: {:?}, value: {:?}", who, token, value);
            }
        }

        pub fn update_trading_pair(pair_index: TradingPairIndex, tick_precision: Option<u32>, new_online: bool) {
            info!(
                "[update_trading_pair] pair_index: {}, tick_precision: {:?}, online:{}",
                pair_index, tick_precision, new_online
            );

            let pair = Self::trading_pair(pair_index)?;
            if let Some(new) = tick_precision {
                ensure!(new >= pair.tick_precision, "tick_precision can not less than the one of pair!");
            }

            <TradingPairOf<T>>::mutate(pair_index, |pair| {
                if let Some(pair) = pair {
                    if let Some(new) = tick_precision {
                        pair.tick_precision = new;
                    }
                    pair.online = new_online;
                }
            });

            Self::update_order_pair_event(&pair);
        }
    }
}

decl_event!(
    pub enum Event<T> where
        <T as system::Trait>::AccountId,
        <T as system::Trait>::BlockNumber,
        <T as xassets::Trait>::Balance,
        <T as Trait>::Price
    {
        UpdateOrder(AccountId, OrderIndex, Balance, BlockNumber, OrderStatus, Balance, Vec<TradeHistoryIndex>),

        PutOrder(AccountId, OrderIndex, TradingPairIndex, OrderType, Price, Side, Balance, BlockNumber),

        FillOrder(TradeHistoryIndex, TradingPairIndex, Price, AccountId, AccountId, OrderIndex, OrderIndex, Balance, u64),

        UpdateOrderPair(TradingPairIndex, CurrencyPair, u32, u32, bool),

        PriceVolatility(u32),
    }
);

decl_storage! {
    trait Store for Module<T: Trait> as XSpot {

        /// How many trading pairs so far.
        pub TradingPairCount get(trading_pair_count): TradingPairIndex ;
        /// Essential info of the trading pair.
        pub TradingPairOf get(trading_pair_of): map TradingPairIndex => Option<TradingPair>;
        /// (latest price, average price, last last update height) of trading pair
        pub TradingPairInfoOf get(trading_pair_info_of): map TradingPairIndex => Option<(T::Price, T::Price, T::BlockNumber)>;
        /// Total transactions has been made for a trading pair.
        pub TradeHistoryIndexOf get(trade_history_index_of): map TradingPairIndex => TradeHistoryIndex;

        /// Total orders has made by an account.
        pub OrderCountOf get(order_count_of): map T::AccountId => OrderIndex;
        /// Details of the order given account and his order ID
        pub OrderInfoOf get(order_info_of): map (T::AccountId, OrderIndex) => Option<OrderInfo<T>>;

        /// All the account and his order number given a certain trading pair and price.
        pub QuotationsOf get(quotations_of) : map (TradingPairIndex, T::Price) => Vec<(T::AccountId, OrderIndex)>;

        /// TradingPairIndex => (highest_bid, lowest_offer)
        pub HandicapOf get(handicap_of): map TradingPairIndex => HandicapInfo<T>;

        /// Price volatility
        pub PriceVolatility get(price_volatility) config(): u32;
    }
}

impl<T: Trait> Module<T> {
    /// Public mutables
    pub fn add_trading_pair(
        currency_pair: CurrencyPair,
        pip_precision: u32,
        tick_precision: u32,
        price: T::Price,
        online: bool,
    ) -> Result {
        info!(
            "[add_trading_pair] currency_pair: {:?}, point_precision: {:}, tick_precision: {:}, price: {:?}, online: {:}",
            currency_pair,
            pip_precision,
            tick_precision,
            price,
            online
        );

        ensure!(
            Self::get_trading_pair_by_currency_pair(&currency_pair).is_none(),
            "The trading pair already exists."
        );

        let index = <TradingPairCount<T>>::get();

        let pair = TradingPair {
            index,
            currency_pair,
            pip_precision,
            tick_precision,
            online,
        };

        <TradingPairOf<T>>::insert(index, &pair);
        <TradingPairInfoOf<T>>::insert(index, (price, price, <system::Module<T>>::block_number()));
        <TradingPairCount<T>>::put(index + 1);

        Self::update_order_pair_event(&pair);

        Ok(())
    }

    pub fn get_trading_pair_by_currency_pair(currency_pair: &CurrencyPair) -> Option<TradingPair> {
        let pair_count = <TradingPairCount<T>>::get();
        for i in 0..pair_count {
            if let Some(pair) = <TradingPairOf<T>>::get(i) {
                if pair.base() == currency_pair.base() && pair.quote() == currency_pair.quote() {
                    return Some(pair);
                }
            }
        }
        None
    }

    pub fn set_price_volatility(price_volatility: u32) -> Result {
        info!(
            "[set_price_volatility] price_volatility: {:}",
            price_volatility
        );
        ensure!(price_volatility < 100, "Price volatility must be less 100!");
        <PriceVolatility<T>>::put(price_volatility);
        Self::deposit_event(RawEvent::PriceVolatility(price_volatility));
        Ok(())
    }

    /// Return the price of unit token measured by PCX, including the precision of PCX.
    /// For example, 1 BTC = 10000 PCX, shoule return 10000 * 10^pcx_precision
    ///
    /// if the trading pair is XXX/PCX, return:
    ///     trading_pair.aver_asset_price * 10^pcx_precision / 10^trading_pair.pip_precision
    ///
    /// if the trading pair is PCX/XXX:, return:
    ///     trading_pair.pip_precision * 10^pcx_precision / trading_pair.aver_asset_price
    pub fn aver_asset_price(token: &Token) -> Option<T::Balance> {
        let pcx = <xassets::Module<T> as ChainT>::TOKEN.to_vec();
        let pcx_asset = <xassets::Module<T>>::get_asset(&pcx).expect("PCX definitely exist.");
        let pcx_precision = 10_u128.pow(u32::from(pcx_asset.precision()));

        let pair_len = <TradingPairCount<T>>::get();
        for i in 0..pair_len {
            if let Some(pair) = <TradingPairOf<T>>::get(i) {
                let pip_precision = 10_u128.pow(pair.pip_precision);

                // XXX/PCX
                if pair.base().eq(token) && pair.quote().eq(&pcx) {
                    if let Some((_, aver, _)) = <TradingPairInfoOf<T>>::get(i) {
                        let price = match (u128::from(aver.into())).checked_mul(pcx_precision) {
                            Some(x) => (x / pip_precision) as u64,
                            None => panic!("aver * pow_pcx_precision overflow"),
                        };

                        return Some(price.into());
                    }
                // PCX/XXX
                } else if pair.base().eq(&pcx) && pair.quote().eq(token) {
                    if let Some((_, aver, _)) = <TradingPairInfoOf<T>>::get(i) {
                        let price = match pip_precision.checked_mul(pcx_precision) {
                            Some(x) => (x / (u128::from(aver.into()))) as u64,
                            None => panic!("pow_pcx_precision * pow_pair_precision overflow"),
                        };

                        return Some(price.into());
                    }
                }
            }
        }

        None
    }

    /// Internal mutables
    fn apply_put_order(
        who: T::AccountId,
        pair_index: TradingPairIndex,
        order_type: OrderType,
        side: Side,
        amount: T::Balance,
        price: T::Price,
        reserve_amount: T::Balance,
    ) -> Result {
        info!(
            "transactor:{:?}, pair_index:{:}, type:{:?}, side:{:?}, amount:{:?}, price:{:?}",
            who, pair_index, order_type, side, amount, price
        );

        let pair = Self::trading_pair(pair_index)?;

        let mut order = Self::inject_order(
            who,
            pair_index,
            price,
            order_type,
            side,
            amount,
            reserve_amount,
        );

        Self::try_match_order(&pair, &mut order, pair_index, side, price);

        Ok(())
    }

    fn check_cancel_order(
        who: &T::AccountId,
        pair_index: TradingPairIndex,
        order_index: OrderIndex,
    ) -> Result {
        let pair = Self::trading_pair(pair_index)?;
        ensure!(
            pair.online,
            "Can't cancel order if the trading pair is already offline"
        );

        let order_status = match Self::order_info_of(&(who.clone(), order_index)) {
            Some(x) => x.status,
            None => return Err("The order doesn't exist"),
        };
        ensure!(
            order_status == OrderStatus::ZeroFill || order_status == OrderStatus::ParitialFill,
            "Only ZeroFill and ParitialFill order can be canceled"
        );

        Ok(())
    }

    fn apply_cancel_order(
        who: &T::AccountId,
        pair_index: TradingPairIndex,
        order_index: OrderIndex,
    ) -> Result {
        info!(
            "[cancel_order] transactor: {:?}, pair_index:{:}, order_index:{:}",
            who, pair_index, order_index
        );

        let pair = Self::trading_pair(pair_index)?;
        let mut order = Self::order_info_of(&(who.clone(), order_index))
            .expect("We have ensured the order exists.");

        Self::update_order_and_unreserve_on_cancel(&mut order, &pair, who)?;

        Self::kill_order(
            pair_index,
            order.price(),
            who.clone(),
            order_index,
            pair,
            order.side(),
        );

        Ok(())
    }

    /// In order to get trading pair easier.
    fn trading_pair(pair_index: TradingPairIndex) -> result::Result<TradingPair, &'static str> {
        <TradingPairOf<T>>::get(pair_index).ok_or("The order pair doesn't exist.")
    }
}

impl<T: Trait> OnAssetRegisterOrRevoke for Module<T> {
    fn on_register(_token: &Token, _is_psedu_intention: bool) -> Result {
        Ok(())
    }

    fn on_revoke(token: &Token) -> Result {
        let pair_len = <TradingPairCount<T>>::get();
        for i in 0..pair_len {
            if let Some(mut pair) = <TradingPairOf<T>>::get(i) {
                if pair.base().eq(token) || pair.quote().eq(token) {
                    pair.online = false;
                    <TradingPairOf<T>>::insert(i, &pair);
                    Self::update_order_pair_event(&pair);
                }
            }
        }
        Ok(())
    }
}
