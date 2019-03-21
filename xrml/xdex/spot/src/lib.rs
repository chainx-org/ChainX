// Copyright 2018 Chainpool.

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

use parity_codec as codec;

mod manager;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
mod types;

pub use types::*;

#[cfg(feature = "std")]
use chrono::prelude::*;
use codec::Codec;
use primitives::traits::{As, CheckedSub, MaybeSerializeDebug, Member, SimpleArithmetic, Zero};
use rstd::{cmp, prelude::*, result};
use runtime_support::{
    decl_event, decl_module, decl_storage, dispatch::Result, ensure, Parameter, StorageMap,
    StorageValue,
};
use system::ensure_signed;
use xassets::{
    assetdef::{ChainT, Token},
    OnAssetRegisterOrRevoke,
};
use xsupport::info;

const PRICE_MAX_ORDER: usize = 1000;

pub type OrderDetails<T> = Order<
    TradingPairIndex,
    <T as system::Trait>::AccountId,
    <T as balances::Trait>::Balance,
    <T as Trait>::Price,
    <T as system::Trait>::BlockNumber,
>;

pub type FillT<T> = Fill<
    TradingPairIndex,
    <T as system::Trait>::AccountId,
    <T as balances::Trait>::Balance,
    <T as Trait>::Price,
    <T as system::Trait>::BlockNumber,
>;

pub type HandicapT<T> = Handicap<<T as Trait>::Price>;

pub trait Trait: xassets::Trait + timestamp::Trait {
    type Price: Parameter
        + Member
        + SimpleArithmetic
        + Codec
        + Default
        + Copy
        + As<u8>
        + As<u16>
        + As<u32>
        + As<u64>
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
            ordertype: OrderType,
            direction: OrderDirection,
            amount: T::Balance,
            price: T::Price
        ) -> Result {
            let who = ensure_signed(origin)?;

            ensure!(!price.is_zero(), "Price can't be zero");
            ensure!(!amount.is_zero(), "Amount can't be zero");
            ensure!(ordertype == OrderType::Limit, "Only support Limit order for now");

            let pair = Self::trading_pair(&pair_index)?;

            ensure!(pair.online, "Pair must be online");

            let min_unit = 10_u64.pow(pair.unit_precision);
            ensure!(price.as_() >= min_unit, "Price must greater than min_unit");
            ensure!((price.as_() % min_unit).is_zero(), "Price must be an integer multiple of the minimum precision");

            let handicap = Self::is_valid_price(price, &direction, pair_index)?;

            let quotations = <QuotationsOf<T>>::get(&(pair.id, price));
            if quotations.len() >= PRICE_MAX_ORDER {
                if let Some(order) = <OrderInfoOf<T>>::get(&quotations[0]) {
                    if order.direction() == direction {
                        return Err("Too much orders at this price and direction in the trading pair.");
                    }
                }
            }

            Self::apply_put_order(who, pair_index, ordertype, direction, amount, price, handicap)
        }

        pub fn cancel_order(origin, pairid: TradingPairIndex, index: ID) -> Result {
            let who = ensure_signed(origin)?;

            let pair = Self::trading_pair(&pairid)?;
            ensure!(pair.online, "Can't cancel order if the trading pair is already offline");

            let order_status = match Self::order_info_of(&(who.clone(), index)) {
                Some(x) => x.status,
                None => return Err( "The order doesn't exist"),
            };
            ensure!(
                order_status == OrderStatus::ZeroExecuted || order_status == OrderStatus::ParitialExecuted,
                "Only ZeroExecuted and ParitialExecuted order can be canceled"
            );

            Self::apply_cancel_order(&who, pairid, index)
        }

    }
}

decl_event!(
    pub enum Event<T> where
        <T as system::Trait>::AccountId,
        <T as system::Trait>::BlockNumber,
        <T as balances::Trait>::Balance,
        <T as Trait>::Price
    {
        UpdateOrder(AccountId,ID,TradingPairIndex,Price,OrderType,OrderDirection,Balance,Balance,BlockNumber,BlockNumber,OrderStatus,Balance,Vec<ID>),
        FillOrder(ID,TradingPairIndex,Price,AccountId,AccountId,ID,ID, Balance,u64),
        UpdateOrderPair(TradingPairIndex,CurrencyPair,u32,u32,bool),
        PriceVolatility(u32),
        Handicap(TradingPairIndex,Price,OrderDirection),
        RemoveUserQuotations(AccountId,ID),
        RemoveQuotationsSlot(TradingPairIndex,Price),
        FillOrderErr(TradingPairIndex,AccountId,ID),
    }
);

