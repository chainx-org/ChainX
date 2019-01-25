// Copyright 2018 Chainpool.

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

// for encode/decode
// Needed for deriving `Serialize` and `Deserialize` for various types.
// We only implement the serde traits for std builds - they're unneeded
// in the wasm runtime.
#[cfg(feature = "std")]
extern crate serde_derive;

extern crate log;

// Needed for deriving `Encode` and `Decode` for `RawEvent`.
#[macro_use]
extern crate parity_codec_derive;
extern crate parity_codec as codec;

// for substrate
// Needed for the set of mock primitives used in our tests.

// for substrate runtime
// map!, vec! marco.
//#[cfg_attr(feature = "std", macro_use)]
extern crate sr_io as runtime_io;
extern crate sr_primitives as primitives;
extern crate sr_std as rstd;
extern crate substrate_primitives;

#[macro_use]
extern crate srml_support as runtime_support;
extern crate srml_balances as balances;
extern crate srml_system as system;
extern crate srml_timestamp as timestamp;

#[cfg(test)]
extern crate srml_consensus as consensus;
extern crate xrml_bridge_bitcoin as xbitcoin;
extern crate xrml_xaccounts as xaccounts;
extern crate xrml_xassets_records as xrecords;

// for chainx runtime module lib
extern crate xrml_xassets_assets as xassets;
extern crate xrml_xsupport as xsupport;
extern crate xrml_xsystem as xsystem;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub mod def;

use codec::Codec;
use def::{
    Fill, Handicap, Order, OrderDirection, OrderPair, OrderPairID, OrderStatus, OrderType, ID,
};
use primitives::traits::{As, MaybeSerializeDebug, Member, SimpleArithmetic, Zero};
use primitives::traits::{CheckedAdd, CheckedSub};
use rstd::prelude::*;
use runtime_support::dispatch::Result;
use runtime_support::{Parameter, StorageMap, StorageValue};
use system::ensure_signed;

use xassets::assetdef::{ChainT,Token};

const PRICE_MAX_ORDER: usize = 1000;

pub type OrderT<T> = Order<
    OrderPairID,
    <T as system::Trait>::AccountId,
    <T as balances::Trait>::Balance,
    <T as Trait>::Price,
    <T as system::Trait>::BlockNumber,
>;
pub type FillT<T> = Fill<
    OrderPairID,
    <T as system::Trait>::AccountId,
    <T as balances::Trait>::Balance,
    <T as Trait>::Price,
>;
pub type HandicapT<T> = Handicap<<T as Trait>::Price>;

pub trait Trait: balances::Trait + xassets::Trait + timestamp::Trait {
    type Price: Parameter
        + Member
        + Codec
        + SimpleArithmetic
        + As<u8>
        + As<u16>
        + As<u32>
        + As<u64>
        + MaybeSerializeDebug
        + Copy
        + Zero
        + Default;
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event<T>() = default;

         //委托
        pub fn put_order(origin,pairid: OrderPairID,ordertype: OrderType,direction:OrderDirection,amount: T::Balance,price:T::Price) -> Result{
            runtime_io::print("[xdex spot] put_order");

            if price == Zero::zero() {
                return Err("price is zero");
            }
            let transactor = ensure_signed(origin)?;
            //从channel模块获得channel_name对应的account

            Self::do_put_order(&transactor, pairid, ordertype, direction,amount, price)
        }
        //取消委托
        pub fn cancel_order(origin,pairid:OrderPairID,index:ID) -> Result{
            runtime_io::print("[exchange xspot] cancel_order");
            return Self::do_cancel_order(origin,pairid,index);
        }


