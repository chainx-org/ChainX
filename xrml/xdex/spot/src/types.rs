use parity_codec::{Decode, Encode};
use rstd::prelude::*;
#[cfg(feature = "std")]
use serde_derive::{Deserialize, Serialize};
use xassets::assetdef::Token;

pub type ID = u64;
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
pub enum OrderDirection {
    Buy,
    Sell,
}
impl Default for OrderDirection {
    fn default() -> Self {
        OrderDirection::Buy
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub enum OrderStatus {
    ZeroExecuted,
    ParitialExecuted,
    AllExecuted,
    ParitialExecutedAndCanceled,
    Canceled,
}

impl Default for OrderStatus {
    fn default() -> Self {
        OrderStatus::ZeroExecuted
    }
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct Handicap<Price: Copy> {
    pub buy: Price,
    pub sell: Price,
}

impl<Price: Copy> Handicap<Price> {
    pub fn new(buy: Price, sell: Price) -> Self {
        Handicap { buy, sell }
    }
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct CurrencyPair(pub Token, pub Token); // base currency / counter currency

impl CurrencyPair {
    pub fn base(&self) -> Token {
        self.0.clone()
    }

    pub fn base_as_ref(&self) -> &Token {
        &self.0
    }

    pub fn counter(&self) -> Token {
        self.1.clone()
    }

    pub fn counter_as_ref(&self) -> &Token {
        &self.1
    }
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct TradingPair {
    pub id: TradingPairIndex,
    pub currency_pair: CurrencyPair,
    pub precision: u32,      // price precision
    pub unit_precision: u32, // minimum unit precision
    pub online: bool,
}

impl TradingPair {
    pub fn new(
        id: TradingPairIndex,
        currency_pair: CurrencyPair,
        precision: u32,
        unit_precision: u32,
        online: bool,
    ) -> Self {
        TradingPair {
            id,
            currency_pair,
            precision,
            unit_precision,
            online,
        }
    }
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct Maker<AccountId, ID>(pub AccountId, pub ID); // (account, order_number) used in Fill

impl<AccountId: Clone, ID: Copy> Maker<AccountId, ID> {
    pub fn maker(&self) -> AccountId {
        self.0.clone()
    }

    pub fn order_index(&self) -> ID {
        self.1
    }
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct Taker<AccountId, ID>(pub AccountId, pub ID); // (account, order_number) used in Fill

impl<AccountId: Clone, ID: Copy> Taker<AccountId, ID> {
    pub fn taker(&self) -> AccountId {
        self.0.clone()
    }

    pub fn order_index(&self) -> ID {
        self.1
    }
}

/// Transaction record, including the order number of both buyer and seller.
#[derive(PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct Fill<Pair, AccountId, Amount, Price, BlockNumber> {
    pub pair: Pair,
    pub price: Price,
    pub index: ID,
    pub maker: Maker<AccountId, ID>,
    pub taker: Taker<AccountId, ID>,
    pub amount: Amount,
    pub time: BlockNumber,
}

impl<Pair: Clone, AccountId: Clone, Amount: Copy, Price: Copy, BlockNumber: Clone>
    Order<Pair, AccountId, Amount, Price, BlockNumber>
{
    pub fn submitter(&self) -> AccountId {
        self.props.0.clone()
    }

    pub fn pair(&self) -> Pair {
        self.props.1.clone()
    }

    pub fn direction(&self) -> OrderDirection {
        self.props.2
    }

    pub fn amount(&self) -> Amount {
        self.props.3
    }

    pub fn price(&self) -> Price {
        self.props.4
    }

    pub fn index(&self) -> ID {
        self.props.5
    }

    pub fn order_type(&self) -> OrderType {
        self.props.6
    }

    pub fn created_at(&self) -> BlockNumber {
        self.props.7.clone()
    }
}

/// We use property to express these immutable infomation of an order.
#[derive(PartialEq, Eq, Clone, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct OrderProperty<Pair, AccountId, Amount, Price, BlockNumber>(
    AccountId,
    Pair,
    OrderDirection,
    Amount,
    Price,
    ID,
    OrderType,
    BlockNumber,
);

impl<Pair: Clone, AccountId: Clone, Amount: Copy, Price: Copy, BlockNumber: Clone>
    OrderProperty<Pair, AccountId, Amount, Price, BlockNumber>
{
    pub fn new(
        pair: Pair,
        index: ID,
        class: OrderType,
        direction: OrderDirection,
        submitter: AccountId,
        amount: Amount,
        price: Price,
        created_at: BlockNumber,
    ) -> Self {
        OrderProperty(
            submitter, pair, direction, amount, price, index, class, created_at,
        )
    }

    pub fn submitter(&self) -> AccountId {
        self.0.clone()
    }

    pub fn pair(&self) -> Pair {
        self.1.clone()
    }

    pub fn direction(&self) -> OrderDirection {
        self.2
    }

    pub fn amount(&self) -> Amount {
        self.3
    }

    pub fn price(&self) -> Price {
        self.4
    }

    pub fn index(&self) -> ID {
        self.5
    }

    pub fn order_type(&self) -> OrderType {
        self.6
    }

    pub fn created_at(&self) -> BlockNumber {
        self.7.clone()
    }
}

#[derive(PartialEq, Eq, Clone, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct Order<Pair, AccountId, Amount, Price, BlockNumber> {
    pub props: OrderProperty<Pair, AccountId, Amount, Price, BlockNumber>,

    pub status: OrderStatus,
    pub remaining: Amount,   // remaining amount, measured by counter currency
    pub fill_index: Vec<ID>, // index of transaction record
    pub already_filled: Amount,
    pub last_update_at: BlockNumber, // FIXME BlockNumber or Timestamp?
}

impl<Pair, AccountId, Amount, Price, BlockNumber>
    Order<Pair, AccountId, Amount, Price, BlockNumber>
{
    pub fn new(
        props: OrderProperty<Pair, AccountId, Amount, Price, BlockNumber>,
        already_filled: Amount,
        last_update_at: BlockNumber,
        status: OrderStatus,
        fill_index: Vec<ID>,
        remaining: Amount,
    ) -> Self {
        Order {
            props,
            already_filled,
            last_update_at,
            status,
            fill_index,
            remaining,
        }
    }
}
