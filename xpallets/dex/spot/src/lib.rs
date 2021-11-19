// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

//! # Spot Module

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::type_complexity)]

mod execution;
mod rpc;
mod types;
pub mod weights;

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
use sp_std::{cmp, fmt::Debug};

use frame_support::{
    dispatch::{DispatchError, DispatchResult},
    ensure,
    log::info,
    traits::{Currency, Get, ReservableCurrency},
    Parameter,
};
use frame_system::{ensure_root, ensure_signed};

use chainx_primitives::AssetId;
use xpallet_assets::AssetErr;

pub use self::rpc::*;
pub use self::types::*;
pub use self::weights::WeightInfo;

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

pub type BalanceOf<T> = <<T as xpallet_assets::Config>::Currency as Currency<
    <T as frame_system::Config>::AccountId,
>>::Balance;

pub type OrderInfo<T> = Order<
    TradingPairId,
    <T as frame_system::Config>::AccountId,
    BalanceOf<T>,
    <T as Config>::Price,
    <T as frame_system::Config>::BlockNumber,
>;

pub type HandicapInfo<T> = Handicap<<T as Config>::Price>;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    #[pallet::config]
    pub trait Config: frame_system::Config + xpallet_assets::Config {
        /// The overarching event type.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

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

    #[pallet::pallet]
    #[pallet::generate_store(pub(crate) trait Store)]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(<T as Config>::WeightInfo::put_order())]
        pub fn put_order(
            origin: OriginFor<T>,
            #[pallet::compact] pair_id: TradingPairId,
            order_type: OrderType,
            side: Side,
            #[pallet::compact] amount: BalanceOf<T>,
            #[pallet::compact] price: T::Price,
        ) -> DispatchResult {
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
                Side::Buy => (
                    pair.quote(),
                    Self::convert_base_to_quote(amount, price, &pair)?,
                ),
                Side::Sell => (pair.base(), amount),
            };
            Self::put_order_reserve(&who, reserve_asset, reserve_amount)?;
            Self::apply_put_order(
                who,
                pair_id,
                order_type,
                side,
                amount,
                price,
                reserve_amount,
            )?;
            Ok(())
        }

        #[pallet::weight(<T as Config>::WeightInfo::cancel_order())]
        pub fn cancel_order(
            origin: OriginFor<T>,
            #[pallet::compact] pair_id: TradingPairId,
            #[pallet::compact] order_id: OrderId,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            Self::do_cancel_order(&who, pair_id, order_id)?;
            Ok(())
        }

        /// Force cancel an order.
        #[pallet::weight(<T as Config>::WeightInfo::force_cancel_order())]
        pub fn force_cancel_order(
            origin: OriginFor<T>,
            who: <T::Lookup as StaticLookup>::Source,
            #[pallet::compact] pair_id: TradingPairId,
            #[pallet::compact] order_id: OrderId,
        ) -> DispatchResult {
            ensure_root(origin)?;
            let who = T::Lookup::lookup(who)?;
            Self::do_cancel_order(&who, pair_id, order_id)?;
            Ok(())
        }

        #[pallet::weight(<T as Config>::WeightInfo::set_handicap())]
        pub fn set_handicap(
            origin: OriginFor<T>,
            #[pallet::compact] pair_id: TradingPairId,
            new: Handicap<T::Price>,
        ) -> DispatchResult {
            ensure_root(origin)?;
            info!(target: "runtime::dex::spot", "[set_handicap] pair_id:{:?}, new handicap:{:?}", pair_id, new);
            HandicapOf::<T>::insert(pair_id, new);
            Ok(())
        }

        #[pallet::weight(<T as Config>::WeightInfo::set_price_fluctuation())]
        pub fn set_price_fluctuation(
            origin: OriginFor<T>,
            #[pallet::compact] pair_id: TradingPairId,
            #[pallet::compact] new: PriceFluctuation,
        ) -> DispatchResult {
            ensure_root(origin)?;
            PriceFluctuationOf::<T>::insert(pair_id, new);
            Self::deposit_event(Event::<T>::PriceFluctuationUpdated(pair_id, new));
            Ok(())
        }

        /// Add a new trading pair.
        #[pallet::weight(<T as Config>::WeightInfo::add_trading_pair())]
        pub fn add_trading_pair(
            origin: OriginFor<T>,
            currency_pair: CurrencyPair,
            #[pallet::compact] pip_decimals: u32,
            #[pallet::compact] tick_decimals: u32,
            #[pallet::compact] latest_price: T::Price,
            tradable: bool,
        ) -> DispatchResult {
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
                tradable,
            );
            Ok(())
        }

        /// Update the trading pair profile.
        #[pallet::weight(<T as Config>::WeightInfo::update_trading_pair())]
        pub fn update_trading_pair(
            origin: OriginFor<T>,
            #[pallet::compact] pair_id: TradingPairId,
            #[pallet::compact] tick_decimals: u32,
            tradable: bool,
        ) -> DispatchResult {
            ensure_root(origin)?;
            let pair = Self::trading_pair(pair_id)?;
            ensure!(
                tick_decimals >= pair.tick_decimals,
                Error::<T>::InvalidTickdecimals
            );
            Self::apply_update_trading_pair(pair_id, tick_decimals, tradable);
            Ok(())
        }
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(crate) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A new order was created. [order_info]
        NewOrder(Order<TradingPairId, T::AccountId, BalanceOf<T>, T::Price, T::BlockNumber>),
        /// There was an update to the order due to it gets executed. [maker_order_info]
        MakerOrderUpdated(
            Order<TradingPairId, T::AccountId, BalanceOf<T>, T::Price, T::BlockNumber>,
        ),
        /// There was an update to the order due to it gets executed. [taker_order_info]
        TakerOrderUpdated(
            Order<TradingPairId, T::AccountId, BalanceOf<T>, T::Price, T::BlockNumber>,
        ),
        /// Overall information about the maker and taker orders when there was an order execution. [order_executed_info]
        OrderExecuted(OrderExecutedInfo<T::AccountId, BalanceOf<T>, T::BlockNumber, T::Price>),
        /// There is an update to the order due to it gets canceled. [order_info]
        CanceledOrderUpdated(
            Order<TradingPairId, T::AccountId, BalanceOf<T>, T::Price, T::BlockNumber>,
        ),
        /// A new trading pair is added. [pair_profile]
        TradingPairAdded(TradingPairProfile),
        /// Trading pair profile has been updated. [pair_profile]
        TradingPairUpdated(TradingPairProfile),
        /// Price fluctuation of trading pair has been updated. [pair_id, price_fluctuation]
        PriceFluctuationUpdated(TradingPairId, PriceFluctuation),
    }

    /// Error for the spot module.
    #[pallet::error]
    pub enum Error<T> {
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

    /// How many trading pairs so far.
    #[pallet::storage]
    #[pallet::getter(fn trading_pair_count)]
    pub(crate) type TradingPairCount<T: Config> = StorageValue<_, TradingPairId, ValueQuery>;

    /// Record the exact balance of reserved native coins in Spot.
    #[pallet::storage]
    #[pallet::getter(fn native_reserves)]
    pub(crate) type NativeReserves<T: Config> =
        StorageMap<_, Twox64Concat, T::AccountId, BalanceOf<T>, ValueQuery>;

    /// The map from trading pair id to its static profile.
    #[pallet::storage]
    #[pallet::getter(fn trading_pair_of)]
    pub(crate) type TradingPairOf<T: Config> =
        StorageMap<_, Twox64Concat, TradingPairId, TradingPairProfile>;

    /// (latest price, last update height) of trading pair
    #[pallet::storage]
    #[pallet::getter(fn trading_pair_info_of)]
    pub(crate) type TradingPairInfoOf<T: Config> =
        StorageMap<_, Twox64Concat, TradingPairId, TradingPairInfo<T::Price, T::BlockNumber>>;

    /// Total transactions has been made for a trading pair.
    #[pallet::storage]
    #[pallet::getter(fn trading_history_index_of)]
    pub(crate) type TradingHistoryIndexOf<T: Config> =
        StorageMap<_, Twox64Concat, TradingPairId, TradingHistoryIndex, ValueQuery>;

    /// Total orders made by an account.
    #[pallet::storage]
    #[pallet::getter(fn order_count_of)]
    pub(crate) type OrderCountOf<T: Config> =
        StorageMap<_, Twox64Concat, T::AccountId, OrderId, ValueQuery>;

    /// Details of an user order given the account ID and order ID.
    #[pallet::storage]
    #[pallet::getter(fn order_info_of)]
    pub(crate) type OrderInfoOf<T: Config> =
        StorageDoubleMap<_, Twox64Concat, T::AccountId, Twox64Concat, OrderId, OrderInfo<T>>;

    /// All the accounts and the order number given the trading pair ID and price.
    #[pallet::storage]
    #[pallet::getter(fn quotations_of)]
    pub(crate) type QuotationsOf<T: Config> = StorageDoubleMap<
        _,
        Twox64Concat,
        TradingPairId,
        Twox64Concat,
        T::Price,
        Vec<(T::AccountId, OrderId)>,
        ValueQuery,
    >;

    /// TradingPairId => (highest_bid, lowest_ask)
    #[pallet::storage]
    #[pallet::getter(fn handicap_of)]
    pub(crate) type HandicapOf<T: Config> =
        StorageMap<_, Twox64Concat, TradingPairId, HandicapInfo<T>, ValueQuery>;

    #[pallet::type_value]
    pub fn DefaultForPriceFluctuationOf() -> PriceFluctuation {
        DEFAULT_FLUCTUATION
    }

    /// The map of trading pair ID to the price fluctuation. Use with caution!
    #[pallet::storage]
    #[pallet::getter(fn price_fluctuation_of)]
    pub(crate) type PriceFluctuationOf<T: Config> = StorageMap<
        _,
        Twox64Concat,
        TradingPairId,
        PriceFluctuation,
        ValueQuery,
        DefaultForPriceFluctuationOf,
    >;

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        pub trading_pairs: Vec<(AssetId, AssetId, u32, u32, T::Price, bool)>,
    }

    #[cfg(feature = "std")]
    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            Self {
                trading_pairs: Default::default(),
            }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
        fn build(&self) {
            let extra_genesis_builder: fn(&Self) = |config| {
                for (base, quote, pip_decimals, tick_decimals, price, tradable) in
                    config.trading_pairs.iter()
                {
                    Pallet::<T>::apply_add_trading_pair(
                        CurrencyPair::new(*base, *quote),
                        *pip_decimals,
                        *tick_decimals,
                        *price,
                        *tradable,
                    );
                }
            };
            extra_genesis_builder(self);
        }
    }
}