        //增加交易对
        pub fn add_pair(first:Token,second:Token,precision:u32,unit:u32, price:T::Price,used:bool)->Result{
             runtime_io::print("[xdex spot] add_pair");
             match Self::get_pair_by(&first, &second) {
                Some(_pair) => Err("have a existed pair in  list"),
                None => {
                    let pair_len=<OrderPairLen<T>>::get();

                    let pair=OrderPair{
                        id:pair_len,
                        first:first,
                        second:second,
                        precision:precision,
                        unit_precision:unit,
                        used:used,
                    };
                    <OrderPairOf<T>>::insert(pair.id,&pair);
                    <OrderPairPriceOf<T>>::insert(pair.id, (price, price, <system::Module<T>>::block_number()));

                    <OrderPairLen<T>>::put(pair_len+1);

                    Self::event_pair(&pair);
                    Ok(())
                },
            }
        }
        //更新交易对
        pub fn update_pair(id:OrderPairID,min:u32,used:bool)->Result{
            runtime_io::print("[xdex spot] update_pair");
            match <OrderPairOf<T>>::get(id) {
                None=> Err("not a existed pair in  list"),
                Some(mut pair) => {
                    if min < pair.unit_precision {
                        return Err("unit_precision error!");
                    }
                    pair.unit_precision=min;
                    pair.used=used;

                     <OrderPairOf<T>>::insert(id,&pair);
                    Self::event_pair(&pair);

                    Ok(())
                },
            }
        }
        pub fn update_price_volatility(price_volatility:u32)->Result{
            runtime_io::print("[xdex spot] update_price_volatility");
            if price_volatility >= 100 {
                return Err("price_volatility must be less 100!");
            }
            <PriceVolatility<T>>::put(price_volatility);

            Self::deposit_event(RawEvent::PriceVolatility(price_volatility));
            Ok(())
        }

    }
}

decl_event!(
    pub enum Event<T> where
        <T as system::Trait>::AccountId,
        <T as system::Trait>::BlockNumber,
        <T as balances::Trait>::Balance,
        <T as Trait>::Price
    {
        UpdateOrder(AccountId,ID,OrderPairID,Price,OrderType,OrderDirection,Balance,Balance,BlockNumber,BlockNumber,OrderStatus,Balance,Vec<ID>),
        FillOrder(ID,OrderPairID,Price,AccountId,AccountId,ID,ID, Balance,u64),
        UpdateOrderPair(OrderPairID,Token,Token,u32,u32,bool),
        PriceVolatility(u32),
    }
);

decl_storage! {
    trait Store for Module<T: Trait> as XSpot {

        //交易对列表
        pub OrderPairLen get(pair_len):  OrderPairID ;
        pub OrderPairOf get(pair_of):map ( OrderPairID ) => Option<OrderPair>;
        pub OrderPairPriceOf get(pair_price_of):map ( OrderPairID ) => Option<(T::Price,T::Price,T::BlockNumber)>;//最新价、成交平均价、更新高度
        //交易对的成交历史的最新编号 递增
        pub FillLen get(fill_len):  map (OrderPairID) => ID;

        //用户维度委托单的最新编号 递增
        pub AccountOrdersLen get(account_orders_len):map (T::AccountId) => Option<ID>;
        //用户+ID=>委托详情
        pub AccountOrder get(account_order): map(T::AccountId,ID) =>Option<OrderT<T>>;

        //报价；交易对+价格=> ( 用户，委托ID)
        pub Quotations get(quotations) : map (OrderPairID,T::Price) =>Option<Vec<(T::AccountId,ID)>>;
        //盘口：交易对=>(买一价、卖一价)
        pub HandicapMap get(handicap_map) :map(OrderPairID)=>Option<HandicapT<T>>;

        pub PriceVolatility get(price_volatility) config(): u32;//价格波动率%
    }
    add_extra_genesis {
        config(pair_list): Vec<(Token, Token, u32, u32, T::Price,bool)>;
        build(|storage: &mut primitives::StorageMap, _: &mut primitives::ChildrenStorageMap, config: &GenesisConfig<T>| {
                use runtime_io::with_externalities;
                use substrate_primitives::Blake2Hasher;
                let src_r = storage.clone().build_storage().unwrap().0;
                let mut tmp_storage: runtime_io::TestExternalities<Blake2Hasher> = src_r.into();
                with_externalities(&mut tmp_storage, || {

                    for (first, second, precision, min, price,status) in config.pair_list.iter() {

                        Module::<T>::add_pair(first.clone(),second.clone(),*precision,*min,*price,*status).unwrap();
                    }

                });
                let map: primitives::StorageMap = tmp_storage.into();
                storage.extend(map);
        });
    }

}