decl_storage! {
    trait Store for Module<T: Trait> as XSpot {

        /// How many trading pairs so far.
        pub TradingPairCount get(trading_pair_count): TradingPairIndex ;
        /// Essential info of the trading pair.
        pub TradingPairOf get(trading_pair_of): map TradingPairIndex => Option<TradingPair>;
        /// (last price, average transaction price, last update height) of trading pair
        pub TradingPairInfoOf get(trading_pair_info_of): map TradingPairIndex => Option<(T::Price, T::Price, T::BlockNumber)>;
        /// Total transactions has been made for a trading pair.
        pub TradeHistoryIndexOf get(trade_history_index_of): map TradingPairIndex => ID;

        /// Total orders has made by an account.
        pub OrderCountOf get(order_count_of): map T::AccountId => ID;
        /// Details of the order given account and his order ID
        pub OrderInfoOf get(order_info_of): map(T::AccountId, ID) => Option<OrderDetails<T>>;

        /// All the account and his order number given a certain trading pair and price.
        pub QuotationsOf get(quotations_of) : map (TradingPairIndex, T::Price) => Vec<(T::AccountId, ID)>;

        /// TradingPairIndex => (Buy, Sell)
        pub HandicapOf get(handicap_of): map TradingPairIndex => Option<HandicapT<T>>;

        /// Price volatility
        pub PriceVolatility get(price_volatility) config(): u32;
    }
}

impl<T: Trait> Module<T> {
    /// Public mutables
    pub fn add_trading_pair(
        currency_pair: CurrencyPair,
        precision: u32,
        unit_precision: u32,
        price: T::Price,
        online: bool,
    ) -> Result {
        info!(
            "currency_pair:{:?}, precision:{:}, unit:{:}, price:{:?}, online:{:}",
            currency_pair, precision, unit_precision, price, online
        );

        ensure!(
            Self::get_trading_pair_by_currency_pair(&currency_pair).is_none(),
            "The trading pair already exists."
        );

        let id = <TradingPairCount<T>>::get();

        let pair = TradingPair {
            id,
            currency_pair,
            precision,
            unit_precision,
            online,
        };

        <TradingPairOf<T>>::insert(id, &pair);
        <TradingPairInfoOf<T>>::insert(id, (price, price, <system::Module<T>>::block_number()));
        <TradingPairCount<T>>::put(id + 1);

        Self::event_pair(&pair);

        Ok(())
    }

    pub fn update_trading_pair(id: TradingPairIndex, unit_precision: u32, online: bool) -> Result {
        info!(
            "update_trading_pair -- pairid: {:}, unit_precision: {:}, online:{:}",
            id, unit_precision, online
        );

        let mut pair = Self::trading_pair(&id)?;

        if unit_precision < pair.unit_precision {
            return Err("unit_precision error!");
        }
        pair.unit_precision = unit_precision;
        pair.online = online;

        <TradingPairOf<T>>::insert(id, &pair);
        Self::event_pair(&pair);

        Ok(())
    }

    /// base currency/counter currency
    pub fn get_trading_pair_by_currency_pair(currency_pair: &CurrencyPair) -> Option<TradingPair> {
        let pair_count = <TradingPairCount<T>>::get();
        for i in 0..pair_count {
            if let Some(pair) = <TradingPairOf<T>>::get(i) {
                let base = pair.currency_pair.base();
                let counter = pair.currency_pair.counter();
                if base == currency_pair.base() && counter == currency_pair.counter() {
                    return Some(pair.clone());
                }
            }
        }
        None
    }

    pub fn set_price_volatility(price_volatility: u32) -> Result {
        info!("set_price_volatility: {:}", price_volatility);
        ensure!(price_volatility < 100, "Price volatility must be less 100!");
        <PriceVolatility<T>>::put(price_volatility);
        Self::deposit_event(RawEvent::PriceVolatility(price_volatility));
        Ok(())
    }

