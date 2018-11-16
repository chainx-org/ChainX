// Copyright 2018 Chainpool.

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

// for encode/decode
// Needed for deriving `Serialize` and `Deserialize` for various types.
// We only implement the serde traits for std builds - they're unneeded
// in the wasm runtime.
#[cfg(feature = "std")]
#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate log;

// Needed for deriving `Encode` and `Decode` for `RawEvent`.
#[macro_use]
extern crate parity_codec_derive;
extern crate parity_codec as codec;

// for substrate
// Needed for the set of mock primitives used in our tests.
#[cfg(test)]
extern crate substrate_primitives;

// for substrate runtime
// map!, vec! marco.
//#[cfg_attr(feature = "std", macro_use)]
extern crate sr_std as rstd;
// Needed for tests (`with_externalities`).
#[cfg(test)]
extern crate sr_io as runtime_io;
extern crate sr_primitives as runtime_primitives;

// for substrate runtime module lib
// Needed for type-safe access to storage DB.
#[macro_use]
extern crate srml_support as runtime_support;
extern crate srml_balances as balances;
extern crate srml_system as system;

// for chainx runtime module lib
#[cfg(test)]
extern crate cxrml_associations as associations;
extern crate cxrml_support as cxsupport;
#[cfg(test)]
extern crate cxrml_system as cxsystem;
extern crate cxrml_tokenbalances as tokenbalances;
//extern crate cxrml_exchange_matchorder as matchorder;

#[cfg(test)]
mod tests;

use codec::Codec;
use rstd::prelude::*;
use runtime_primitives::traits::OnFinalise;
use runtime_primitives::traits::{As, Member, SimpleArithmetic, Zero};
use runtime_support::dispatch::Result;
use runtime_support::{Parameter, StorageMap, StorageValue};
use system::ensure_signed;
use tokenbalances::{ReservedType, Symbol};

pub trait Trait: tokenbalances::Trait {
    type Amount: Parameter
        + Member
        + Codec
        + SimpleArithmetic
        + As<u8>
        + As<u16>
        + As<u32>
        + As<u64>
        + As<u128>
        + Copy
        + Zero
        + Default;
    type Price: Parameter
        + Member
        + Codec
        + SimpleArithmetic
        + As<u8>
        + As<u16>
        + As<u32>
        + As<u64>
        + As<u128>
        + Copy
        + Zero
        + Default;
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn add_pair(pair:OrderPair)->Result;
        fn set_order_fee(val: T::Balance) -> Result;
        /// pub call
        fn put_order(origin,pair: OrderPair,ordertype: OrderType,amount: T::Amount,price:T::Price) -> Result;
        fn cancel_order(origin,pair:OrderPair,index:u64) -> Result;

    }
}

decl_event!(
    pub enum Event<T> where
        <T as system::Trait>::AccountId,
        <T as system::Trait>::BlockNumber,
        <T as balances::Trait>::Balance,
        <T as Trait>::Amount,
        <T as Trait>::Price
    {
        ///  User Put Order 
        PutOrder(AccountId, OrderPair,u64, OrderType,Amount,Price, BlockNumber),
        ///  Fill Order
        FillOrder(OrderPair,u128,AccountId,AccountId,u64,u64,Price, Amount,Amount,Amount,BlockNumber),
        ///  User Cancel Order
        CancelOrder(AccountId, OrderPair,u64, BlockNumber),

        SetOrderFee(Balance),

        AddOrderPair(OrderPair),
    }
);