impl<T: Trait> Module<T> {
    pub fn get_pair_by(first: &Token, second: &Token) -> Option<OrderPair> {
        let pair_len = <OrderPairLen<T>>::get();
        for i in 0..pair_len {
            if let Some(pair) = <OrderPairOf<T>>::get(i) {
                if pair.first.eq(first) && pair.second.eq(second) {
                    return Some(pair.clone());
                }
            }
        }
        None
    }

    pub fn aver_asset_price(token:&Token)->Option<T::Balance> {
        /*
        如果交易对ID是XXX/PCX，则：
        返回：交易对Map[交易对ID].平均价 / 10^报价精度
        如果交易对ID是PCX/XXX，则：
        返回：10^交易对Map[交易对ID].报价精度 / 平均价  
        */
        let pair_len = <OrderPairLen<T>>::get();
        for i in 0..pair_len {
            if let Some(pair) = <OrderPairOf<T>>::get(i) {
                if pair.first.eq(token) && pair.second.eq(&<xassets::Module<T> as ChainT>::TOKEN.to_vec()) {
                    if let Some((_,aver,_))= <OrderPairPriceOf<T>>::get(i) {
                        let price:T::Balance=As::sa(aver.as_()/(10_u64.pow(pair.precision.as_() )));
                        return Some(price );
                    }
                }
                else if pair.first.eq(&<xassets::Module<T> as ChainT>::TOKEN.to_vec()) && pair.second.eq(token) {
                    if let Some((_,aver,_))= <OrderPairPriceOf<T>>::get(i) {
                        let price:T::Balance=As::sa(10_u64.pow(pair.precision.as_() )/aver.as_());
                        return Some(price) ;
                    }
                }
            }
        }

        None
    }

    fn do_cancel_order(origin: T::Origin, pairid: OrderPairID, index: ID) -> Result {
        let pair = match <OrderPairOf<T>>::get(pairid) {
            None => return Err("not a existed pair in  list"),
            Some(pair) => pair,
        };

        let transactor = ensure_signed(origin)?;

        if let Some(mut order) = Self::account_order((transactor.clone(), index)) {
            match order.status {
                OrderStatus::FillNo | OrderStatus::FillPart => {
                    //更新状态
                    order.status = if order.hasfill_amount > Zero::zero() {
                        OrderStatus::FillPartAndCancel
                    } else {
                        OrderStatus::Cancel
                    };
                    order.lastupdate_time = As::sa(<timestamp::Module<T>>::now().as_());

                    //回退用户资产
                    let back_token: &Token = match order.direction {
                        OrderDirection::Sell => &pair.first,
                        OrderDirection::Buy => &pair.second,
                    };

                    let back_amount: T::Balance = match order.direction {
                        OrderDirection::Sell => {
                            match order.amount.checked_sub(&order.hasfill_amount) {
                                Some(v) => v,
                                None => Default::default(),
                            }
                        } //As::sa(order.amount.as_() - order.hasfill_amount.as_()),
                        OrderDirection::Buy => As::sa(order.reserve_last.as_()), //剩余的都退回
                    };

                    //回退资产
                    Self::unreserve_token(&transactor, &back_token, back_amount)?;

                    order.reserve_last = match order.reserve_last.checked_sub(&back_amount) {
                        Some(v) => v,
                        None => Default::default(),
                    };

                    //先更新 更新挂单中会删除
                    <AccountOrder<T>>::insert((order.user.clone(), order.index), &order);

                    //更新挂单
                    Self::check_and_delete_quotations(order.pair, order.price);
                    //更新盘口
                    Self::update_handicap(
                        &pair,
                        order.price,
                        match order.direction {
                            OrderDirection::Sell => OrderDirection::Buy,
                            OrderDirection::Buy => OrderDirection::Sell,
                        },
                    );
                }
                _ => {
                    return Err(
                        "order status error( FiillAll|FillPartAndCancel|Cancel) cann't be cancel",
                    );
                }
            }
            Ok(())
        } else {
            Err("cann't find this index of order")
        }
    }