    /// 返回以PCX计价的"单位"token的价格，已含pcx精度
    /// 譬如1BTC=10000PCX，返回的是10000*（10.pow(pcx精度))
    ///
    /// 如果交易对ID是XXX/PCX，则：
    /// 返回：(交易对Map[交易对ID].平均价*（10^PCX精度)) / 10^报价精度
    ///
    /// 如果交易对ID是PCX/XXX，则：
    /// 返回：(10^交易对Map[交易对ID].报价精度*（10^PCX精度)) / 平均价
    pub fn aver_asset_price(token: &Token) -> Option<T::Balance> {
        let pcx = <xassets::Module<T> as ChainT>::TOKEN.to_vec();
        let pcx_asset = <xassets::Module<T>>::get_asset(&pcx).expect("PCX definitely exist.");
        let pcx_precision = 10_u128.pow(pcx_asset.precision() as u32);

        let pair_len = <TradingPairCount<T>>::get();
        for i in 0..pair_len {
            if let Some(pair) = <TradingPairOf<T>>::get(i) {
                let pair_precision = 10_u128.pow(pair.precision.as_());
                let currency_pair = pair.currency_pair.clone();

                if currency_pair.base().eq(token) && currency_pair.counter().eq(&pcx) {
                    if let Some((_, aver, _)) = <TradingPairInfoOf<T>>::get(i) {
                        let price = match (aver.as_() as u128).checked_mul(pcx_precision) {
                            Some(x) => x / pair_precision,
                            None => panic!("aver * pow_pcx_precision overflow"),
                        };

                        return Some(T::Balance::sa(price as u64));
                    }
                } else if currency_pair.base().eq(&pcx) && currency_pair.counter().eq(token) {
                    if let Some((_, aver, _)) = <TradingPairInfoOf<T>>::get(i) {
                        let price = match pcx_precision.checked_mul(pair_precision) {
                            Some(x) => x / (aver.as_() as u128),
                            None => panic!("pow_pcx_precision * pow_pair_precision overflow"),
                        };

                        return Some(T::Balance::sa(price as u64));
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
        ordertype: OrderType,
        direction: OrderDirection,
        amount: T::Balance,
        price: T::Price,
        handicap: HandicapT<T>,
    ) -> Result {
        info!(
            "transactor:{:?},pairid:{:},ordertype:{:?},direction:{:?},amount:{:?},price:{:?}",
            who, pair_index, ordertype, direction, amount, price
        );

        let pair = Self::trading_pair(&pair_index)?;
        let remaining = Self::try_put_order_reserve(&who, &pair, &direction, amount, price)?;

        let order_index = Self::order_count_of(&who);
        <OrderCountOf<T>>::insert(&who, order_index + 1);

        let mut order = Self::new_fresh_order(
            pair_index,
            price,
            order_index,
            who,
            ordertype,
            direction,
            amount,
            remaining,
        );
        <OrderInfoOf<T>>::insert(&(order.submitter(), order.index()), &order);
        Self::event_order(&order);

        Self::match_order(&pair, &mut order, &handicap);

        Ok(())
    }

    fn match_order(pair: &TradingPair, order: &mut OrderDetails<T>, handicap: &HandicapT<T>) {
        #[cfg(feature = "std")]
        let begin = Local::now().timestamp_millis();

        Self::try_match_order(order, pair, handicap);

        #[cfg(feature = "std")]
        let end = Local::now().timestamp_millis();
        info!("do_match cost time:{:}", end - begin);

        Self::update_quotations_and_handicap(pair, order);
    }

    fn apply_cancel_order(who: &T::AccountId, pairid: TradingPairIndex, index: ID) -> Result {
        info!(
            "transactor: {:?}, pairid:{:}, index:{:}",
            who, pairid, index
        );

        let pair = Self::trading_pair(&pairid)?;
        let mut order =
            Self::order_info_of(&(who.clone(), index)).expect("We have ensured the order exists.");

        //更新状态
        order.status = if order.already_filled > Zero::zero() {
            OrderStatus::ParitialExecutedAndCanceled
        } else {
            OrderStatus::Canceled
        };
        order.last_update_at = <system::Module<T>>::block_number();

        //回退用户资产, 剩余的都退回
        let (back_token, back_amount) = match order.direction() {
            OrderDirection::Sell => (
                pair.currency_pair.base(),
                order
                    .amount()
                    .checked_sub(&order.already_filled)
                    .unwrap_or_default(),
            ),
            OrderDirection::Buy => (pair.currency_pair.counter(), As::sa(order.remaining.as_())),
        };

        //回退资产
        Self::cancel_order_unreserve(&who, &back_token, back_amount)?;

        order.remaining = order
            .remaining
            .checked_sub(&back_amount)
            .unwrap_or_default();

        //先更新 更新挂单中会删除
        <OrderInfoOf<T>>::insert((order.submitter(), order.index()), &order);

        //更新挂单
        Self::check_and_delete_quotations(order.pair(), order.price());

        //更新盘口
        Self::update_handicap(&pair, order.price(), order.direction());

        Ok(())
    }

    /// In order to get trading pair easier.
    fn trading_pair(pair_id: &TradingPairIndex) -> result::Result<TradingPair, &'static str> {
        match <TradingPairOf<T>>::get(pair_id) {
            Some(pair) => Ok(pair),
            None => Err("The order pair doesn't exist."),
        }
    }

    /// See if the price is valid. Return handicap if it's valid.
    fn is_valid_price(
        price: T::Price,
        direction: &OrderDirection,
        pairid: TradingPairIndex,
    ) -> result::Result<HandicapT<T>, &'static str> {
        let handicap = <HandicapOf<T>>::get(pairid).unwrap_or_default();
        let volatility = <PriceVolatility<T>>::get();

        match *direction {
            OrderDirection::Buy => {
                let sell = handicap.sell;
                let threshold = sell * As::sa(100_u32 + volatility) / As::sa(100_u32);
                // FIXME sell could be zero?
                if sell > Zero::zero() && price > threshold {
                    return Err("Price can't greater than PriceVolatility");
                }
            }
            OrderDirection::Sell => {
                let buy = handicap.buy;
                let threshold = buy * As::sa(100_u32 - volatility) / As::sa(100_u32);
                if buy > Zero::zero() && price < threshold {
                    return Err("price can't greater than PriceVolatility");
                }
            }
        }

        Ok(handicap)
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
                if pair.currency_pair.0.eq(token) || pair.currency_pair.1.eq(token) {
                    pair.online = false;
                    <TradingPairOf<T>>::insert(i, &pair);
                    Self::event_pair(&pair);
                }
            }
        }
        Ok(())
    }
}