decl_storage! {
    trait Store for Module<T: Trait> as PendingOrders {
        pub OrderFee get(order_fee) config(): T::Balance;
        pub OrderPairList get(pair_list) config():  Vec<OrderPair> = [
            OrderPair {
                first: b"pcx".to_vec(),
                second: b"btc".to_vec(),
                precision: 8,
            }].to_vec();
        pub FillIndexOf get(fill_index_of):  map OrderPair => u128; //交易对的成交历史的index
        pub OrdersOf get(order_of):map (T::AccountId, OrderPair,u64) => Option<OrderT<T>>;
        pub LastOrderIndexOf get(last_order_index_of): map(T::AccountId,OrderPair)=>Option<u64>;
        pub FillsOf get(fill_of): map (OrderPair,u128) => Option<FillT<T>>;

        pub MaxCommandId get(max_command_id) config():u64; //每个块 最后重制为0
        pub CommandOf get(command_of) : map(u64) =>Option<(T::AccountId,OrderPair,u64,CommandType,u128)>; //存放当前块的所有挂单（需要撮合) matchorder 会从这里读取，然后清空
    }
}

impl<T: Trait> OnFinalise<T::BlockNumber> for Module<T> {
    fn on_finalise(_time: T::BlockNumber) {}
}

impl<T: Trait> Module<T> {
    /// Deposit one of this module's events.
    pub fn deposit_event(event: Event<T>) {
        <system::Module<T>>::deposit_event(<T as Trait>::Event::from(event).into());
    }
    // 增加交易对
    pub fn add_pair(pair: OrderPair) -> Result {
        if let Err(_) = Self::is_valid_pair(&pair) {
            info!("add_pair:{:?}", pair.clone());
            let mut pair_list: Vec<OrderPair> = <OrderPairList<T>>::get();
            pair_list.push(pair);
            <OrderPairList<T>>::put(pair_list);
        }

        Ok(())
    }
    //判定是否存在
    fn is_valid_pair(pair: &OrderPair) -> Result {
        let pair_list: Vec<OrderPair> = <OrderPairList<T>>::get();

        if pair_list.contains(pair) {
            Ok(())
        } else {
            Err("have a existed pair in orderpair list")
        }
    }
    pub fn set_order_fee(val: T::Balance) -> Result {
        info!("set_order_fee:{:?}", val);
        <OrderFee<T>>::put(val);
        Self::deposit_event(RawEvent::SetOrderFee(val));
        Ok(())
    }