    fn do_put_order(
        who: &T::AccountId,
        pairid: OrderPairID,
        ordertype: OrderType,
        direction: OrderDirection,
        amount: T::Balance,
        price: T::Price,
    ) -> Result {
        /***********************常规检查**********************/
        //检查交易对
        let pair = match <OrderPairOf<T>>::get(pairid) {
            None => return Err("not a existed pair in  list"),
            Some(pair) => pair,
        };
        if OrderType::Limit != ordertype {
            return Err("not support order type!");
        }
        //检查 数量和价格
        if amount == Zero::zero() {
            return Err("amount cann't be zero");
        }
        let min_unit=10_u64.pow(pair.unit_precision);
        if price < As::sa(min_unit.as_()) {
            return Err("price cann't be less min_unit");
        }
        if price%As::sa(min_unit) != Zero::zero() {
            return Err("price % min_unit must be 0");
        }

        //检查不能超过最大 且方向相同
        match <Quotations<T>>::get((pair.id, price)) {
            Some(list) => {
                if list.len() >= PRICE_MAX_ORDER {
                    if let Some(order) = <AccountOrder<T>>::get(&list[0]) {
                        if order.direction == direction {
                            return Err("some price&direction too much order");
                        }
                    }
                }
            }
            None => {}
        }
        //盘口
        let handicap = match <HandicapMap<T>>::get(pairid) {
            Some(handicap) => handicap,
            None => Default::default(),
        };

        let mut reserve_last: T::Balance = Default::default();
        //检查价格范围
        match direction {
            OrderDirection::Buy => {
                if handicap.sell > Zero::zero()
                    && price
                        > ((handicap.sell
                            * As::sa(100_u32 + <PriceVolatility<T>>::get())) / As::sa(100_u32))
                {
                    return Err("price cann't > PriceVolatility");
                }
            }
            OrderDirection::Sell => {
                if handicap.buy > Zero::zero()
                    && price
                        < (handicap.buy * As::sa(100_u32 - <PriceVolatility<T>>::get()) / As::sa(100_u32))
                {
                    return Err("price cann't > PriceVolatility");
                }
            }
        }
        /***********************锁定资产**********************/
        if let Some(sum) = Self::trans_amount(amount, price, &pair) {
            match direction {
                OrderDirection::Buy => {
                    if <xassets::Module<T>>::free_balance(&who, &pair.second) < sum {
                        return Err("transactor's free token balance too low, can't put buy order");
                    }
                    reserve_last = As::sa(sum.as_());
                    //  锁定用户资产
                    Self::reserve_token(who, &pair.second, sum)?;
                }
                OrderDirection::Sell => {
                    if <xassets::Module<T>>::free_balance(&who, &pair.first) < As::sa(amount.as_())
                    {
                        return Err("transactor's free token balance too low, can't put sell order");
                    }
                    //  锁定用户资产
                    reserve_last = amount;
                    Self::reserve_token(who, &pair.first, As::sa(amount.as_()))?;
                }
            }
        } else {
            return Err("amount*price too small");
        }

        // 更新用户的交易对的挂单index
        let id: ID = match <AccountOrdersLen<T>>::get(who) {
            Some(id) => id,
            None => 0,
        };
        <AccountOrdersLen<T>>::insert(who, id + 1);

        //新增挂单记录
        let mut order = Order {
            pair: pairid,
            price: price,
            index: id,
            user: who.clone(),
            class: ordertype,
            direction: direction,
            amount: amount,
            hasfill_amount: Zero::zero(),
            create_time: As::sa(<timestamp::Module<T>>::now().as_()),
            lastupdate_time: As::sa(<timestamp::Module<T>>::now().as_()),
            status: OrderStatus::FillNo,
            reserve_last: reserve_last,
            fill_index: Default::default(),
        };
        // 更新用户挂单
        Self::event_order(&order);
        <AccountOrder<T>>::insert((order.user.clone(), order.index), &order);

        //撮合
        Self::do_match(&mut order, &pair, &handicap);

        /*********************** 更新报价 盘口**********************/
        Self::new_order(&mut order);

        //Event 记录状态变更

        Ok(())
    }
    fn do_match(order: &mut OrderT<T>, pair: &OrderPair, handicap: &HandicapT<T>) {
        let mut opponent_price: T::Price = match order.direction {
            OrderDirection::Buy => handicap.sell,
            OrderDirection::Sell => handicap.buy,
        };
        let min_unit=10_u64.pow(pair.unit_precision);

        loop {
            if opponent_price == Zero::zero() {
                break;
            }
            if order.hasfill_amount >= order.amount {
                order.status = OrderStatus::FillAll;
                break;
            }

            let mut find = false;
            match order.direction {
                OrderDirection::Buy => {
                    if order.price >= opponent_price {
                        find = true;
                    } else {
                        break;
                    }
                }
                OrderDirection::Sell => {
                    if order.price <= opponent_price {
                        find = true;
                    } else {
                        break;
                    }
                }
            };

            if find {
                match <Quotations<T>>::get((pair.id, opponent_price)) {
                    Some(list) => {
                        for i in 0..list.len() {
                            if order.hasfill_amount >= order.amount {
                                order.status = OrderStatus::FillAll;
                                break;
                            }
                            // 找到匹配的单
                            if let Some(mut maker_order) = <AccountOrder<T>>::get(&list[i]) {
                                let mut amount: T::Balance;

                                let v1: T::Balance =
                                    match order.amount.checked_sub(&order.hasfill_amount) {
                                        Some(v) => v,
                                        None => Default::default(),
                                    };
                                let v2: T::Balance = match maker_order
                                    .amount
                                    .checked_sub(&maker_order.hasfill_amount)
                                {
                                    Some(v) => v,
                                    None => Default::default(),
                                };

                                if v1 >= v2 {
                                    amount = v2;
                                } else {
                                    amount = v1;
                                }

                                //填充成交
                                if let Err(_msg) = Self::fill_order(
                                    pair.id,
                                    &mut maker_order,
                                    order,
                                    opponent_price,
                                    amount,
                                ) {
                                    // 记失败 event
                                }
                                //更新最新价、平均价
                                Self::update_last_average_price(pair.id, opponent_price);
                            }
                        }
                        //删除更新被完全撮合的单
                        Self::check_and_delete_quotations(pair.id, opponent_price);
                        //更新盘口
                        Self::update_handicap(&pair, opponent_price, order.direction);
                    }
                    None => {
                        //do nothing
                    }
                };
            }

            //移动对手价
            match order.direction {
                OrderDirection::Buy => {
                    opponent_price = match opponent_price
                        .checked_add(&As::sa(min_unit))
                    {
                        Some(v) => v,
                        None => Default::default(),
                    };
                }
                OrderDirection::Sell => {
                    opponent_price = match opponent_price
                        .checked_sub(&As::sa(min_unit))
                    {
                        Some(v) => v,
                        None => Default::default(),
                    };
                }
            }
        }
    }

