// Copyright 2019 Chainpool.
//! This module defines all the enum and structs.

use super::*;
use codec::{Decode, Encode};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_arithmetic::traits::BaseArithmetic;
use sp_runtime::RuntimeDebug;

/// Type for counting the number of user orders.
pub type OrderId = u64;

/// Type for counting the number of trading pairs.
pub type TradingPairId = u32;

/// Type for counting the number of executed orders given a trading pair.
pub type TradingHistoryIndex = u64;

/// A tick is a measure the minimum upward/downward movement in the price.
pub type Tick = u64;

/// Type of an order.
///
/// Currently only Limit Order is supported.
#[derive(PartialEq, Eq, Clone, Copy, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub enum OrderType {
    Limit,
    Market,
}

impl Default for OrderType {
    fn default() -> Self {
        Self::Limit
    }
}

/// Direction of an order.
#[derive(PartialEq, Eq, Clone, Copy, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub enum Side {
    Buy,
    Sell,
}

impl Default for Side {
    fn default() -> Self {
        Self::Buy
    }
}

/// Status of an order.
#[derive(PartialEq, Eq, Clone, Copy, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub enum OrderStatus {
    /// Order just got created.
    Created,
    /// Order has been filled partially.
    ParitialFill,
    /// Order has been filled completely.
    Filled,
    /// Order has been canceled with partial fill.
    ParitialFillAndCanceled,
    /// Order has been canceled without any deal.
    Canceled,
}

impl Default for OrderStatus {
    fn default() -> Self {
        Self::Created
    }
}

/// The best prices of a trading pair.
///
/// ------------------- Lowest Offer
///   ask(sell price)
/// -------------------
///   bid(buy price)
/// ------------------- Highest Bid
#[derive(PartialEq, Eq, Clone, Encode, Decode, Default, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct Handicap<Price> {
    pub highest_bid: Price,
    pub lowest_offer: Price,
}

impl<Price: Copy + BaseArithmetic> Handicap<Price> {
    pub fn new(highest_bid: Price, lowest_offer: Price) -> Self {
        Self {
            highest_bid,
            lowest_offer,
        }
    }

    /// Decreases the `highest_bid` by one tick.
    pub fn tick_down_highest_bid(&mut self, tick_precision: u32) -> Price {
        let tick = 10_u64.pow(tick_precision);

        self.highest_bid = self
            .highest_bid
            .checked_sub(&tick.saturated_into())
            .unwrap_or_else(Zero::zero);

        self.highest_bid
    }

    /// Increases the `lowest_offer` by one tick.
    pub fn tick_up_lowest_offer(&mut self, tick_precision: u32) -> Price {
        let tick = 10_u64.pow(tick_precision);

        self.lowest_offer = match self.lowest_offer.checked_add(&tick.saturated_into()) {
            Some(x) => x,
            None => panic!("Fail to tick up lowest_offer"),
        };

        self.lowest_offer
    }
}

/// A currency pair is the quotation of two different currencies,
/// with the value of one currency being quoted against the other.
///
/// PCX/BTC: PCX(base)/BTC(quote)
///
/// It indicates how much of the quote currency is needed to purchase
/// one unit of the base currency.
///
/// The first listed currency of a currency pair is called the `base` currency,
/// and the second currency is called the `quote` currency.
///
/// If you buy a currency pair, you buy the base currency and implicitly
/// sell the quoted currency. The bid (buy price) represents how much of
/// the quote currency you need to get one unit of the base currency.
///
/// Conversely, when you sell the currency pair, you sell the base currency
/// and receive the quote currency. The ask (sell price) for the currency pair
/// represents how much you will get in the quote currency for selling one unit of base currency.
#[derive(PartialEq, Eq, Clone, Default, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct CurrencyPair {
    /// The first currency of a currency pair.
    pub base: AssetId,
    /// The second currency of a currency pair.
    pub quote: AssetId,
}

impl CurrencyPair {
    pub fn new(base: AssetId, quote: AssetId) -> Self {
        Self { base, quote }
    }
}

/// Profile of a trading pair.
///
/// PCX/BTC = pip, a.k.a, percentage in point. Also called exchange rate.
/// tick precision for BTC
#[derive(PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct TradingPairProfile {
    /// The trading pair identifier.
    pub id: TradingPairId,
    /// The currency pair of trading pair.
    pub currency_pair: CurrencyPair,
    /// How many decimal places of the currency pair.
    pub pip_precision: u32,
    ///
    pub tick_precision: u32,
    /// Is the trading pair still tradable.
    pub online: bool,
}

impl TradingPairProfile {
    pub fn new(
        id: TradingPairId,
        currency_pair: CurrencyPair,
        pip_precision: u32,
        tick_precision: u32,
        online: bool,
    ) -> Self {
        Self {
            id,
            currency_pair,
            pip_precision,
            tick_precision,
            online,
        }
    }

    /// Returns the base currency of trading pair.
    pub fn base(&self) -> AssetId {
        self.currency_pair.base
    }

    /// Returns the quote currency of trading pair.
    pub fn quote(&self) -> AssetId {
        self.currency_pair.quote
    }

    /// Returns the tick size of trading pair.
    pub fn tick(&self) -> Tick {
        10_u64.pow(self.tick_precision)
    }

    /// The maximum ticks that the price can deviate from the handicap.
    pub fn fluctuation(&self) -> Tick {
        FLUCTUATION
            .saturated_into::<Tick>()
            .saturating_mul(self.tick())
    }

