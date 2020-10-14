// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

//! # Spot Module

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::type_complexity)]

mod execution;
mod rpc;
mod types;
mod weight_info;

#[cfg(any(feature = "runtime-benchmarks", test))]
mod benchmarking;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

use codec::Codec;

use sp_runtime::traits::{
    AtLeast32BitUnsigned, MaybeSerializeDeserialize, Member, SaturatedConversion, StaticLookup,
    Zero,
};
use sp_std::prelude::*;
use sp_std::{cmp, fmt::Debug, result};

use frame_support::{
    decl_error, decl_event, decl_module, decl_storage,
    dispatch::{DispatchError, DispatchResult},
    ensure,
    traits::{Currency, Get, ReservableCurrency},
    Parameter,
};
use frame_system::{ensure_root, ensure_signed};

use chainx_primitives::AssetId;
use xpallet_assets::AssetErr;
use xpallet_support::info;

pub use rpc::*;
pub use types::*;
pub use weight_info::WeightInfo;

/// Maximum of backlog orders.
const MAX_BACKLOG_ORDER: usize = 1000;

/// The maximum ticks that a price can deviated from the handicap.
///
/// NOTE:
/// In the veryinitial design, this limit is 10% of the handicap,
/// which resulted in the endless loop when matching the orders.
/// Now we use the fixed size of ticks to restrict the quote.
///
/// Currently we match the order by trying one tick at a time, if the
/// order prices have a large gap, the matching logic can take much
/// more time than the Block time to finish.
const DEFAULT_FLUCTUATION: u32 = 100;

pub type BalanceOf<T> = <<T as xpallet_assets::Trait>::Currency as Currency<
    <T as frame_system::Trait>::AccountId,
>>::Balance;

pub type OrderInfo<T> = Order<
    TradingPairId,
    <T as frame_system::Trait>::AccountId,
    BalanceOf<T>,
    <T as Trait>::Price,
    <T as frame_system::Trait>::BlockNumber,
>;

pub type HandicapInfo<T> = Handicap<<T as Trait>::Price>;

pub trait Trait: xpallet_assets::Trait {
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

    /// The price of an order.
    type Price: Parameter
        + Member
        + AtLeast32BitUnsigned
        + Codec
        + Default
        + Copy
        + MaybeSerializeDeserialize
        + Debug;

    type WeightInfo: WeightInfo;
}

decl_storage! {
    trait Store for Module<T: Trait> as XSpot {
        /// How many trading pairs so far.
        pub TradingPairCount get(fn trading_pair_count): TradingPairId;

        /// Record the exact balance of reserved native coins in Spot.
        pub NativeReserves get(fn native_reserves):
            map hasher(twox_64_concat) T::AccountId => BalanceOf<T>;

        /// The map from trading pair id to its static profile.
        pub TradingPairOf get(fn trading_pair_of):
            map hasher(twox_64_concat) TradingPairId => Option<TradingPairProfile>;

        /// (latest price, last update height) of trading pair
        pub TradingPairInfoOf get(fn trading_pair_info_of):
            map hasher(twox_64_concat) TradingPairId => Option<TradingPairInfo<T::Price, T::BlockNumber>>;

        /// Total transactions has been made for a trading pair.
        pub TradingHistoryIndexOf get(fn trading_history_index_of):
            map hasher(twox_64_concat) TradingPairId => TradingHistoryIndex;

        /// Total orders made by an account.
        pub OrderCountOf get(fn order_count_of):
            map hasher(twox_64_concat) T::AccountId => OrderId;

        /// Details of an user order given the account ID and order ID.
        pub OrderInfoOf get(fn order_info_of):
            double_map hasher(twox_64_concat) T::AccountId, hasher(twox_64_concat) OrderId
            => Option<OrderInfo<T>>;

        /// All the accounts and the order number given the trading pair ID and price.
        pub QuotationsOf get(fn quotations_of):
            double_map hasher(twox_64_concat) TradingPairId, hasher(twox_64_concat) T::Price
            => Vec<(T::AccountId, OrderId)>;

        /// TradingPairId => (highest_bid, lowest_ask)
        pub HandicapOf get(fn handicap_of):
            map hasher(twox_64_concat) TradingPairId => HandicapInfo<T>;

        /// The map of trading pair ID to the price fluctuation. Use with caution!
        pub PriceFluctuationOf get(fn price_fluctuation_of):
            map hasher(twox_64_concat) TradingPairId => PriceFluctuation = DEFAULT_FLUCTUATION;
    }

    add_extra_genesis {
        config(trading_pairs): Vec<(AssetId, AssetId, u32, u32, T::Price, bool)>;
        build(|config| {
            for (base, quote, pip_decimals, tick_decimals, price, tradable) in config.trading_pairs.iter() {
                Module::<T>::apply_add_trading_pair(
                    CurrencyPair::new(*base, *quote),
                    *pip_decimals,
                    *tick_decimals,
                    *price,
                    *tradable
                );
            }
        })
    }
}