    fn new_order(order: &mut OrderT<T>) {
        if order.amount > order.hasfill_amount {
            if order.hasfill_amount > Zero::zero() {
                order.status = OrderStatus::FillPart;
            }
            <AccountOrder<T>>::insert((order.user.clone(), order.index), &order.clone());

            //更新报价
            match <Quotations<T>>::get((order.pair, order.price)) {
                Some(mut list) => {
                    list.push((order.user.clone(), order.index));
                    <Quotations<T>>::insert((order.pair, order.price), list);
                }
                None => {
                    let mut list: Vec<(T::AccountId, ID)> = Vec::new();
                    list.push((order.user.clone(), order.index));
                    <Quotations<T>>::insert((order.pair, order.price), list);
                }
            };

            //更新盘口
            match order.direction {
                OrderDirection::Buy => {
                    if let Some(mut handicap) = <HandicapMap<T>>::get(order.pair) {
                        if order.price > handicap.buy || handicap.buy == Default::default() {
                            handicap.buy = order.price;
                            <HandicapMap<T>>::insert(order.pair, handicap);
                        }
                    } else {
                        let mut handicap: HandicapT<T> = Default::default();
                        handicap.buy = order.price;
                        <HandicapMap<T>>::insert(order.pair, handicap);
                    }
                }
                OrderDirection::Sell => {
                    if let Some(mut handicap) = <HandicapMap<T>>::get(order.pair) {
                        if order.price < handicap.sell || handicap.sell == Default::default() {
                            handicap.sell = order.price;
                            <HandicapMap<T>>::insert(order.pair, handicap);
                        }
                    } else {
                        let mut handicap: HandicapT<T> = Default::default();
                        handicap.sell = order.price;
                        <HandicapMap<T>>::insert(order.pair, handicap);
                    }
                }
            }
        } else {
            //更新状态 删除
            order.status = OrderStatus::FillAll;
            Self::event_order(&order);
            <AccountOrder<T>>::remove((order.user.clone(), order.index));
        }

        //Event 记录order状态通知链外
    }

