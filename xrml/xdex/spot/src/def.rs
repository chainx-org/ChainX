use rstd::prelude::*;
use xassets::assetdef::Token;

pub type ID = u64;
pub type OrderPairID = u32;

#[derive(PartialEq, Eq, Clone, Copy, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub enum OrderType {
    Limit,  //限价单
    Market, //市价单
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
    FillNo,
    FillPart,
    FillAll,
    FillPartAndCancel,
    Cancel,
}

impl Default for OrderStatus {
    fn default() -> Self {
        OrderStatus::FillNo
    }
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct Handicap<Price>
where
    Price: Copy,
{
    pub buy: Price,
    pub sell: Price,
}

impl<Price> Handicap<Price>
where
    Price: Copy,
{
    pub fn new(buy: Price, sell: Price) -> Self {
        Handicap {
            buy: buy,
            sell: sell,
        }
    }
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct OrderPair {
    pub id: OrderPairID,
    pub first: Token,
    pub second: Token,
    pub precision: u32, //价格精度
    pub unit_precision:u32,//最小单位精度
    pub used: bool,
}
impl OrderPair {
    pub fn new(id: OrderPairID, first: Token, second: Token, precision: u32, unit: u32,status: bool) -> Self {
        OrderPair {
            id: id,
            first: first,
            second: second,
            precision: precision,
            unit_precision:unit,
            used: status,
        }
    }
}

/// 成交的历史，包含了双方的挂单编号
#[derive(PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct Fill<Pair, AccountId, Amount, Price>
where
    Pair: Clone,
    AccountId: Clone,
    Amount: Copy,
    Price: Copy,
{
    pub pair: Pair,
    pub price: Price,
    pub index: ID,
    pub maker_user: AccountId,
    pub taker_user: AccountId,
    pub maker_user_order_index: ID,
    pub taker_user_order_index: ID,
    pub amount: Amount,
    pub time: u64,
}

/// 用户的委托记录 包含了成交历史的index
#[derive(PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct Order<Pair, AccountId, Amount, Price, BlockNumber>
where
    Pair: Clone,
    AccountId: Clone,
    Amount: Copy,
    Price: Copy,
    BlockNumber: Copy,
{
    pub pair: Pair,
    pub price: Price,
    pub index: ID,

    pub user: AccountId,
    pub class: OrderType,
    pub direction: OrderDirection,

    pub amount: Amount,
    pub hasfill_amount: Amount,
    pub create_time: BlockNumber,
    pub lastupdate_time: BlockNumber,
    pub status: OrderStatus,
    pub reserve_last: Amount, //未被交易 未被回退
    pub fill_index: Vec<ID>,  // 填充历史记录的索引
}

impl<Pair, AccountId, Amount, Price, BlockNumber> Order<Pair, AccountId, Amount, Price, BlockNumber>
where
    Pair: Clone,
    AccountId: Clone,
    Amount: Copy,
    Price: Copy,
    BlockNumber: Copy,
{
    pub fn new(
        pair: Pair,
        index: ID,
        class: OrderType,
        direction: OrderDirection,
        user: AccountId,
        amount: Amount,
        hasfill_amount: Amount,
        price: Price,
        create_time: BlockNumber,
        lastupdate_time: BlockNumber,
        status: OrderStatus,
        fill_index: Vec<ID>,
        reserve_last: Amount,
    ) -> Self {
        Order {
            pair: pair,
            index: index,
            class: class,
            direction,
            user: user,
            amount: amount,
            hasfill_amount: hasfill_amount,
            price: price,
            create_time: create_time,
            lastupdate_time: lastupdate_time,
            status: status,
            fill_index: fill_index,
            reserve_last: reserve_last,
        }
    }
}
