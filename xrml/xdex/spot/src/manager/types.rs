// Copyright 2019 Chainpool.
//! This module defines all the enum and structs.

use parity_codec::{Decode, Encode};
#[cfg(feature = "std")]
use serde_derive::{Deserialize, Serialize};

// Substrate
use primitives::traits::{As, SimpleArithmetic, Zero};
use rstd::prelude::*;

// ChainX
use xassets::Token;

/// Index for the trading pair or users' order.
pub type OrderIndex = u64;
pub type TradeHistoryIndex = u64;
pub type TradingPairIndex = u32;

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

#[derive(PartialEq, Eq, Clone, Copy, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub enum OrderStatus {
    ZeroFill,
    ParitialFill,
    Filled,
    ParitialFillAndCanceled,
    Canceled,
}

impl Default for OrderStatus {
    fn default() -> Self {
        OrderStatus::ZeroFill
    }
}

///     Seller
/// ----------------- Lowest Offer
///      ask
/// ----------------- MID
///      bid
/// ----------------- Highest Bid
///     Buyer
#[derive(PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct Handicap<Price> {
    pub highest_bid: Price,
    pub lowest_offer: Price,
}

impl<Price: Copy + SimpleArithmetic + As<u64> + As<u32>> Handicap<Price> {
    pub fn new(highest_bid: Price, lowest_offer: Price) -> Self {
        Handicap {
            highest_bid,
            lowest_offer,
        }
    }

    /// Decrease the highest_bid by one tick.
    pub fn tick_down_highest_bid(&mut self, tick_precision: u32) -> Price {
        let tick = 10_u64.pow(tick_precision);

        self.highest_bid = self
            .highest_bid
            .checked_sub(&As::sa(tick))
            .unwrap_or(Zero::zero());

        self.highest_bid
    }

    /// Increase the lowest_offer by one tick.
    pub fn tick_up_lowest_offer(&mut self, tick_precision: u32) -> Price {
        let tick = 10_u64.pow(tick_precision);

        self.lowest_offer = match self.lowest_offer.checked_add(&As::sa(tick)) {
            Some(x) => x,
            None => panic!("Fail to tick up lowest_offer"),
        };

        self.lowest_offer
    }
}

/// PCX/BTC, base currency / quote currency
#[derive(PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct CurrencyPair(Token, Token);

impl CurrencyPair {
    pub fn new(base: Token, quote: Token) -> Self {
        CurrencyPair(base, quote)
    }
    pub fn base(&self) -> Token {
        self.0.clone()
    }

    pub fn base_as_ref(&self) -> &Token {
        &self.0
    }

    pub fn quote(&self) -> Token {
        self.1.clone()
    }

    pub fn quote_as_ref(&self) -> &Token {
        &self.1
    }
}

#[cfg(feature = "std")]
impl std::fmt::Debug for CurrencyPair {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "CurrencyPair: {}/{}",
            String::from_utf8_lossy(&self.0).into_owned(),
            String::from_utf8_lossy(&self.1).into_owned()
        )
    }
}

/// PCX/BTC = pip, a.k.a, percentage in point. Also called exchange rate.
/// tick precision for BTC
#[derive(PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct TradingPair {
    pub index: TradingPairIndex,
    pub currency_pair: CurrencyPair,
    pub pip_precision: u32,
    pub tick_precision: u32,
    pub online: bool,
}

impl TradingPair {
    pub fn new(
        index: TradingPairIndex,
        currency_pair: CurrencyPair,
        pip_precision: u32,
        tick_precision: u32,
        online: bool,
    ) -> Self {
        TradingPair {
            index,
            currency_pair,
            pip_precision,
            tick_precision,
            online,
        }
    }

    pub fn base(&self) -> Token {
        self.currency_pair.base()
    }

    pub fn base_as_ref(&self) -> &Token {
        self.currency_pair.base_as_ref()
    }

    pub fn quote(&self) -> Token {
        self.currency_pair.quote()
    }

    pub fn quote_as_ref(&self) -> &Token {
        self.currency_pair.quote_as_ref()
    }

    pub fn tick(&self) -> u64 {
        10_u64.pow(self.tick_precision)
    }

    pub fn fluctuation(&self) -> u64 {
        100 * self.tick()
    }
}

/// We use property to express these immutable information of an order.
#[derive(PartialEq, Eq, Clone, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct OrderProperty<PairIndex, AccountId, Amount, Price, BlockNumber>(
    AccountId,
    PairIndex,
    Side,
    Amount,
    Price,
    OrderIndex,
    OrderType,
    BlockNumber,
);