    fn fill_order(
        pairid: OrderPairID,
        maker_order: &mut OrderT<T>,
        taker_order: &mut OrderT<T>,
        price: T::Price,
        amount: T::Balance,
    ) -> Result {
        let pair = match <OrderPairOf<T>>::get(pairid) {
            None => return Err("not a existed pair in  list"),
            Some(pair) => pair,
        };

        //更新挂单、成交历史、资产转移
        let new_fill_index = Self::fill_len(pairid) + 1;

        //更新maker对应的订单
        {
            maker_order.fill_index.push(new_fill_index);
            maker_order.hasfill_amount = match maker_order.hasfill_amount.checked_add(&amount) {
                Some(v) => v,
                None => Default::default(),
            };

            if maker_order.hasfill_amount == maker_order.amount {
                maker_order.status = OrderStatus::FillAll;
            } else if maker_order.hasfill_amount < maker_order.amount {
                maker_order.status = OrderStatus::FillPart;
            } else {
                return Err(" maker order has not enough amount");
            }

            maker_order.lastupdate_time = As::sa(<timestamp::Module<T>>::now().as_());
        }

        //更新taker对应的订单
        {
            taker_order.fill_index.push(new_fill_index);
            taker_order.hasfill_amount = match taker_order.hasfill_amount.checked_add(&amount) {
                Some(v) => v,
                None => Default::default(),
            };
            if taker_order.hasfill_amount == taker_order.amount {
                taker_order.status = OrderStatus::FillAll;
            } else if taker_order.hasfill_amount < taker_order.amount {
                taker_order.status = OrderStatus::FillPart;
            } else {
                return Err(" taker order has not enough amount");
            }

            taker_order.lastupdate_time = As::sa(<timestamp::Module<T>>::now().as_());
        }
        let maker_user = &maker_order.user;
        let taker_user = &taker_order.user;

        //转移 maker和taker中的资产
        match maker_order.direction {
            OrderDirection::Sell => {
                //卖家先解锁first token 并move给买家，
                let maker_back_token: &Token = &pair.first;
                let maker_back_amount: T::Balance = amount;
                maker_order.reserve_last =
                    match maker_order.reserve_last.checked_sub(&maker_back_amount) {
                        Some(v) => v,
                        None => Default::default(),
                    };

                Self::move_token(
                    &maker_back_token,
                    maker_back_amount,
                    &maker_user.clone(),
                    &taker_user.clone(),
                )?;

                //计算买家的数量，解锁second,并move 给卖家
                let taker_back_token: &Token = &pair.second;
                let taker_back_amount: T::Balance = match Self::trans_amount(amount, price, &pair) {
                    Some(sum) => sum,
                    None => Zero::zero(),
                };
                taker_order.reserve_last =
                    match taker_order.reserve_last.checked_sub(&taker_back_amount) {
                        Some(v) => v,
                        None => Default::default(),
                    };

                Self::move_token(
                    &taker_back_token,
                    taker_back_amount,
                    &taker_user.clone(),
                    &maker_user.clone(),
                )?
            }
            OrderDirection::Buy => {
                //买先解锁second token 并move给卖家，和手续费账户
                let maker_back_token: &Token = &pair.second;
                let maker_back_amount: T::Balance = match Self::trans_amount(amount, price, &pair) {
                    Some(sum) => sum,
                    None => Zero::zero(),
                };
                maker_order.reserve_last =
                    match maker_order.reserve_last.checked_sub(&maker_back_amount) {
                        Some(v) => v,
                        None => Default::default(),
                    };

                Self::move_token(
                    &maker_back_token,
                    maker_back_amount,
                    &maker_user.clone(),
                    &taker_user.clone(),
                )?;
                //计算卖家的数量，解锁second,并move 给买家,和手续费账户
                let taker_back_token: &Token = &pair.first;
                let taker_back_amount: T::Balance = As::sa(amount.as_());
                taker_order.reserve_last =
                    match taker_order.reserve_last.checked_sub(&taker_back_amount) {
                        Some(v) => v,
                        None => Default::default(),
                    };

                Self::move_token(
                    &taker_back_token,
                    taker_back_amount,
                    &taker_user.clone(),
                    &maker_user.clone(),
                )?
            }
        }

        //插入新的成交记录
        let fill = Fill {
            pair: pairid,
            price: price,
            index: new_fill_index,
            maker_user: maker_user.clone(),
            taker_user: taker_user.clone(),
            maker_user_order_index: maker_order.index,
            taker_user_order_index: taker_order.index,
            amount: amount,
            time: (<timestamp::Module<T>>::now().as_()),
        };

        <FillLen<T>>::insert(pairid, new_fill_index);

        //插入更新后的订单
        Self::event_order(&maker_order.clone());
        <AccountOrder<T>>::insert(
            (maker_order.user.clone(), maker_order.index),
            &maker_order.clone(),
        );

        Self::event_order(&taker_order.clone());
        <AccountOrder<T>>::insert(
            (taker_order.user.clone(), taker_order.index),
            &taker_order.clone(),
        );

        // 记录日志
        Self::deposit_event(RawEvent::FillOrder(
            fill.index,
            fill.pair,
            fill.price,
            fill.maker_user,
            fill.taker_user,
            fill.maker_user_order_index,
            fill.taker_user_order_index,
            fill.amount,
            As::sa(<timestamp::Module<T>>::now().as_()),
        ));

        Ok(())
    }