impl<T: Config> From<AssetErr> for Error<T> {
    fn from(_: AssetErr) -> Self {
        Self::AssetError
    }
}

impl<T: Config> Pallet<T> {
    /// Public mutables
    pub fn get_trading_pair_by_currency_pair(
        currency_pair: &CurrencyPair,
    ) -> Option<TradingPairProfile> {
        let pair_count = TradingPairCount::<T>::get();
        for i in 0..pair_count {
            if let Some(pair) = TradingPairOf::<T>::get(i) {
                if pair.base() == currency_pair.base && pair.quote() == currency_pair.quote {
                    return Some(pair);
                }
            }
        }
        None
    }

    #[inline]
    fn trading_pair(pair_id: TradingPairId) -> Result<TradingPairProfile, Error<T>> {
        TradingPairOf::<T>::get(pair_id).ok_or(Error::<T>::InvalidTradingPair)
    }

    fn get_order(who: &T::AccountId, order_id: OrderId) -> Result<OrderInfo<T>, Error<T>> {
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
        let pair_id = TradingPairCount::<T>::get();

        let pair = TradingPairProfile {
            id: pair_id,
            currency_pair,
            pip_decimals,
            tick_decimals,
            tradable,
        };

        info!(target: "runtime::dex::spot", "New trading pair: {:?}", pair);

        TradingPairOf::<T>::insert(pair_id, &pair);
        TradingPairInfoOf::<T>::insert(
            pair_id,
            TradingPairInfo {
                latest_price,
                last_updated: <frame_system::Pallet<T>>::block_number(),
            },
        );

        TradingPairCount::<T>::put(pair_id + 1);

        Self::deposit_event(Event::<T>::TradingPairAdded(pair));
    }

