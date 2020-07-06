// Copyright 2019 Chainpool.
//! This module defines all the enum and structs.

use super::*;
use codec::{Decode, Encode};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_arithmetic::traits::BaseArithmetic;
use sp_runtime::RuntimeDebug;

/// Index for the trading pair or users' order.
pub type OrderId = u64;
///
pub type TradeHistoryIndex = u64;
///
pub type TradingPairIndex = u32;

/// Currently only Limit Order is supported.
#[derive(PartialEq, Eq, Clone, Copy, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub enum OrderType {
    Limit,
    Market,
}

impl Default for OrderType {
    fn default() -> Self {
        OrderType::Limit
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
        Side::Buy
    }
}

/// Status of an order.
#[derive(PartialEq, Eq, Clone, Copy, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub enum OrderStatus {
    /// Order just got created, zero filled.
    Created,
    /// Order has been filled partially.
    ParitialFill,
    /// Order has been filled completely.
    Filled,
    /// Order has been canceled with partial fill.
    ParitialFillAndCanceled,
    /// Order has been canceled with zero fill.
    Canceled,
}

impl Default for OrderStatus {
    fn default() -> Self {
        OrderStatus::Created
    }
}

///     Seller
/// ----------------- Lowest Offer
///      ask
/// ----------------- MID
///      bid
/// ----------------- Highest Bid
///     Buyer
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

    /// Decreases the highest_bid by one tick.
    pub fn tick_down_highest_bid(&mut self, tick_precision: u32) -> Price {
        let tick = 10_u64.pow(tick_precision);

        self.highest_bid = self
            .highest_bid
            .checked_sub(&tick.saturated_into())
            .unwrap_or_else(Zero::zero);

        self.highest_bid
    }

    /// Increases the lowest_offer by one tick.
    pub fn tick_up_lowest_offer(&mut self, tick_precision: u32) -> Price {
        let tick = 10_u64.pow(tick_precision);

        self.lowest_offer = match self.lowest_offer.checked_add(&tick.saturated_into()) {
            Some(x) => x,
            None => panic!("Fail to tick up lowest_offer"),
        };

        self.lowest_offer
    }
}

/// PCX/BTC, base currency / quote currency
#[derive(PartialEq, Eq, Clone, Encode, Decode, Default, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct CurrencyPair {
    /// The former currency of pair, e.g., PCX for PCX/BTC.
    pub base: AssetId,
    /// The latter currency of pair, e.g., BTC for PCX/BTC.
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
    pub index: TradingPairIndex,
    pub currency_pair: CurrencyPair,
    pub pip_precision: u32,
    pub tick_precision: u32,
    pub online: bool,
}

impl TradingPairProfile {
    pub fn new(
        index: TradingPairIndex,
        currency_pair: CurrencyPair,
        pip_precision: u32,
        tick_precision: u32,
        online: bool,
    ) -> Self {
        Self {
            index,
            currency_pair,
            pip_precision,
            tick_precision,
            online,
        }
    }

    pub fn base(&self) -> AssetId {
        self.currency_pair.base
    }

    pub fn quote(&self) -> AssetId {
        self.currency_pair.quote
    }

    pub fn tick(&self) -> u64 {
        10_u64.pow(self.tick_precision)
    }

    pub fn fluctuation(&self) -> u64 {
        FLUCTUATION.saturated_into::<u64>() * self.tick()
    }
}

/// Immutable information of an order.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct OrderProperty<PairIndex, AccountId, Amount, Price, BlockNumber> {
    pub submitter: AccountId,
    pub pair_index: PairIndex,
    pub side: Side,
    pub amount: Amount,
    pub price: Price,
    pub index: OrderId,
    pub order_type: OrderType,
    pub created_at: BlockNumber,
}

/// PCX/BTC
/// The first one is called the base currency, the latter is called the quote currency.
///
/// Buy:  BTC -> PCX
/// Sell: PCX -> BTC
///
/// Notes:
///
/// The field `amount` and `already_filled` are measured according to the base currency.
/// the remaining field means the `remaining` part, which is measured by the quote currency.
///
/// While (props.amount() - already_filled) is also the remaining but measuredy by the base currency.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct Order<PairIndex, AccountId, Balance, Price, BlockNumber> {
    pub props: OrderProperty<PairIndex, AccountId, Balance, Price, BlockNumber>,

    pub status: OrderStatus,
    pub remaining: Balance,
    pub executed_indices: Vec<TradeHistoryIndex>, // indices of transaction record
    pub already_filled: Balance,
    pub last_update_at: BlockNumber,
}

impl<PairIndex, AccountId, Balance, Price, BlockNumber>
    Order<PairIndex, AccountId, Balance, Price, BlockNumber>
where
    PairIndex: Copy,
    AccountId: Clone,
    Balance: Copy + Ord + BaseArithmetic,
    Price: Copy,
    BlockNumber: Copy,
{
    pub fn new(
        props: OrderProperty<PairIndex, AccountId, Balance, Price, BlockNumber>,
        already_filled: Balance,
        last_update_at: BlockNumber,
        status: OrderStatus,
        executed_indices: Vec<TradeHistoryIndex>,
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

    // Wrapper for the innner OrderProperty.
    pub fn submitter(&self) -> AccountId {
        self.props.submitter.clone()
    }

    pub fn pair_index(&self) -> PairIndex {
        self.props.pair_index
    }

    pub fn side(&self) -> Side {
        self.props.side
    }

    pub fn amount(&self) -> Balance {
        self.props.amount
    }

    pub fn price(&self) -> Price {
        self.props.price
    }

    pub fn index(&self) -> OrderId {
        self.props.index
    }

    pub fn order_type(&self) -> OrderType {
        self.props.order_type
    }

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
    /// The turnover is measured in the quote currency.
    /// So (remaining - turnover) is what we need.
    pub fn decrease_remaining_on_execute(&mut self, turnover: Balance) {
        self._sub_remaining(turnover)
    }

    pub fn decrease_remaining_on_cancel(&mut self, refund: Balance) {
        self._sub_remaining(refund)
    }

    /// If the already_filled is not zero, then the status of order become
    /// ParitialExecutedAndCanceled, or else Canceled.
    pub fn update_status_on_cancel(&mut self) {
        self.status = if !self.already_filled.is_zero() {
            OrderStatus::ParitialFillAndCanceled
        } else {
            OrderStatus::Canceled
        };
    }
}

/// (latest price, average price, last last update height) of trading pair
#[derive(PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct TradingPairInfo<Price, BlockNumber> {
    pub latest_price: Price,
    pub last_updated: BlockNumber,
}