    fn unreserve_token(who: &T::AccountId, token: &Token, value: T::Balance) -> Result {
        <xassets::Module<T>>::move_balance(
            token,
            who,
            xassets::AssetType::ReservedDexSpot,
            who,
            xassets::AssetType::Free,
            value,
        )
        .map_err(|e| e.info())
    }

    fn reserve_token(who: &T::AccountId, token: &Token, value: T::Balance) -> Result {
        <xassets::Module<T>>::move_balance(
            token,
            who,
            xassets::AssetType::Free,
            who,
            xassets::AssetType::ReservedDexSpot,
            value,
        )
        .map_err(|e| e.info())
    }
    fn move_token(
        token: &Token,
        value: T::Balance,
        from: &T::AccountId,
        to: &T::AccountId,
    ) -> Result {
        <xassets::Module<T>>::move_balance(
            token,
            from,
            xassets::AssetType::ReservedDexSpot,
            to,
            xassets::AssetType::Free,
            value,
        )
        .map_err(|e| e.info())
    }

    //更新盘口
    fn update_handicap(pair: &OrderPair, price: T::Price, direction: OrderDirection) {
        //这里方向是反的，注意
        let min_unit=10_u64.pow(pair.unit_precision);

        match <Quotations<T>>::get((pair.id, price)) {
            Some(_list) => {}
            None => {
                match direction {
                    OrderDirection::Buy => {
                        //更新卖一
                        if let Some(mut handicap) = <HandicapMap<T>>::get(pair.id) {
                            handicap.sell = match handicap
                                .sell
                                .checked_add(&As::sa(min_unit))
                            {
                                Some(v) => v,
                                None => Default::default(),
                            };

                            <HandicapMap<T>>::insert(pair.id, handicap);
                        }
                    }
                    OrderDirection::Sell => {
                        //更新买一
                        if let Some(mut handicap) = <HandicapMap<T>>::get(pair.id) {
                            handicap.buy = match handicap
                                .buy
                                .checked_sub(&As::sa(min_unit))
                            {
                                Some(v) => v,
                                None => Default::default(),
                            };
                            <HandicapMap<T>>::insert(pair.id, handicap);
                        }
                    }
                };
            }
        };
    }
    fn blocks_per_day() -> u64 {
        let period = <timestamp::Module<T>>::block_period();
        let seconds = (24 * 60 * 60) as u64;
        seconds / period.as_()
    }
    fn update_last_average_price(pairid: OrderPairID, price: T::Price) {
        let blocks_per_day: u64 = Self::blocks_per_day();
        let number = <system::Module<T>>::block_number();

        match <OrderPairPriceOf<T>>::get(pairid) {
            Some((_last, mut aver, time)) => {
                if number - time < As::sa(blocks_per_day) {
                    let new_weight: u64 = price.as_() * (number - time).as_();
                    let old_weight: u64 = aver.as_() * (blocks_per_day + time.as_() - number.as_());
                    aver = As::sa((new_weight + old_weight) / blocks_per_day);
                } else {
                    aver = price;
                }
                <OrderPairPriceOf<T>>::insert(pairid, (price, aver, number));
            }
            None => {
                <OrderPairPriceOf<T>>::insert(pairid, (price, price, number));
            }
        }
    }
    //检查和更新报价
    fn check_and_delete_quotations(id: u32, price: T::Price) {
        match <Quotations<T>>::get((id, price)) {
            Some(list) => {
                let mut new_list: Vec<(T::AccountId, ID)> = Vec::new();
                for i in 0..list.len() {
                    if let Some(order) = <AccountOrder<T>>::get(&list[i]) {
                        if order.hasfill_amount >= order.amount
                            || OrderStatus::FillPartAndCancel == order.status
                            || OrderStatus::Cancel == order.status
                        {
                            //Event 记录挂单详情状态变更
                            Self::event_order(&order);
                            //删除挂单详情
                            <AccountOrder<T>>::remove(&list[i]);
                        } else {
                            new_list.push(list[i].clone());
                        }
                    }
                }
                //空了就删除
                if new_list.len() < 1 {
                    <Quotations<T>>::remove((id, price));
                } else {
                    <Quotations<T>>::insert((id, price), new_list)
                }
            }
            None => {}
        };
    }
    fn event_order(order: &OrderT<T>) {
        Self::deposit_event(RawEvent::UpdateOrder(
            order.user.clone(),
            order.index,
            order.pair,
            order.price,
            order.class,
            order.direction,
            order.amount,
            order.hasfill_amount,
            order.create_time,
            order.lastupdate_time,
            order.status,
            order.reserve_last,
            order.fill_index.clone(),
        ));
    }
    fn event_pair(pair: &OrderPair) {
        Self::deposit_event(RawEvent::UpdateOrderPair(
            pair.id,
            pair.first.clone(),
            pair.second.clone(),
            pair.precision,
            pair.unit_precision,
            pair.used,
        ));
    }
    fn trans_amount(
        amount: T::Balance, /*first的数量单位*/
        price: T::Price,    /*以second计价的价格*/
        pair: &OrderPair,
    ) -> Option<T::Balance> {
        // 公式=（amount*price*10^second精度 ）/（first精度*price精度）
        match <xassets::Module<T>>::asset_info(&pair.first) {
            Some((first, _, _)) => match <xassets::Module<T>>::asset_info(&pair.second) {
                Some((second, _, _)) => {
                    let trans_amount: T::Balance = As::sa(
                        (amount.as_() * price.as_() * (10_u64.pow(second.precision().as_())))
                            / (10_u64.pow(first.precision().as_())
                                * 10_u64.pow(pair.precision.as_())),
                    );
                    if trans_amount == Zero::zero() {
                        None
                    } else {
                        Some(trans_amount)
                    }
                }
                None => None,
            },
            None => None,
        }
    }
}