decl_event!(
    pub enum Event<T>
    where
        Balance = BalanceOf<T>,
        <T as frame_system::Trait>::AccountId,
        <T as frame_system::Trait>::BlockNumber,
        <T as Trait>::Price,
    {
        /// A new order was created. [order_info]
        NewOrder(Order<TradingPairId, AccountId, Balance, Price, BlockNumber>),
        /// There was an update to the order due to it gets executed. [maker_order_info]
        MakerOrderUpdated(Order<TradingPairId, AccountId, Balance, Price, BlockNumber>),
        /// There was an update to the order due to it gets executed. [taker_order_info]
        TakerOrderUpdated(Order<TradingPairId, AccountId, Balance, Price, BlockNumber>),
        /// Overall information about the maker and taker orders when there was an order execution. [order_executed_info]
        OrderExecuted(OrderExecutedInfo<AccountId, Balance, BlockNumber, Price>),
        /// There is an update to the order due to it gets canceled. [order_info]
        CanceledOrderUpdated(Order<TradingPairId, AccountId, Balance, Price, BlockNumber>),
        /// A new trading pair is added. [pair_profile]
        TradingPairAdded(TradingPairProfile),
        /// Trading pair profile has been updated. [pair_profile]
        TradingPairUpdated(TradingPairProfile),
        /// Price fluctuation of trading pair has been updated. [pair_id, price_fluctuation]
        PriceFluctuationUpdated(TradingPairId, PriceFluctuation),
    }
);

decl_error! {
    /// Error for the spot module.
    pub enum Error for Module<T: Trait> {
        /// Price can not be zero, and must be an integer multiple of the tick decimals.
        InvalidPrice,
        /// The bid price can not higher than the PriceVolatility of current lowest ask.
        TooHighBidPrice,
        /// The ask price can not lower than the PriceVolatility of current highest bid.
        TooLowAskPrice,
        /// Failed to convert_base_to_quote since amount*price too small.
        VolumeTooSmall,
        /// Amount can not be zero.
        ZeroAmount,
        /// Can not put order if transactor's free token too low.
        InsufficientBalance,
        /// Invalid validator target.
        InvalidOrderType,
        /// The trading pair doesn't exist.
        InvalidTradingPair,
        /// The trading pair is untradable.
        TradingPairUntradable,
        /// The trading pair does not exist.
        NonexistentTradingPair,
        /// tick_decimals can not less than the one of pair.
        InvalidTickdecimals,
        /// Price volatility must be less 100.
        InvalidPriceVolatility,
        /// The trading pair already exists.
        TradingPairAlreadyExists,
        /// Too many orders for the same price.
        TooManyBacklogOrders,
        /// Can not retrieve the asset info given the trading pair.
        InvalidTradingPairAsset,
        /// Only the orders with ZeroFill or PartialFill can be canceled.
        CancelOrderNotAllowed,
        /// Can not find the order given the order index.
        InvalidOrderId,
        /// Error from assets module.
        AssetError,
    }
}