    /// public call 挂单
    /// 注意 tokenbalance需要支持对pcx的挂单锁定
    pub fn put_order(
        origin: T::Origin,
        pair: OrderPair,
        ordertype: OrderType,
        amount: T::Amount,
        price: T::Price,
    ) -> Result {
        let transactor = ensure_signed(origin)?;
        info!(
            "put_order:{:?} {:?} {:?} {:?} {:?}  ",
            transactor.clone(),
            pair.clone(),
            ordertype,
            amount,
            price
        );
        //判断交易对是否存在
        if let Err(_) = Self::is_valid_pair(&pair) {
            error!(
                "put_order: not a existed pair in orderpair list {:?}",
                pair.clone()
            );
            return Err("not a existed pair in orderpair list");
        }
        //判定 数量和价格
        if amount == Zero::zero() {
            error!("put_order:  amount cann't be zero");
            return Err("amount cann't be zero");
        }
        if price == Zero::zero() {
            error!("put_order:price cann't be zero");
            return Err("price cann't be zero");
        }
        //手续费
        let fee = Self::order_fee();
        let sender = &transactor;
        <cxsupport::Module<T>>::handle_fee_after(sender, fee, true, || {
            //计算总额
            let sum: <T as tokenbalances::Trait>::TokenBalance = As::sa(amount.as_() * price.as_());
            match ordertype {
                OrderType::Buy => {
                    if <tokenbalances::Module<T>>::free_token(&(
                        sender.clone(),
                        pair.second.clone(),
                    )) < sum
                    {
                        error!("put_order: transactor's free token balance too low, can't put buy order");
                        return Err("transactor's free token balance too low, can't put buy order");
                    }
                    //  锁定用户资产
                    if let Err(msg) = <tokenbalances::Module<T>>::reserve(
                        sender,
                        &pair.second,
                        sum,
                        ReservedType::Exchange,
                    ) {
                        error!("put_order: buy tokenbalance reserve:{:?}", msg);
                        return Err(msg);
                    }
                }
                OrderType::Sell => {
                    if <tokenbalances::Module<T>>::free_token(&(sender.clone(), pair.first.clone()))
                        < As::sa(amount.as_())
                    {
                        error!("put_order: transactor's free token balance too low, can't put sell order");
                        return Err("transactor's free token balance too low, can't put sell order");
                    }
                    //  锁定用户资产
                    if let Err(msg) = <tokenbalances::Module<T>>::reserve(
                        sender,
                        &pair.first,
                        As::sa(amount.as_()),
                        ReservedType::Exchange,
                    ) {
                        error!("put_order: sell tokenbalance reserve:{:?}", msg);
                        return Err(msg);
                    }
                }
            }

            // 更新用户的交易对的挂单index
            let new_last_index =
                Self::last_order_index_of((sender.clone(), pair.clone())).unwrap_or_default() + 1;
            <LastOrderIndexOf<T>>::insert((sender.clone(), pair.clone()), new_last_index);
            //新增挂单记录
            let order = Order {
                pair: pair.clone(),
                index: new_last_index,
                class: ordertype,
                user: sender.clone(),
                amount,
                hasfill_amount: Zero::zero(),
                price,
                create_time: <system::Module<T>>::block_number(),
                lastupdate_time: <system::Module<T>>::block_number(),
                status: OrderStatus::FillNo,
                fill_index: Default::default(),
            };
            Self::insert_order(new_last_index, &order);
            info!(
                "put_order: insert new order {:?} {:?} {}",
                sender.clone(),
                pair.clone(),
                new_last_index
            );
            // 记录日志
            Self::deposit_event(RawEvent::PutOrder(
                sender.clone(),
                pair.clone(),
                new_last_index,
                ordertype,
                amount,
                price,
                <system::Module<T>>::block_number(),
            ));

            // 先缓存，ordermatch模块会清空
            let las_command_id = Self::max_command_id() + 1;
            <CommandOf<T>>::insert(
                las_command_id,
                (
                    order.user.clone(),
                    order.pair.clone(),
                    order.index,
                    CommandType::Match,
                    0,
                ),
            );
            <MaxCommandId<T>>::put(las_command_id);

            Ok(())
        })?;

        Ok(())
    }
    pub fn update_command_of(command_id: u64, bid: u128) {
        if let Some(mut command) = Self::command_of(command_id) {
            command.4 = bid;
            <CommandOf<T>>::insert(command_id, command);
        }
    }
    fn insert_order(index: u64, order: &OrderT<T>) {
        <OrdersOf<T>>::insert(
            (order.user.clone(), order.pair.clone(), index),
            order.clone(),
        );
    }
    pub fn cancel_order(origin: T::Origin, pair: OrderPair, index: u64) -> Result {
        let transactor = ensure_signed(origin)?;
        info!(
            "cancel_order:{:?} {:?} {:?} ",
            transactor.clone(),
            pair.clone(),
            index
        );

        if let Some(mut order) = Self::order_of((transactor.clone(), pair.clone(), index)) {
            match order.status {
                OrderStatus::FillNo | OrderStatus::FillPart => {
                    //更新状态
                    order.status = if order.hasfill_amount > Zero::zero() {
                        OrderStatus::FillPartAndCancel
                    } else {
                        OrderStatus::Cancel
                    };
                    order.lastupdate_time = <system::Module<T>>::block_number();
                    Self::insert_order(index, &order);
                    info!(
                        "cancel_order:{:?} {} {:?}",
                        transactor.clone(),
                        index,
                        order.status
                    );
                    //回退用户资产
                    let back_symbol: Symbol = match order.class {
                        OrderType::Sell => pair.clone().first,
                        OrderType::Buy => pair.clone().second,
                    };

                    let back_amount: <T as tokenbalances::Trait>::TokenBalance = match order.class {
                        OrderType::Sell => As::sa(order.amount.as_() - order.hasfill_amount.as_()),
                        OrderType::Buy => As::sa(
                            (order.amount.as_() - order.hasfill_amount.as_()) * order.price.as_(),
                        ),
                    };

                    if let Err(msg) = <tokenbalances::Module<T>>::unreserve(
                        &transactor.clone(),
                        &back_symbol,
                        back_amount,
                        ReservedType::Exchange,
                    ) {
                        error!("cancel_order:{:?}", msg);
                        return Err(msg);
                    }

                    //通知撮合，更新盘口
                    //先缓存，ordermatch模块会清空
                    let las_command_id = Self::max_command_id() + 1;
                    <CommandOf<T>>::insert(
                        las_command_id,
                        (
                            order.user.clone(),
                            order.pair.clone(),
                            order.index,
                            CommandType::Cancel,
                            0,
                        ),
                    );
                    <MaxCommandId<T>>::put(las_command_id);

                    //记录日志
                    Self::deposit_event(RawEvent::CancelOrder(
                        transactor.clone(),
                        pair.clone(),
                        index,
                        <system::Module<T>>::block_number(),
                    ));
                }
                _ => {
                    error!("cancel_order:order status error( FiillAll|FillPartAndCancel|Cancel) cann't be cancel");
                    return Err(
                        "order status error( FiillAll|FillPartAndCancel|Cancel) cann't be cancel",
                    );
                }
            }
            Ok(())
        } else {
            error!("cancel_order:cann't find this index of order");
            Err("cann't find this index of order")
        }
    }