    /// Returns true if the price is divisible by tick.
    pub fn is_valid_price<Price: BaseArithmetic>(&self, price: Price) -> bool {
        (price.saturated_into::<u128>() % u128::from(self.tick())).is_zero()
    }
}

/// Immutable information of an order.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct OrderProperty<PairId, AccountId, Amount, Price, BlockNumber> {
    /// The order identifier.
    pub id: OrderId,
    /// The direction of order.
    pub side: Side,
    /// The price of order.
    pub price: Price,
    /// The amount of order, measured in the base currency.
    pub amount: Amount,
    /// The trading pair identifier.
    pub pair_id: PairId,
    /// The account that submitted the order.
    pub submitter: AccountId,
    /// The type of order.
    pub order_type: OrderType,
    /// Block number at which the order is created.
    pub created_at: BlockNumber,
}

/// Details of an order.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct Order<PairId, AccountId, Balance, Price, BlockNumber> {
    /// Immutable details of the order.
    pub props: OrderProperty<PairId, AccountId, Balance, Price, BlockNumber>,
    /// Status of the order.
    pub status: OrderStatus,
    /// The amount of unexecuted, measured by the **quote** currency.
    ///
    /// While (props.amount() - already_filled) can be expressed as
    /// the remaining amount as well, it's measured by the base currency.
    pub remaining: Balance,
    /// Indices of all executed transaction records.
    pub executed_indices: Vec<TradingHistoryIndex>,
    /// The amount of executed, measured by the **base** currency.
    pub already_filled: Balance,
    /// Block number at which the order details updated.
    pub last_update_at: BlockNumber,
}

impl<PairId, AccountId, Balance, Price, BlockNumber>
    Order<PairId, AccountId, Balance, Price, BlockNumber>
where
    PairId: Copy,
    AccountId: Clone,
    Balance: Copy + Ord + BaseArithmetic,
    Price: Copy,
    BlockNumber: Copy,
{
    pub fn new(
        props: OrderProperty<PairId, AccountId, Balance, Price, BlockNumber>,
        already_filled: Balance,
        last_update_at: BlockNumber,
        status: OrderStatus,
        executed_indices: Vec<TradingHistoryIndex>,
        remaining: Balance,
    ) -> Self {
        Self {
            props,
            already_filled,
            last_update_at,
            status,
            executed_indices,
            remaining,
        }
    }

    /// Returns the submitter of the order.
    pub fn submitter(&self) -> AccountId {
        self.props.submitter.clone()
    }

    /// Returns the pair ID of the order.
    pub fn pair_id(&self) -> PairId {
        self.props.pair_id
    }

    /// Returns the side of the order.
    pub fn side(&self) -> Side {
        self.props.side
    }

    /// Returns the amount of the order.
    pub fn amount(&self) -> Balance {
        self.props.amount
    }

    /// Returns the price of the order.
    pub fn price(&self) -> Price {
        self.props.price
    }

    /// Returns the id of the order.
    pub fn id(&self) -> OrderId {
        self.props.id
    }

    /// Returns the type of the order.
    pub fn order_type(&self) -> OrderType {
        self.props.order_type
    }

    /// Returns the block number of the order created.
    pub fn created_at(&self) -> BlockNumber {
        self.props.created_at
    }

    /// The `remaining` field is measured by the quote currency.
    /// (self.amount - self.already_filled) is the remaining in the base currency,
    pub fn remaining_in_base(&self) -> Balance {
        match self.amount().checked_sub(&self.already_filled) {
            Some(x) => x,
            None => panic!("Order.amount fail to sub already_filled"),
        }
    }

    /// Returns true if the order has been completely filled.
    pub fn is_fulfilled(&self) -> bool {
        self.already_filled >= self.amount()
    }

    /// Returns true if the `status` of order is `Canceled` or `ParitialFillAndCanceled`.
    pub fn is_canceled(&self) -> bool {
        self.status == OrderStatus::Canceled || self.status == OrderStatus::ParitialFillAndCanceled
    }

    fn _sub_remaining(&mut self, value: Balance) {
        self.remaining = match self.remaining.checked_sub(&value) {
            Some(x) => x,
            None => panic!("Fail to sub turnover when set remaining"),
        }
    }

    /// Minus the `remaining` of the order when it has been executed successfully.
    ///
    /// The turnover is measured in the quote currency.
    /// So (remaining - turnover) is what we need.
    pub fn decrease_remaining_on_execute(&mut self, turnover: Balance) {
        self._sub_remaining(turnover)
    }

    pub fn decrease_remaining_on_cancel(&mut self, refund: Balance) {
        self._sub_remaining(refund)
    }

    /// Updates the status of an order when it's being canceled.
    ///
    /// If the `already_filled` is not zero, then the status of order become
    /// `ParitialFillAndCanceled`, otherwise it's `Canceled`.
    pub fn update_status_on_cancel(&mut self) {
        self.status = if !self.already_filled.is_zero() {
            OrderStatus::ParitialFillAndCanceled
        } else {
            OrderStatus::Canceled
        };
    }
}

/// Latest price of a trading pair.
#[derive(PartialEq, Eq, Clone, Default, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct TradingPairInfo<Price, BlockNumber> {
    /// Price of Latest executed order.
    pub latest_price: Price,
    /// Block number at which point `TradingPairInfo` is updated.
    pub last_updated: BlockNumber,
}

/// The order of the quotation at some price.
#[derive(PartialEq, Eq, Clone, Default, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct Quotation<AccountId> {
    pub trader: AccountId,
    pub order_id: OrderId,
}
