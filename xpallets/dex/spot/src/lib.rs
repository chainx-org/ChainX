//! # Staking Module

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

///
const MAX_BACKLOG_ORDER: usize = 1000;
/// TODO: doc this properly.
const FLUCTUATION: u32 = 100;

pub trait Trait: frame_system::Trait + xpallet_assets::Trait + pallet_timestamp::Trait {
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
    TradingPairIndex,
    <T as frame_system::Trait>::AccountId,
    <T as xpallet_assets::Trait>::Balance,
    <T as Trait>::Price,
    <T as frame_system::Trait>::BlockNumber,
>;

pub type HandicapInfo<T> = Handicap<<T as Trait>::Price>;

decl_storage! {
    trait Store for Module<T: Trait> as XSpot {
        /// How many trading pairs so far.
        pub TradingPairCount get(fn trading_pair_count): TradingPairIndex;

        /// The map from trading pair index to its static profile.
        pub TradingPairOf get(fn trading_pair_of):
            map hasher(twox_64_concat) TradingPairIndex => Option<TradingPairProfile>;

        /// (latest price, average price, last last update height) of trading pair
        pub TradingPairInfoOf get(fn trading_pair_info_of):
            map hasher(twox_64_concat) TradingPairIndex => Option<TradingPairInfo<T::Price, T::BlockNumber>>;

        /// Total transactions has been made for a trading pair.
        pub TradeHistoryIndexOf get(fn trade_history_index_of):
            map hasher(twox_64_concat) TradingPairIndex => TradeHistoryIndex;

        /// Total orders has made by an account.
        pub OrderCountOf get(fn order_count_of):
            map hasher(twox_64_concat) T::AccountId => OrderIndex;

        /// Details of the order given account and his order ID
        pub OrderInfoOf get(fn order_info_of):
            double_map hasher(twox_64_concat) T::AccountId, hasher(twox_64_concat) OrderIndex
            => Option<OrderInfo<T>>;

        /// All the account and his order number given a certain trading pair and price.
        pub QuotationsOf get(fn quotations_of):
            double_map hasher(twox_64_concat) TradingPairIndex, hasher(twox_64_concat) T::Price
            => Vec<(T::AccountId, OrderIndex)>;

        /// TradingPairIndex => (highest_bid, lowest_offer)
        pub HandicapOf get(fn handicap_of):
            map hasher(twox_64_concat) TradingPairIndex => HandicapInfo<T>;

        /// Price volatility
        pub PriceVolatility get(fn price_volatility) config(): u32;
    }
}

decl_event!(
    pub enum Event<T>
    where
        <T as frame_system::Trait>::AccountId,
        <T as xpallet_assets::Trait>::Balance,
    {
        /// The staker has been rewarded by this amount. `AccountId` is the stash account.
        PutOrder(AccountId, Balance),
        /// One validator (and its nominators) has been slashed by the given amount.
        Slash(AccountId, Balance),
        /// Nominator has bonded to the validator this amount.
        Bond(AccountId, AccountId, Balance),
        /// An account has unbonded this amount.
        Unbond(AccountId, AccountId, Balance),
        ///
        Claim(AccountId, AccountId, Balance),
        /// An account has called `withdraw_unbonded` and removed unbonding chunks worth `Balance`
        /// from the unlocking queue.
        WithdrawUnbonded(AccountId, Balance),
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
        /// Only the orders with ZeroFill or PartialFill can be canceled.
        CancelOrderNotAllowed,
        /// Can not find the order given the order index.
        InvalidOrderIndex,
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
            pair_index: TradingPairIndex,
            order_type: OrderType,
            side: Side,
            amount: T::Balance,
            price: T::Price
        ) {
            let who = ensure_signed(origin)?;

            ensure!(!price.is_zero(), Error::<T>::InvalidPrice);
            ensure!(!amount.is_zero(), Error::<T>::ZeroAmount);
            ensure!(order_type == OrderType::Limit, Error::<T>::InvalidOrderType);

            let pair = Self::trading_pair(pair_index)?;

            ensure!(pair.online, Error::<T>::TradingPairOffline);
            ensure!(
                (price.saturated_into() % u128::from(10_u64.pow(pair.tick_precision))).is_zero(),
                Error::<T>::InvalidPrice
            );

            Self::is_within_quotation_range(price, side, pair_index)?;
            Self::has_too_many_backlog_orders(pair_index, price, side)?;

            // Reserve the token according to the order side.
            let (reserve_token, reserve_amount) = match side {
                Side::Buy => {
                    (pair.quote(), Self::convert_base_to_quote(amount, price, &pair)?)
                }
                Side::Sell => {
                    (pair.base(), amount)
                }
            };

            Self::put_order_reserve(&who, &reserve_token, reserve_amount)?;

            Self::apply_put_order(who, pair_index, order_type, side, amount, price, reserve_amount)?;
        }

        #[weight = 10]
        pub fn cancel_order(origin, pair_index: TradingPairIndex, order_index: OrderIndex) {
            let who = ensure_signed(origin)?;

            Self::check_cancel_order(&who, pair_index, order_index)?;
            Self::apply_cancel_order(&who, pair_index, order_index)?;
        }

        #[weight = 10]
        fn set_cancel_order(origin, who: T::AccountId, pair_index: TradingPairIndex, order_index: OrderIndex) {
            ensure_root(origin)?;

            Self::check_cancel_order(&who, pair_index, order_index)?;
            Self::apply_cancel_order(&who, pair_index, order_index)?;
        }

        #[weight = 10]
        fn set_handicap(origin, pair_index: TradingPairIndex, highest_bid: T::Price, lowest_offer: T::Price) {
            ensure_root(origin)?;
            HandicapOf::<T>::insert(pair_index, HandicapInfo::<T>::new(highest_bid, lowest_offer));
            info!(
                    "[set_handicap] pair_index: {:?}, highest_bid: {:?}, lowest_offer: {:?}",
                    pair_index,
                    highest_bid,
                    lowest_offer,
                );
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

        let index = TradingPairCount::get();

        let pair = TradingPairProfile {
            index,
            currency_pair,
            pip_precision,
            tick_precision,
            online,
        };

        TradingPairOf::insert(index, &pair);
        TradingPairInfoOf::<T>::insert(
            index,
            TradingPairInfo {
                latest_price: price,
                last_updated: <frame_system::Module<T>>::block_number(),
            },
        );
        TradingPairCount::put(index + 1);

        Self::update_order_pair_event(&pair);

        Ok(())
    }

    pub fn update_trading_pair(
        pair_index: TradingPairIndex,
        tick_precision: u32,
        online: bool,
    ) -> Result<T> {
        info!(
            "[update_trading_pair] pair_index: {:}, tick_precision: {:}, online:{:}",
            pair_index, tick_precision, online
        );

        let pair = Self::trading_pair(pair_index)?;

        if tick_precision < pair.tick_precision {
            return Err(Error::<T>::InvalidTickPrecision);
        }

        TradingPairOf::mutate(pair_index, |pair| {
            if let Some(pair) = pair {
                pair.tick_precision = tick_precision;
                pair.online = online;
            }
        });

        Self::update_order_pair_event(&pair);

        Ok(())
    }

    pub fn get_trading_pair_by_currency_pair(
        currency_pair: &CurrencyPair,
    ) -> Option<TradingPairProfile> {
        let pair_count = TradingPairCount::get();
        for i in 0..pair_count {
            if let Some(pair) = TradingPairOf::get(i) {
                if pair.base() == currency_pair.base() && pair.quote() == currency_pair.quote() {
                    return Some(pair);
                }
            }
        }
        None
    }

    pub fn set_price_volatility(price_volatility: u32) -> Result<T> {
        info!(
            "[set_price_volatility] price_volatility: {:}",
            price_volatility
        );
        ensure!(
            price_volatility < FLUCTUATION,
            Error::<T>::InvalidPriceVolatility
        );
        PriceVolatility::put(price_volatility);
        // Self::deposit_event(RawEvent::PriceVolatility(price_volatility));
        Ok(())
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
    ) -> Result<T> {
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

    fn get_order(
        who: &T::AccountId,
        order_index: OrderIndex,
    ) -> result::Result<OrderInfo<T>, Error<T>> {
        Self::order_info_of(who, order_index).ok_or(Error::<T>::InvalidOrderIndex)
    }

    fn check_cancel_order(
        who: &T::AccountId,
        pair_index: TradingPairIndex,
        order_index: OrderIndex,
    ) -> Result<T> {
        let pair = Self::trading_pair(pair_index)?;
        ensure!(pair.online, Error::<T>::TradingPairOffline);

        let order = Self::get_order(who, order_index)?;

        ensure!(
            order.status == OrderStatus::Created || order.status == OrderStatus::ParitialFill,
            Error::<T>::CancelOrderNotAllowed
        );

        Ok(())
    }

    fn apply_cancel_order(
        who: &T::AccountId,
        pair_index: TradingPairIndex,
        order_index: OrderIndex,
    ) -> Result<T> {
        info!(
            "[cancel_order] transactor: {:?}, pair_index:{:}, order_index:{:}",
            who, pair_index, order_index
        );

        let pair = Self::trading_pair(pair_index)?;
        let mut order = Self::get_order(who, order_index)?;

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

    #[inline]
    fn trading_pair(pair_index: TradingPairIndex) -> result::Result<TradingPairProfile, Error<T>> {
        TradingPairOf::get(pair_index).ok_or(Error::<T>::InvalidOrderPair)
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
                    Self::update_order_pair_event(&pair);
                }
            }
        }
        Ok(())
    }
}