    pub fn fill_order(
        pair: OrderPair,
        maker_user: T::AccountId,
        taker_user: T::AccountId,
        maker_user_order_index: u64,
        taker_user_order_index: u64,
        price: T::Price,
        amount: T::Amount,
        maker_fee: T::Amount,
        taker_fee: T::Amount,
    ) -> Result {
        info!(
            "fill_order:{:?} {:?} {:?} {:?} {:?} {:?} {:?} {:?} {:?}",
            pair.clone(),
            maker_user,
            taker_user,
            maker_user_order_index,
            taker_user_order_index,
            price,
            amount,
            maker_fee,
            taker_fee
        );

        //逻辑校验 在调用方撮合模块中实现，此处只维护挂单、成交历史、资产转移
        let new_last_fill_index = Self::last_fill_index_of_pair(&pair) + 1;

        //更新maker对应的订单
        let maker_order = if let Some(mut maker_order) =
            Self::order_of((maker_user.clone(), pair.clone(), maker_user_order_index))
        {
            maker_order.fill_index.push(new_last_fill_index);
            maker_order.hasfill_amount = maker_order.hasfill_amount + amount;
            if maker_order.hasfill_amount == maker_order.amount {
                maker_order.status = OrderStatus::FillAll;
            } else if maker_order.hasfill_amount < maker_order.amount {
                maker_order.status = OrderStatus::FillPart;
            } else {
                error!("fill_order: maker order has not enough amount");
                return Err(" maker order has not enough amount");
            }

            maker_order.lastupdate_time = <system::Module<T>>::block_number();
            maker_order
        } else {
            error!("fill_order: maker cann't find this maker order");
            return Err("cann't find this maker order");
        };

        //更新taker对应的订单
        let taker_order = if let Some(mut taker_order) =
            Self::order_of((taker_user.clone(), pair.clone(), taker_user_order_index))
        {
            taker_order.fill_index.push(new_last_fill_index);
            taker_order.hasfill_amount = taker_order.hasfill_amount + amount;
            if taker_order.hasfill_amount == taker_order.amount {
                taker_order.status = OrderStatus::FillAll;
            } else if taker_order.hasfill_amount < taker_order.amount {
                taker_order.status = OrderStatus::FillPart;
            } else {
                error!("fill_order: taker order has not enough amount");
                return Err(" taker order has not enough amount");
            }

            taker_order.lastupdate_time = <system::Module<T>>::block_number();
            taker_order
        } else {
            error!("fill_order: taker cann't find this maker order");
            return Err("cann't find this taker order");
        };

        //插入新的成交记录
        let fill = Fill {
            pair: pair.clone(),
            index: new_last_fill_index,
            maker_user: maker_user.clone(),
            taker_user: taker_user.clone(),
            maker_user_order_index: maker_order.index,
            taker_user_order_index: taker_order.index,
            price,
            amount,
            maker_fee,
            taker_fee,
            time: <system::Module<T>>::block_number(),
        };
        Self::insert_fill(&fill);
        <FillIndexOf<T>>::insert(&pair, new_last_fill_index);
        //插入更新后的maker对应的订单
        Self::insert_order(maker_order.index(), &maker_order);
        //插入更新后的taker对应的订单
        Self::insert_order(taker_order.index(), &taker_order);
        info!("fill_order:insert new fill {}", new_last_fill_index);
        //转移 maker和taker中的资产
        //move_free_token(from: &T::AccountId, to: &T::AccountId, symbol: &Symbol, value: T::TokenBalance)
        match maker_order.class {
            OrderType::Sell => {
                //卖家先解锁first token 并move给买家，和手续费账户
                let maker_back_symbol: Symbol = pair.clone().first;
                let maker_back_amount: <T as tokenbalances::Trait>::TokenBalance =
                    As::sa(amount.as_());
                if let Err(msg) = Self::move_token(
                    &maker_back_symbol,
                    maker_back_amount,
                    As::sa(maker_fee.as_()),
                    &maker_user.clone(),
                    &taker_user.clone(),
                    &maker_user.clone(),
                ) {
                    error!("fill_order: sell maker move_token {:?}", msg);
                    return Err(msg);
                }
                //计算买家的数量，解锁second,并move 给卖家,和手续费账户
                let taker_back_symbol: Symbol = pair.clone().second;
                let taker_back_amount: <T as tokenbalances::Trait>::TokenBalance =
                    As::sa(amount.as_() * price.as_());
                if let Err(msg) = Self::move_token(
                    &taker_back_symbol,
                    taker_back_amount,
                    As::sa(taker_fee.as_()),
                    &taker_user.clone(),
                    &maker_user.clone(),
                    &taker_user.clone(),
                ) {
                    error!("fill_order: sell taker move_token {:?}", msg);
                    return Err(msg);
                }
            }
            OrderType::Buy => {
                //买先解锁first token 并move给卖家，和手续费账户
                let maker_back_symbol: Symbol = pair.clone().second;
                let maker_back_amount: <T as tokenbalances::Trait>::TokenBalance =
                    As::sa(amount.as_() * price.as_());
                if let Err(msg) = Self::move_token(
                    &maker_back_symbol,
                    maker_back_amount,
                    As::sa(maker_fee.as_()),
                    &maker_user.clone(),
                    &taker_user.clone(),
                    &maker_user.clone(),
                ) {
                    error!("fill_order: sell maker move_token {:?}", msg);
                    return Err(msg);
                }
                //计算卖家的数量，解锁second,并move 给买家,和手续费账户
                let taker_back_symbol: Symbol = pair.clone().first;
                let taker_back_amount: <T as tokenbalances::Trait>::TokenBalance =
                    As::sa(amount.as_());
                if let Err(msg) = Self::move_token(
                    &taker_back_symbol,
                    taker_back_amount,
                    As::sa(taker_fee.as_()),
                    &taker_user.clone(),
                    &maker_user.clone(),
                    &taker_user.clone(),
                ) {
                    error!("fill_order: sell taker move_token {:?}", msg);
                    return Err(msg);
                }
            }
        }

        // 记录日志
        Self::deposit_event(RawEvent::FillOrder(
            fill.pair.clone(),
            fill.index,
            fill.maker_user,
            fill.taker_user,
            fill.maker_user_order_index,
            fill.taker_user_order_index,
            fill.price,
            fill.amount,
            fill.maker_fee,
            fill.taker_fee,
            <system::Module<T>>::block_number(),
        ));

        // 这里要处理撮合手续费 将其移动到固定账户中，并在下个块开始时，自动生成购买pcx的订单

        Ok(())
    }
    fn move_token(
        symbol: &Symbol,
        value: <T as tokenbalances::Trait>::TokenBalance,
        fee: <T as tokenbalances::Trait>::TokenBalance,
        from: &T::AccountId,
        to: &T::AccountId,
        _fee_account: &T::AccountId,
    ) -> Result {
        if let Err(msg) =
            <tokenbalances::Module<T>>::unreserve(from, symbol, value, ReservedType::Exchange)
        {
            return Err(msg);
        }
        //fee 外部算好了
        if let Err(e) = <tokenbalances::Module<T>>::move_free_token(
            &from.clone(),
            &to.clone(),
            &symbol.clone(),
            value - fee,
        ) {
            return Err(e.info());
        }

        Ok(())
    }