impl<PairIndex: Clone, AccountId: Clone, Amount: Copy, Price: Copy, BlockNumber: Clone>
    OrderProperty<PairIndex, AccountId, Amount, Price, BlockNumber>
{
    pub fn new(
        pair_index: PairIndex,
        index: OrderIndex,
        class: OrderType,
        side: Side,
        submitter: AccountId,
        amount: Amount,
        price: Price,
        created_at: BlockNumber,
    ) -> Self {
        OrderProperty(
            submitter, pair_index, side, amount, price, index, class, created_at,
        )
    }

    pub fn submitter(&self) -> AccountId {
        self.0.clone()
    }

    pub fn pair_index(&self) -> PairIndex {
        self.1.clone()
    }

    pub fn side(&self) -> Side {
        self.2
    }

    pub fn amount(&self) -> Amount {
        self.3
    }

    pub fn price(&self) -> Price {
        self.4
    }

    pub fn index(&self) -> OrderIndex {
        self.5
    }

    pub fn order_type(&self) -> OrderType {
        self.6
    }

    pub fn created_at(&self) -> BlockNumber {
        self.7.clone()
    }
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
#[derive(PartialEq, Eq, Clone, Encode, Decode)]
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

#[cfg(feature = "std")]
impl<
        PI: Clone + std::fmt::Debug,
        AI: Clone + std::fmt::Debug,
        A: Copy + Ord + SimpleArithmetic + std::fmt::Debug,
        P: Copy + std::fmt::Debug,
        B: Clone + std::fmt::Debug,
    > std::fmt::Debug for Order<PI, AI, A, P, B>
{
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "
            Order {{
                submitter: {:?},
                pair_index: {:?},
                side: {:?},
                amount: {:?},
                price: {:?},
                order_index: {:?},
                type: {:?},
                created_at: {:?},

                status: {:?},
                remaining: {:?},
                executed_indices: {:?}
                already_filled: {:?},
                last_update_at: {:?}
            }}",
            self.submitter(),
            self.pair_index(),
            self.side(),
            self.amount(),
            self.price(),
            self.index(),
            self.order_type(),
            self.created_at(),
            self.status,
            self.remaining,
            self.executed_indices,
            self.already_filled,
            self.last_update_at
        )
    }
}

impl<
        PairIndex: Clone,
        AccountId: Clone,
        Balance: Copy + Ord + SimpleArithmetic,
        Price: Copy,
        BlockNumber: Clone,
    > Order<PairIndex, AccountId, Balance, Price, BlockNumber>
{
    pub fn new(
        props: OrderProperty<PairIndex, AccountId, Balance, Price, BlockNumber>,
        already_filled: Balance,
        last_update_at: BlockNumber,
        status: OrderStatus,
        executed_indices: Vec<TradeHistoryIndex>,
        remaining: Balance,
    ) -> Self {
        Order {
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
        self.props.submitter()
    }

    pub fn pair_index(&self) -> PairIndex {
        self.props.pair_index()
    }

    pub fn side(&self) -> Side {
        self.props.side()
    }

    pub fn amount(&self) -> Balance {
        self.props.amount()
    }

    pub fn price(&self) -> Price {
        self.props.price()
    }

    pub fn index(&self) -> OrderIndex {
        self.props.index()
    }

    pub fn order_type(&self) -> OrderType {
        self.props.order_type()
    }

    pub fn created_at(&self) -> BlockNumber {
        self.props.created_at()
    }

    /// The `remaining` field is measured by the quote currency.
    /// (self.amount - self.already_filled) is the remaining in the base currency,
    pub fn remaining_in_base(&self) -> Balance {
        match self.amount().checked_sub(&self.already_filled) {
            Some(x) => x,
            None => panic!("Order.amount fail to sub already_filled"),
        }
    }

    /// If the order has been completely filled.
    pub fn is_fulfilled(&self) -> bool {
        self.already_filled >= self.amount()
    }

    /// If the `status` of order is Canceled or ParitialExecutedAndCanceled.
    pub fn is_canceled(&self) -> bool {
        self.status == OrderStatus::Canceled || self.status == OrderStatus::ParitialFillAndCanceled
    }

    fn sub_remaining(&mut self, value: Balance) {
        self.remaining = match self.remaining.checked_sub(&value) {
            Some(x) => x,
            None => panic!("Fail to sub turnover when set remaining"),
        }
    }

    /// Minus the `remaining` of the order when it has been executed successfully.
    /// The turnover is measured in the quote currency.
    /// So (remaining - turnover) is what we need.
    pub fn decrease_remaining_on_execute(&mut self, turnover: Balance) {
        self.sub_remaining(turnover)
    }

    pub fn decrease_remaining_on_cancel(&mut self, refund: Balance) {
        self.sub_remaining(refund)
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
