//! # Spot Module

#![cfg_attr(not(feature = "std"), no_std)]

mod execution;
mod types;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

use codec::Codec;

use sp_runtime::traits::{
    AtLeast32BitUnsigned, MaybeSerializeDeserialize, Member, SaturatedConversion, Zero,
};
use sp_std::prelude::*;
use sp_std::{cmp, fmt::Debug, result};

use frame_support::{
    decl_error, decl_event, decl_module, decl_storage, dispatch::DispatchResult, ensure, Parameter,
};
use frame_system::{self as system, ensure_root, ensure_signed};

use chainx_primitives::AssetId;
use xpallet_assets::AssetErr;
use xpallet_support::info;

use types::*;

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

pub trait Trait: frame_system::Trait + xpallet_assets::Trait {
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
}

type Result<T> = result::Result<(), Error<T>>;

pub type OrderInfo<T> = Order<
    TradingPairId,
    <T as frame_system::Trait>::AccountId,
    <T as xpallet_assets::Trait>::Balance,
    <T as Trait>::Price,
    <T as frame_system::Trait>::BlockNumber,
>;

pub type HandicapInfo<T> = Handicap<<T as Trait>::Price>;

decl_storage! {
    trait Store for Module<T: Trait> as XSpot {
        /// How many trading pairs so far.
        pub TradingPairCount get(fn trading_pair_count): TradingPairId;

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

        /// TradingPairId => (highest_bid, lowest_offer)
        pub HandicapOf get(fn handicap_of):
            map hasher(twox_64_concat) TradingPairId => HandicapInfo<T>;

        /// The map of trading pair ID to the price fluctuation. Use with caution!
        pub PriceFluctuationOf get(fn price_fluctuation_of):
            map hasher(twox_64_concat) TradingPairId => PriceFluctuation = DEFAULT_FLUCTUATION;
    }

    add_extra_genesis {
        config(trading_pairs): Vec<(AssetId, AssetId, u32, u32, T::Price, bool)>;
        build(|config| {
            for (base, quote, pip_precision, tick_precision, price, online) in config.trading_pairs.iter() {
                Module::<T>::add_trading_pair(
                    CurrencyPair::new(*base, *quote),
                    *pip_precision,
                    *tick_precision,
                    *price,
                    *online
                ).expect("genesis initialization can not fail");
            }
        })
    }
}

decl_event!(
    pub enum Event<T>
    where
        <T as frame_system::Trait>::AccountId,
        <T as frame_system::Trait>::BlockNumber,
        <T as xpallet_assets::Trait>::Balance,
        <T as Trait>::Price,
    {
        /// A new order is created.
        PutOrder(Order<TradingPairId, AccountId, Balance, Price, BlockNumber>),
        /// There is an update to the order due to it's canceled or get executed.
        UpdateOrder(Order<TradingPairId, AccountId, Balance, Price, BlockNumber>),
        /// The order gets executed.
        OrderExecuted(OrderExecutedInfo<AccountId, Balance, BlockNumber, Price>),
        /// Trading pair profile has been updated.
        TradingPairUpdated(TradingPairProfile),
        /// Price fluctuation of trading pair has been updated.
        PriceFluctuationUpdated(TradingPairId, PriceFluctuation),
    }
);