    fn apply_update_trading_pair(pair_id: TradingPairId, tick_decimals: u32, tradable: bool) {
        info!(
            target: "runtime::dex::spot",
            "[update_trading_pair] pair_id: {:}, tick_decimals: {:}, tradable:{:}",
            pair_id, tick_decimals, tradable
        );
        TradingPairOf::<T>::mutate(pair_id, |pair| {
            if let Some(pair) = pair {
                pair.tick_decimals = tick_decimals;
                pair.tradable = tradable;
                Self::deposit_event(Event::<T>::TradingPairUpdated(pair.clone()));
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
    ) -> Result<(), Error<T>> {
        info!(
            target: "runtime::dex::spot",
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
            target: "runtime::dex::spot",
            "[apply_cancel_order] who:{:?}, pair_id:{}, order_id:{}",
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

impl<T: Config> xpallet_assets_registrar::RegistrarHandler for Pallet<T> {
    fn on_deregister(token: &AssetId) -> DispatchResult {
        let pair_len = TradingPairCount::<T>::get();
        for i in 0..pair_len {
            if let Some(mut pair) = TradingPairOf::<T>::get(i) {
                if pair.base().eq(token) || pair.quote().eq(token) {
                    pair.tradable = false;
                    TradingPairOf::<T>::insert(i, &pair);
                    Self::deposit_event(Event::<T>::TradingPairUpdated(pair));
                }
            }
        }
        Ok(())
    }
}