impl<T: Trait> From<AssetErr> for Error<T> {
    fn from(_: AssetErr) -> Self {
        Self::AssetError
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {

        type Error = Error<T>;

        fn deposit_event() = default;

        #[weight = <T as Trait>::WeightInfo::put_order()]
        pub fn put_order(
            origin,
            #[compact] pair_id: TradingPairId,
            order_type: OrderType,
            side: Side,
            #[compact] amount: BalanceOf<T>,
            #[compact] price: T::Price
        ) {
            let who = ensure_signed(origin)?;

            ensure!(!price.is_zero(), Error::<T>::InvalidPrice);
            ensure!(!amount.is_zero(), Error::<T>::ZeroAmount);
            ensure!(order_type == OrderType::Limit, Error::<T>::InvalidOrderType);

            let pair = Self::trading_pair(pair_id)?;

            ensure!(pair.tradable, Error::<T>::TradingPairUntradable);
            ensure!(pair.is_valid_price(price), Error::<T>::InvalidPrice);

            Self::is_valid_quote(price, side, pair_id)?;
            Self::has_too_many_backlog_orders(pair_id, price, side)?;

            // Reserve the token according to the order side.
            let (reserve_asset, reserve_amount) = match side {
                Side::Buy => (pair.quote(), Self::convert_base_to_quote(amount, price, &pair)?),
                Side::Sell => (pair.base(), amount)
            };
            Self::put_order_reserve(&who, reserve_asset, reserve_amount)?;
            Self::apply_put_order(who, pair_id, order_type, side, amount, price, reserve_amount)?;
        }

        #[weight = <T as Trait>::WeightInfo::cancel_order()]
        pub fn cancel_order(
            origin,
            #[compact] pair_id: TradingPairId,
            #[compact] order_id: OrderId
        ) {
            let who = ensure_signed(origin)?;
            Self::do_cancel_order(&who, pair_id, order_id)?;
        }

        /// Force cancel an order.
        #[weight = <T as Trait>::WeightInfo::force_cancel_order()]
        fn force_cancel_order(
            origin,
            who: <T::Lookup as StaticLookup>::Source,
            #[compact] pair_id: TradingPairId,
            #[compact] order_id: OrderId
        ) {
            ensure_root(origin)?;
            let who = T::Lookup::lookup(who)?;
            Self::do_cancel_order(&who, pair_id, order_id)?;
        }

        #[weight = <T as Trait>::WeightInfo::set_handicap()]
        fn set_handicap(origin, #[compact] pair_id: TradingPairId, new: Handicap< T::Price>) {
            ensure_root(origin)?;
            info!("[set_handicap]pair_id:{:?},new handicap:{:?}", pair_id, new);
            HandicapOf::<T>::insert(pair_id, new);
        }

        #[weight = <T as Trait>::WeightInfo::set_price_fluctuation()]
        fn set_price_fluctuation(
            origin,
            #[compact] pair_id: TradingPairId,
            #[compact] new: PriceFluctuation
        ) {
            ensure_root(origin)?;
            PriceFluctuationOf::insert(pair_id, new);
            Self::deposit_event(RawEvent::PriceFluctuationUpdated(pair_id, new));
        }

        /// Add a new trading pair.
        #[weight = <T as Trait>::WeightInfo::add_trading_pair()]
        pub fn add_trading_pair(
            origin,
            currency_pair: CurrencyPair,
            #[compact] pip_decimals: u32,
            #[compact] tick_decimals: u32,
            #[compact] latest_price: T::Price,
            tradable: bool,
        ) {
            ensure_root(origin)?;
            ensure!(
                Self::get_trading_pair_by_currency_pair(&currency_pair).is_none(),
                Error::<T>::TradingPairAlreadyExists
            );
            Self::apply_add_trading_pair(
                currency_pair,
                pip_decimals,
                tick_decimals,
                latest_price,
                tradable
            );
        }

        /// Update the trading pair profile.
        #[weight = <T as Trait>::WeightInfo::update_trading_pair()]
        pub fn update_trading_pair(
            origin,
            #[compact] pair_id: TradingPairId,
            #[compact] tick_decimals: u32,
            tradable: bool
        ) {
            ensure_root(origin)?;
            let pair = Self::trading_pair(pair_id)?;
            ensure!(tick_decimals >= pair.tick_decimals, Error::<T>::InvalidTickdecimals);
            Self::apply_update_trading_pair(pair_id, tick_decimals, tradable);
            Self::deposit_event(RawEvent::TradingPairUpdated(pair));
        }
    }
}

impl<T: Trait> Module<T> {
    /// Public mutables
    pub fn get_trading_pair_by_currency_pair(
        currency_pair: &CurrencyPair,
    ) -> Option<TradingPairProfile> {
        let pair_count = TradingPairCount::get();
        for i in 0..pair_count {
            if let Some(pair) = TradingPairOf::get(i) {
                if pair.base() == currency_pair.base && pair.quote() == currency_pair.quote {
                    return Some(pair);
                }
            }
        }
        None
    }