decl_error! {
    /// Error for the spot module.
    pub enum Error for Module<T: Trait> {
        /// Price can not be zero, and must be an integer multiple of the tick precision.
        InvalidPrice,
        /// The bid price can not higher than the PriceVolatility of current lowest_offer.
        TooHighBidPrice,
        /// The ask price can not lower than the PriceVolatility of current highest_bid.
        TooLowAskPrice,
        /// Failed to convert_base_to_quote since amount*price too small.
        VolumeTooSmall,
        /// Amount can not be zero.
        ZeroAmount,
        /// Can not put order if transactor's free token too low.
        InsufficientBalance,
        /// Invalid validator target.
        InvalidOrderType,
        /// The order pair doesn't exist.
        InvalidOrderPair,
        /// Can not force validator to be chilled.
        TradingPairOffline,
        /// The trading pair does not exist.
        NonexistentTradingPair,
        /// tick_precision can not less than the one of pair.
        InvalidTickPrecision,
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

        #[weight = 10]
        pub fn put_order(
            origin,
            pair_id: TradingPairId,
            order_type: OrderType,
            side: Side,
            amount: T::Balance,
            price: T::Price
        ) {
            let who = ensure_signed(origin)?;

            ensure!(!price.is_zero(), Error::<T>::InvalidPrice);
            ensure!(!amount.is_zero(), Error::<T>::ZeroAmount);
            ensure!(order_type == OrderType::Limit, Error::<T>::InvalidOrderType);

            let pair = Self::trading_pair(pair_id)?;

            ensure!(pair.online, Error::<T>::TradingPairOffline);
            ensure!(pair.is_valid_price(price), Error::<T>::InvalidPrice);

            Self::is_valid_quote(price, side, pair_id)?;
            Self::has_too_many_backlog_orders(pair_id, price, side)?;

            // Reserve the token according to the order side.
            let (reserve_asset, reserve_amount) = match side {
                Side::Buy => (pair.quote(), Self::convert_base_to_quote(amount, price, &pair)?),
                Side::Sell => (pair.base(), amount)
            };

            Self::put_order_reserve(&who, &reserve_asset, reserve_amount)?;

            Self::apply_put_order(who, pair_id, order_type, side, amount, price, reserve_amount)?;
        }

        #[weight = 10]
        pub fn cancel_order(origin, pair_id: TradingPairId, order_id: OrderId) {
            let who = ensure_signed(origin)?;
            Self::do_cancel_order(&who, pair_id, order_id)?;
        }

        #[weight = 10]
        fn set_cancel_order(origin, who: T::AccountId, pair_id: TradingPairId, order_id: OrderId) {
            ensure_root(origin)?;
            Self::do_cancel_order(&who, pair_id, order_id)?;
        }

        #[weight = 10]
        fn set_handicap(origin, pair_id: TradingPairId, highest_bid: T::Price, lowest_offer: T::Price) {
            ensure_root(origin)?;
            info!("[set_handicap]pair_id:{:?},highest_bid:{:?},lowest_offer:{:?}", pair_id, highest_bid, lowest_offer,);
            HandicapOf::<T>::insert(pair_id, HandicapInfo::<T>::new(highest_bid, lowest_offer));
        }

        #[weight = 10]
        fn set_price_fluctuation(origin, pair_id: TradingPairId, new: PriceFluctuation) {
            ensure_root(origin)?;
            PriceFluctuationOf::insert(pair_id, new);
            Self::deposit_event(RawEvent::PriceFluctuationUpdated(pair_id, new));
        }
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
    ) -> Result<T> {
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
            Error::<T>::TradingPairAlreadyExists
        );

        let pair_id = TradingPairCount::get();

        let pair = TradingPairProfile {
            id: pair_id,
            currency_pair,
            pip_precision,
            tick_precision,
            online,
        };

        TradingPairOf::insert(pair_id, &pair);
        TradingPairInfoOf::<T>::insert(
            pair_id,
            TradingPairInfo {
                latest_price: price,
                last_updated: <frame_system::Module<T>>::block_number(),
            },
        );
        TradingPairCount::put(pair_id + 1);

        Self::deposit_event(RawEvent::TradingPairUpdated(pair));

        Ok(())
    }

    pub fn update_trading_pair(
        pair_id: TradingPairId,
        tick_precision: u32,
        online: bool,
    ) -> Result<T> {
        info!(
            "[update_trading_pair] pair_id: {:}, tick_precision: {:}, online:{:}",
            pair_id, tick_precision, online
        );

        let pair = Self::trading_pair(pair_id)?;

        ensure!(
            tick_precision >= pair.tick_precision,
            Error::<T>::InvalidTickPrecision
        );

        TradingPairOf::mutate(pair_id, |pair| {
            if let Some(pair) = pair {
                pair.tick_precision = tick_precision;
                pair.online = online;
            }
        });

        Self::deposit_event(RawEvent::TradingPairUpdated(pair));

        Ok(())
    }

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

    /// Internal mutables
    fn apply_put_order(
        who: T::AccountId,
        pair_id: TradingPairId,
        order_type: OrderType,
        side: Side,
        amount: T::Balance,
        price: T::Price,
        reserve_amount: T::Balance,
    ) -> Result<T> {
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

    fn get_order(who: &T::AccountId, order_id: OrderId) -> result::Result<OrderInfo<T>, Error<T>> {
        Self::order_info_of(who, order_id).ok_or(Error::<T>::InvalidOrderId)
    }

    fn do_cancel_order(who: &T::AccountId, pair_id: TradingPairId, order_id: OrderId) -> Result<T> {
        let pair = Self::trading_pair(pair_id)?;
        ensure!(pair.online, Error::<T>::TradingPairOffline);

        let order = Self::get_order(who, order_id)?;
        ensure!(
            order.status == OrderStatus::Created || order.status == OrderStatus::ParitialFill,
            Error::<T>::CancelOrderNotAllowed
        );

        Self::apply_cancel_order(&who, pair_id, order_id)?;

        Ok(())
    }

    fn apply_cancel_order(
        who: &T::AccountId,
        pair_id: TradingPairId,
        order_id: OrderId,
    ) -> Result<T> {
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

    #[inline]
    fn trading_pair(pair_id: TradingPairId) -> result::Result<TradingPairProfile, Error<T>> {
        TradingPairOf::get(pair_id).ok_or(Error::<T>::InvalidOrderPair)
    }
}

impl<T: Trait> xpallet_assets::OnAssetRegisterOrRevoke for Module<T> {
    fn on_revoke(token: &AssetId) -> DispatchResult {
        let pair_len = TradingPairCount::get();
        for i in 0..pair_len {
            if let Some(mut pair) = TradingPairOf::get(i) {
                if pair.base().eq(token) || pair.quote().eq(token) {
                    pair.online = false;
                    TradingPairOf::insert(i, &pair);
                    Self::deposit_event(RawEvent::TradingPairUpdated(pair));
                }
            }
        }
        Ok(())
    }
}