    fn insert_fill(fill: &FillT<T>) {
        <FillsOf<T>>::insert((fill.pair.clone(), fill.index), fill.clone());
    }
    pub fn clear_command() {
        <MaxCommandId<T>>::put(0);
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub enum OrderType {
    Buy,
    Sell,
}

impl Default for OrderType {
    fn default() -> Self {
        OrderType::Buy
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

#[derive(PartialEq, Eq, Clone, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub struct OrderPair {
    first: Symbol,
    second: Symbol,
    precision: u32, //价格精度
}

impl OrderPair {
    pub fn new(first: Symbol, second: Symbol, precision: u32) -> Self {
        return OrderPair {
            first,
            second,
            precision,
        };
    }
}

impl Default for OrderPair {
    fn default() -> Self {
        OrderPair {
            first: Default::default(),
            second: Default::default(),
            precision: 0,
        }
    }
}

/// 成交的历史，包含了双方的挂单index
#[derive(PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub struct Fill<Pair, AccountId, Amount, Price, BlockNumber>
where
    Pair: Clone,
    AccountId: Clone,
    Amount: Copy,
    Price: Copy,
    BlockNumber: Copy,
{
    pair: Pair,
    index: u128,
    maker_user: AccountId,
    taker_user: AccountId,
    maker_user_order_index: u64,
    taker_user_order_index: u64,
    price: Price,
    amount: Amount,
    maker_fee: Amount,
    taker_fee: Amount,
    time: BlockNumber,
}

impl<Pair, AccountId, Amount, Price, BlockNumber> Fill<Pair, AccountId, Amount, Price, BlockNumber>
where
    Pair: Clone,
    AccountId: Clone,
    Amount: Copy,
    Price: Copy,
    BlockNumber: Copy,
{
    pub fn pair(&self) -> Pair {
        self.pair.clone()
    }
    pub fn index(&self) -> u128 {
        self.index
    }
    pub fn maker_user(&self) -> AccountId {
        self.maker_user.clone()
    }
    pub fn taker_user(&self) -> AccountId {
        self.taker_user.clone()
    }
    pub fn maker_user_order_index(&self) -> u64 {
        self.maker_user_order_index
    }
    pub fn taker_user_order_index(&self) -> u64 {
        self.taker_user_order_index
    }
    pub fn price(&self) -> Price {
        self.price
    }

    pub fn amount(&self) -> Amount {
        self.amount
    }
    pub fn maker_fee(&self) -> Amount {
        self.maker_fee
    }
    pub fn taker_fee(&self) -> Amount {
        self.taker_fee
    }
    pub fn time(&self) -> BlockNumber {
        self.time
    }
}

pub type FillT<T> = Fill<
    OrderPair,
    <T as system::Trait>::AccountId,
    <T as Trait>::Amount,
    <T as Trait>::Price,
    <T as system::Trait>::BlockNumber,
>;

/// 用户的挂单记录 包含了成交历史的index
#[derive(PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub struct Order<Pair, AccountId, Amount, Price, BlockNumber>
where
    Pair: Clone,
    AccountId: Clone,
    Amount: Copy,
    Price: Copy,
    BlockNumber: Copy,
{
    pair: Pair,
    index: u64,
    class: OrderType,
    user: AccountId,
    amount: Amount,
    hasfill_amount: Amount,
    price: Price,
    create_time: BlockNumber,
    lastupdate_time: BlockNumber,
    status: OrderStatus,
    fill_index: Vec<u128>, // 填充历史记录的索引
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
        index: u64,
        class: OrderType,
        user: AccountId,
        amount: Amount,
        hasfill_amount: Amount,
        price: Price,
        create_time: BlockNumber,
        lastupdate_time: BlockNumber,
        status: OrderStatus,
        fill_index: Vec<u128>,
    ) -> Self {
        return Order {
            pair,
            index,
            class,
            user,
            amount,
            hasfill_amount,
            price,
            create_time,
            lastupdate_time,
            status,
            fill_index,
        };
    }
    pub fn pair(&self) -> Pair {
        self.pair.clone()
    }
    pub fn index(&self) -> u64 {
        self.index
    }
    pub fn class(&self) -> OrderType {
        self.class
    }
    pub fn user(&self) -> AccountId {
        self.user.clone()
    }
    pub fn amount(&self) -> Amount {
        self.amount
    }
    pub fn hasfill_amount(&self) -> Amount {
        self.hasfill_amount
    }
    pub fn price(&self) -> Price {
        self.price
    }
    pub fn create_time(&self) -> BlockNumber {
        self.create_time
    }
    pub fn lastupdate_time(&self) -> BlockNumber {
        self.lastupdate_time
    }
    pub fn status(&self) -> OrderStatus {
        self.status
    }
    pub fn fill_index(&self) -> Vec<u128> {
        self.fill_index.clone()
    }
}

pub type OrderT<T> = Order<
    OrderPair,
    <T as system::Trait>::AccountId,
    <T as Trait>::Amount,
    <T as Trait>::Price,
    <T as system::Trait>::BlockNumber,
>;

#[derive(PartialEq, Eq, Clone, Copy, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub enum CommandType {
    Match,
    Cancel,
}

impl Default for CommandType {
    fn default() -> Self {
        CommandType::Match
    }
}

impl<T: Trait> Module<T> {
    /// get the order list for a account
    pub fn order_list(who: &T::AccountId, pair: &OrderPair) -> Vec<OrderT<T>> {
        let mut records: Vec<OrderT<T>> = Vec::new();
        let last_index = Self::last_order_index_of((who.clone(), pair.clone())).unwrap_or_default();
        for i in (1..(last_index + 1)).rev() {
            if let Some(r) = <OrdersOf<T>>::get((who.clone(), pair.clone(), i)) {
                records.push(r);
            }
        }

        records
    }
}

const FILL_PAGE_SIZE: u128 = 1000;

impl<T: Trait> Module<T> {
    pub fn last_fill_index_of_pair(pair: &OrderPair) -> u128 {
        Self::fill_index_of(pair.clone())
    }

    /// 成交历史记录
    /// 每次只返回 1000条
    /// 必须加分page逻辑
    pub fn fill_list(pair: &OrderPair, start_: u128) -> Vec<FillT<T>> {
        let mut records: Vec<FillT<T>> = Vec::new();
        let last = Self::last_fill_index_of_pair(pair);

        let mut start = start_;
        if start == Zero::zero() {
            start = last;
        }
        if start > last {
            start = last;
        }
        let end = if start < FILL_PAGE_SIZE {
            0
        } else {
            start - FILL_PAGE_SIZE
        };

        for i in start..end {
            if let Some(r) = Self::fill_of((pair.clone(), i)) {
                records.push(r);
            }
        }

        records
    }
}