    #[inline]
    fn trading_pair(pair_id: TradingPairId) -> result::Result<TradingPairProfile, Error<T>> {
        TradingPairOf::get(pair_id).ok_or(Error::<T>::InvalidTradingPair)
    }

    fn get_order(who: &T::AccountId, order_id: OrderId) -> result::Result<OrderInfo<T>, Error<T>> {
        Self::order_info_of(who, order_id).ok_or(Error::<T>::InvalidOrderId)
    }

    /// Internal mutables
    fn apply_add_trading_pair(
        currency_pair: CurrencyPair,
        pip_decimals: u32,
        tick_decimals: u32,
        latest_price: T::Price,
        tradable: bool,
    ) {
        let pair_id = TradingPairCount::get();

        let pair = TradingPairProfile {
            id: pair_id,
            currency_pair,
            pip_decimals,
            tick_decimals,
            tradable,
        };

        info!("new trading pair: {:?}", pair);

        TradingPairOf::insert(pair_id, &pair);
        TradingPairInfoOf::<T>::insert(
            pair_id,
            TradingPairInfo {
                latest_price,
                last_updated: <frame_system::Module<T>>::block_number(),
            },
        );

        TradingPairCount::put(pair_id + 1);

        Self::deposit_event(RawEvent::TradingPairAdded(pair));
    }

    fn apply_update_trading_pair(pair_id: TradingPairId, tick_decimals: u32, tradable: bool) {
        info!(
            "[update_trading_pair]pair_id: {:}, tick_decimals: {:}, tradable:{:}",
            pair_id, tick_decimals, tradable
        );
        TradingPairOf::mutate(pair_id, |pair| {
            if let Some(pair) = pair {
                pair.tick_decimals = tick_decimals;
                pair.tradable = tradable;
            }
        });
    }

    fn apply_put_order(
        who: T::AccountId,
        pair_id: TradingPairId,
        order_type: OrderType,
        side: Side,
        amount: BalanceOf<T>,
        price: T::Price,
        reserve_amount: BalanceOf<T>,
    ) -> result::Result<(), Error<T>> {
        info!(
            "transactor:{:?}, pair_id:{:}, type:{:?}, side:{:?}, amount:{:?}, price:{:?}",
            who, pair_id, order_type, side, amount, price
        );

        let pair = Self::trading_pair(pair_id)?;

        let mut order = Self::inject_order(
            who,
            pair_id,
            price,
            order_type,
            side,
            amount,
            reserve_amount,
        );

        Self::try_match_order(&pair, &mut order, pair_id, side, price);

        Ok(())
    }

    fn do_cancel_order(
        who: &T::AccountId,
        pair_id: TradingPairId,
        order_id: OrderId,
    ) -> DispatchResult {
        let pair = Self::trading_pair(pair_id)?;
        ensure!(pair.tradable, Error::<T>::TradingPairUntradable);

        let order = Self::get_order(who, order_id)?;
        ensure!(
            order.status == OrderStatus::Created || order.status == OrderStatus::PartialFill,
            Error::<T>::CancelOrderNotAllowed
        );

        Self::apply_cancel_order(&who, pair_id, order_id)?;

        Ok(())
    }

    fn apply_cancel_order(
        who: &T::AccountId,
        pair_id: TradingPairId,
        order_id: OrderId,
    ) -> DispatchResult {
        info!(
            "[apply_cancel_order]who:{:?}, pair_id:{}, order_id:{}",
            who, pair_id, order_id
        );

        let pair = Self::trading_pair(pair_id)?;
        let mut order = Self::get_order(who, order_id)?;

        Self::update_order_and_unreserve_on_cancel(&mut order, &pair, who)?;

        Self::kill_order(
            pair_id,
            order.price(),
            who.clone(),
            order_id,
            pair,
            order.side(),
        );

        Ok(())
    }
}

impl<T: Trait> xpallet_assets_registrar::RegistrarHandler for Module<T> {
    fn on_deregister(token: &AssetId) -> DispatchResult {
        let pair_len = TradingPairCount::get();
        for i in 0..pair_len {
            if let Some(mut pair) = TradingPairOf::get(i) {
                if pair.base().eq(token) || pair.quote().eq(token) {
                    pair.tradable = false;
                    TradingPairOf::insert(i, &pair);
                    Self::deposit_event(RawEvent::TradingPairUpdated(pair));
                }
            }
        }
        Ok(())
    }
}
