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

extern crate sr_io as runtime_io;
extern crate sr_primitives as runtime_primitives;

// for substrate runtime module lib
// Needed for type-safe access to storage DB.
#[macro_use]
extern crate srml_support as runtime_support;
extern crate srml_balances as balances;
extern crate srml_system as system;

// for chainx runtime module lib
extern crate xrml_xaccounts as xaccounts;
extern crate xrml_xassets_assets as assets;
extern crate xrml_xsupport as xsupport;
extern crate xrml_xsystem as xsystem;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

use assets::assetdef::{ChainT, Token};
use codec::Codec;
use rstd::prelude::*;
use rstd::result::Result as StdResult;
use runtime_primitives::traits::{As, Member, SimpleArithmetic, Zero};
use runtime_support::dispatch::Result;
use runtime_support::{Parameter, StorageMap, StorageValue};
use system::ensure_signed;

pub trait Trait: balances::Trait + xsystem::Trait + assets::Trait {
    type Amount: Parameter
        + Member
        + Codec
        + SimpleArithmetic
        + As<u8>
        + As<u16>
        + As<u32>
        + As<u64>
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
        + Copy
        + Zero
        + Default;
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event() = default;

        /// pub call
        pub fn put_order(origin,pair: OrderPair,ordertype: OrderType,amount: T::Amount,price:T::Price,channel:Channel) -> Result{
            runtime_io::print("[exchange pendingorders] put_order");

            if channel.len() > 32 {
                return Err("channel name too long");
            }
            if price == Zero::zero() {
                return Err("price is zero");
            }
            let transactor = ensure_signed(origin)?;
            //从channel模块获得channel_name对应的account

            Self::do_put_order(&transactor, &pair, ordertype, amount, price, &channel)
        }
        pub fn cancel_order(origin,pair:OrderPair,index:u64) -> Result{
            return Self::do_cancel_order(origin,pair,index);
        }

        pub fn add_pair(pair:OrderPair)->Result{
            if let Err(_) = Self::is_valid_pair(&pair) {
                let mut pair_list: Vec<OrderPair> = <OrderPairList<T>>::get();
                pair_list.push(pair);
                <OrderPairList<T>>::put(pair_list);
            }

            Ok(())
        }
        pub fn set_order_fee(val: T::Balance) -> Result{
            <OrderFee<T>>::put(val);
            Self::deposit_event(RawEvent::SetOrderFee(val));
            Ok(())
        }

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
        UpdateOrder(OrderPair,u64,OrderType,AccountId,Amount,Channel,Amount,Price,BlockNumber,BlockNumber,OrderStatus,Vec<u128>,Amount),

        ///  Fill Order
        FillOrder(OrderPair,u128,AccountId,AccountId,u64,u64,Price, Amount,Amount,Token,Amount,Token,BlockNumber),
        ///  User Cancel Order
        CancelOrder(AccountId, OrderPair,u64, BlockNumber),

        SetOrderFee(Balance),
        SetAveragePriceLen(Amount),
        AddOrderPair(OrderPair),

        FeeBuy(Token,Balance,AccountId),
    }
);

decl_storage! {
    trait Store for Module<T: Trait> as PendingOrders {
        pub OrderFee get(order_fee) config(): T::Balance;
        pub OrderPairList get(pair_list):  Vec<OrderPair> ;
        pub OrderPairDetailMap get(pair_detail_of):map  OrderPair => Option<OrderPairDetail>;

        pub FillIndexOf get(fill_index_of):  map OrderPair => u128; //交易对的成交历史的index
        pub OrdersOf get(order_of):map (T::AccountId, OrderPair,u64) => Option<OrderT<T>>;
        pub LastOrderIndexOf get(last_order_index_of): map(T::AccountId,OrderPair)=>Option<u64>;

        pub MaxCommandId get(max_command_id) config():u64; //每个块 最后重制为0
        pub CommandOf get(command_of) : map u64 =>Option<(T::AccountId,OrderPair,u64,CommandType,u128)>; //存放当前块的所有挂单（需要撮合) xmatchorder 会从这里读取，然后清空

        pub AveragePriceLen get(average_price_len) config(): T::Amount;
        pub LastAveragePrice get(last_average_price) : map OrderPair  => Option<T::Price>;  //
        pub LastPrice get(last_price) : map OrderPair  => Option<T::Price>;

        pub FeeBuyOrder get(fee_buy_order) : map u64 =>Option<(OrderPair,T::Amount,T::Price,T::AccountId)>; //存储块尾的买单
        pub FeeBuyOrderMax get(fee_buy_order_max) : u64;
    }
    add_extra_genesis {
        config(pair_list): Vec<(OrderPair, u32)>;
        build(|storage: &mut runtime_primitives::StorageMap, _: &mut runtime_primitives::ChildrenStorageMap, config: &GenesisConfig<T>| {
                use codec::Encode;

                let mut p_list: Vec<OrderPair> = Vec::new();
                for (pair,precision) in config.pair_list.iter() {
                    let detail = OrderPairDetail{ precision: *precision };
                    storage.insert(GenesisConfig::<T>::hash(&<OrderPairDetailMap<T>>::key_for(pair)).to_vec(), detail.encode());
                    p_list.push(pair.clone());
                }
                storage.insert(GenesisConfig::<T>::hash(&<OrderPairList<T>>::key()).to_vec(), p_list.encode());
        });
    }
}

impl<T: Trait> Module<T> {
    fn get_pair_by_second_token(second: &Token) -> Option<OrderPair> {
        let pair_list: Vec<OrderPair> = <OrderPairList<T>>::get();

        for i in 0..pair_list.len() {
            if pair_list[i].second.eq(second) {
                return Some(pair_list[i].clone());
            }
        }

        None
    }
    fn get_pair_by(first: &Token, second: &Token) -> Option<OrderPair> {
        let pair_list: Vec<OrderPair> = <OrderPairList<T>>::get();

        for i in 0..pair_list.len() {
            //if pair_list[i].first.clone() == first.clone() && pair_list[i].second.clone() == second.clone() {
            if pair_list[i].first.eq(first) && pair_list[i].second.eq(second) {
                return Some(pair_list[i].clone());
            } else {
            }
        }

        None
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

    pub fn set_average_price_len(val: T::Amount) -> Result {
        <AveragePriceLen<T>>::put(val);
        Self::deposit_event(RawEvent::SetAveragePriceLen(val));
        Ok(())
    }
    fn update_last_average_price(pair: &OrderPair, amount: T::Amount, price: T::Price) {
        if price > Zero::zero() && amount > Zero::zero() {
            <LastPrice<T>>::insert(pair.clone(), price);

            let last_average = match Self::last_average_price(pair.clone()) {
                None => {
                    <LastAveragePrice<T>>::insert(pair.clone(), price);
                    price
                }
                Some(p) => {
                    match <assets::Module<T>>::asset_info(pair.second.clone()) {
                        Some((asset, _, _)) => {
                            let average_sum: T::Amount = As::sa(
                                10_u64.pow(asset.precision().as_())
                                    * Self::average_price_len().as_(),
                            ); //将精度考虑进去
                            let last_average_price: T::Price = As::sa(
                                (p.as_() * average_sum.as_() + amount.as_() * price.as_())
                                    / (average_sum.as_() + amount.as_()),
                            );
                            <LastAveragePrice<T>>::insert(pair.clone(), last_average_price);
                            last_average_price
                        }
                        None => price,
                    }
                }
            };

            if pair.first.as_slice() == <assets::Module<T> as ChainT>::TOKEN {
                let v: T::Balance = As::sa(last_average.as_());
                assets::PCXPriceFor::<T>::insert(&pair.second, v);
            }
        }
    }
    /**
     * 返回另外一个token的数量
     */
    fn trans_amount(
        amount: T::Amount, /*token的数量单位*/
        price: T::Price,   /*以另一个token计价的价格*/
        token: &Token,
    ) -> Option<T::Balance> {
        match <assets::Module<T>>::asset_info(token) {
            Some((asset, _, _)) => {
                let trans_amount: T::Balance =
                    As::sa((amount.as_() * price.as_()) / (10_u64.pow(asset.precision().as_())));
                if trans_amount == Zero::zero() {
                    None
                } else {
                    Some(trans_amount)
                }
            }
            None => None,
        }
    }

    fn do_put_order(
        who: &T::AccountId,
        pair: &OrderPair,
        ordertype: OrderType,
        amount: T::Amount,
        price: T::Price,
        channel: &Channel,
    ) -> Result {
        //判断交易对是否存在
        if let Err(_) = Self::is_valid_pair(&pair) {
            return Err("not a existed pair in orderpair list");
        }
        //判定 数量和价格
        if amount == Zero::zero() {
            return Err("amount cann't be zero");
        }
        if price == Zero::zero() {
            return Err("price cann't be zero");
        }

        let sender = who;

        //计算总额
        let mut reserve_last: T::Amount = Zero::zero();

        if let Some(sum) = Self::trans_amount(amount, price, &pair.clone().first) {
            match ordertype {
                OrderType::Buy => {
                    if <assets::Module<T>>::free_balance(sender, &pair.second) < sum {
                        return Err("transactor's free token balance too low, can't put buy order");
                    }
                    reserve_last = As::sa(sum.as_());
                    //  锁定用户资产
                    if let Err(msg) = <assets::Module<T>>::reserve(
                        sender,
                        &pair.second,
                        sum,
                        assets::AssetType::ReservedDexSpot,
                    ) {
                        return Err(msg);
                    }
                }
                OrderType::Sell => {
                    if <assets::Module<T>>::free_balance(sender, &pair.first) < As::sa(amount.as_())
                    {
                        return Err("transactor's free token balance too low, can't put sell order");
                    }
                    //  锁定用户资产
                    reserve_last = amount;
                    if let Err(msg) = <assets::Module<T>>::reserve(
                        sender,
                        &pair.first,
                        As::sa(amount.as_()),
                        assets::AssetType::ReservedDexSpot,
                    ) {
                        return Err(msg);
                    }
                }
            }
        } else {
            return Err("amount*price too small");
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
            amount: amount,
            channel: channel.clone(),
            hasfill_amount: Zero::zero(),
            price: price,
            create_time: <system::Module<T>>::block_number(),
            lastupdate_time: <system::Module<T>>::block_number(),
            status: OrderStatus::FillNo,
            fill_index: Default::default(),
            reserve_last: reserve_last,
        };
        Self::insert_order(new_last_index, &order);

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
        if OrderStatus::FillAll == order.status()
            || OrderStatus::FillPartAndCancel == order.status()
            || OrderStatus::Cancel == order.status()
        {
            //Note: 删除掉订单
            //<OrdersOf<T>>::remove((order.user.clone(), order.pair.clone(), index));
        }
        // 每次更新都记录日志
        Self::deposit_event(RawEvent::UpdateOrder(
            order.pair().clone(),
            order.index(),
            order.class(),
            order.user(),
            order.amount(),
            order.channel(),
            order.hasfill_amount(),
            order.price(),
            order.create_time(),
            order.lastupdate_time(),
            order.status(),
            order.fill_index(),
            order.reserve_last(),
        ));
    }
    fn do_cancel_order(origin: T::Origin, pair: OrderPair, index: u64) -> Result {
        runtime_io::print("[exchange pendingorders] cancel_order");
        let transactor = ensure_signed(origin)?;

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

                    //回退用户资产
                    let back_token: Token = match order.class {
                        OrderType::Sell => pair.clone().first,
                        OrderType::Buy => pair.clone().second,
                    };

                    let back_amount: T::Balance = match order.class {
                        OrderType::Sell => As::sa(order.amount.as_() - order.hasfill_amount.as_()),
                        OrderType::Buy => As::sa(order.reserve_last.as_()), //剩余的都退回
                    };

                    if let Err(msg) = <assets::Module<T>>::unreserve(
                        &transactor.clone(),
                        &back_token,
                        back_amount,
                        assets::AssetType::ReservedDexSpot,
                    ) {
                        return Err(msg);
                    }

                    order.reserve_last = As::sa(order.reserve_last.as_() - back_amount.as_());

                    Self::insert_order(index, &order);

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
    pub fn channel_account(order: &OrderT<T>) -> T::AccountId {
        //从channel模块获得channel_name对应的account
        match <xaccounts::Module<T>>::intention_of(&order.channel) {
            Some(relation) => relation,
            None => <xsystem::Module<T>>::death_account(), //如果没有渠道，那么就销毁
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
        //逻辑校验 在调用方撮合模块中实现，此处只维护挂单、成交历史、资产转移
        let new_last_fill_index = Self::last_fill_index_of_pair(&pair) + 1;

        //更新maker对应的订单
        let mut maker_order = if let Some(mut maker_order) =
            Self::order_of((maker_user.clone(), pair.clone(), maker_user_order_index))
        {
            maker_order.fill_index.push(new_last_fill_index);
            maker_order.hasfill_amount = maker_order.hasfill_amount + amount;
            if maker_order.hasfill_amount == maker_order.amount {
                maker_order.status = OrderStatus::FillAll;
            } else if maker_order.hasfill_amount < maker_order.amount {
                maker_order.status = OrderStatus::FillPart;
            } else {
                return Err(" maker order has not enough amount");
            }

            maker_order.lastupdate_time = <system::Module<T>>::block_number();
            maker_order
        } else {
            return Err("cann't find this maker order");
        };

        //更新taker对应的订单
        let mut taker_order = if let Some(mut taker_order) =
            Self::order_of((taker_user.clone(), pair.clone(), taker_user_order_index))
        {
            taker_order.fill_index.push(new_last_fill_index);
            taker_order.hasfill_amount = taker_order.hasfill_amount + amount;
            if taker_order.hasfill_amount == taker_order.amount {
                taker_order.status = OrderStatus::FillAll;
            } else if taker_order.hasfill_amount < taker_order.amount {
                taker_order.status = OrderStatus::FillPart;
            } else {
                return Err(" taker order has not enough amount");
            }

            taker_order.lastupdate_time = <system::Module<T>>::block_number();
            taker_order
        } else {
            return Err("cann't find this taker order");
        };

        let mut after_maker_fee: T::Amount = maker_fee;
        let maker_fee_token: Token;
        let mut after_taker_fee: T::Amount = taker_fee;
        let taker_fee_token: Token;

        //转移 maker和taker中的资产
        match maker_order.class {
            OrderType::Sell => {
                //卖家先解锁first token 并move给买家，和手续费账户
                let maker_back_token: Token = pair.clone().first;
                let maker_back_amount: T::Balance = As::sa(amount.as_());
                maker_order.reserve_last =
                    As::sa(maker_order.reserve_last.as_() - maker_back_amount.as_());

                match Self::move_token_and_handle_fee(
                    &maker_back_token,
                    maker_back_amount,
                    As::sa(taker_fee.as_()),
                    &maker_user.clone(),
                    &taker_user.clone(),
                    &Self::channel_account(&maker_order.clone()).clone(),
                ) {
                    Ok((token, fee)) => {
                        taker_fee_token = token;
                        after_taker_fee = As::sa(fee.as_());
                    }
                    Err(msg) => {
                        return Err(msg);
                    }
                };

                //计算买家的数量，解锁second,并move 给卖家,和手续费账户
                let taker_back_token: Token = pair.clone().second;
                let taker_back_amount: T::Balance =
                    match Self::trans_amount(amount, price, &pair.clone().first) {
                        Some(sum) => sum,
                        None => Zero::zero(),
                    };
                taker_order.reserve_last =
                    As::sa(taker_order.reserve_last.as_() - taker_back_amount.as_());

                let maker_fee_back_amount: T::Balance =
                    match Self::trans_amount(maker_fee, price, &pair.clone().first) {
                        Some(sum) => sum,
                        None => Zero::zero(),
                    };
                match Self::move_token_and_handle_fee(
                    &taker_back_token,
                    taker_back_amount,
                    maker_fee_back_amount,
                    &taker_user.clone(),
                    &maker_user.clone(),
                    &Self::channel_account(&taker_order.clone()).clone(),
                ) {
                    Ok((token, fee)) => {
                        maker_fee_token = token;
                        after_maker_fee = As::sa(fee.as_());
                        ;
                    }
                    Err(msg) => {
                        return Err(msg);
                    }
                };
            }
            OrderType::Buy => {
                //买先解锁second token 并move给卖家，和手续费账户
                let maker_back_token: Token = pair.clone().second;
                let maker_back_amount: T::Balance =
                    match Self::trans_amount(amount, price, &pair.clone().first) {
                        Some(sum) => sum,
                        None => Zero::zero(),
                    };
                maker_order.reserve_last =
                    As::sa(maker_order.reserve_last.as_() - maker_back_amount.as_());

                let taker_fee_back_amount: T::Balance =
                    match Self::trans_amount(taker_fee, price, &pair.clone().first) {
                        Some(sum) => sum,
                        None => Zero::zero(),
                    };

                match Self::move_token_and_handle_fee(
                    &maker_back_token,
                    maker_back_amount,
                    taker_fee_back_amount,
                    &maker_user.clone(),
                    &taker_user.clone(),
                    &Self::channel_account(&maker_order.clone()).clone(),
                ) {
                    Ok((token, fee)) => {
                        taker_fee_token = token;
                        after_taker_fee = As::sa(fee.as_());
                        ;
                    }
                    Err(msg) => {
                        return Err(msg);
                    }
                };
                //计算卖家的数量，解锁second,并move 给买家,和手续费账户
                let taker_back_token: Token = pair.clone().first;
                let taker_back_amount: T::Balance = As::sa(amount.as_());
                taker_order.reserve_last =
                    As::sa(taker_order.reserve_last.as_() - taker_back_amount.as_());

                match Self::move_token_and_handle_fee(
                    &taker_back_token,
                    taker_back_amount,
                    As::sa(maker_fee.as_()),
                    &taker_user.clone(),
                    &maker_user.clone(),
                    &Self::channel_account(&taker_order.clone()).clone(),
                ) {
                    Ok((token, fee)) => {
                        maker_fee_token = token;
                        after_maker_fee = As::sa(fee.as_());
                        ;
                    }
                    Err(msg) => {
                        return Err(msg);
                    }
                }
            }
        }

        //插入新的成交记录
        let fill = Fill {
            pair: pair.clone(),
            index: new_last_fill_index,
            maker_user: maker_user.clone(),
            taker_user: taker_user.clone(),
            maker_user_order_index: maker_order.index,
            taker_user_order_index: taker_order.index,
            price: price,
            amount: amount,
            maker_fee: after_maker_fee,
            maker_fee_token: maker_fee_token,
            taker_fee: after_taker_fee,
            taker_fee_token: taker_fee_token,
            time: <system::Module<T>>::block_number(),
        };

        <FillIndexOf<T>>::insert(&pair, new_last_fill_index);

        Self::update_last_average_price(&pair.clone(), amount, price); //更新平均价格

        //插入更新后的maker对应的订单
        Self::insert_order(maker_order.index(), &maker_order);
        //插入更新后的taker对应的订单
        Self::insert_order(taker_order.index(), &taker_order);

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
            fill.maker_fee_token,
            fill.taker_fee,
            fill.taker_fee_token,
            <system::Module<T>>::block_number(),
        ));

        Ok(())
    }
    //计算手续费折扣
    fn discount_fee(account: &T::AccountId, token: &Token, amount: T::Amount) -> T::Amount {
        match <assets::Module<T>>::asset_info(token) {
            Some((asset, _, _)) => {
                //计算关联账户的额度
                let total_token = <assets::Module<T>>::free_balance(account, &asset.token());

                //将精度考虑进去
                let after_discount: T::Amount = if total_token > As::sa(0) {
                    if total_token <= As::sa(10_u64.pow(asset.precision().as_()) * 10000) {
                        As::sa((amount.as_() * 7_u64) / 10_u64)
                    } else if total_token <= As::sa(10_u64.pow(asset.precision().as_()) * 100000) {
                        As::sa((amount.as_() * 6_u64) / 10_u64)
                    } else if total_token <= As::sa(10_u64.pow(asset.precision().as_()) * 1000000) {
                        As::sa((amount.as_() * 5_u64) / 10_u64)
                    } else if total_token <= As::sa(10_u64.pow(asset.precision().as_()) * 10000000)
                    {
                        As::sa((amount.as_() * 3_u64) / 10_u64)
                    } else if total_token <= As::sa(10_u64.pow(asset.precision().as_()) * 100000000)
                    {
                        As::sa((amount.as_() * 2_u64) / 10_u64)
                    } else {
                        As::sa((amount.as_() * 1_u64) / 10_u64)
                    }
                } else {
                    amount
                };

                after_discount
            }
            None => amount,
        }
    }

    // 80% 销毁 20% 给渠道
    fn dispatch_fee(
        token: &Token,
        fee: T::Balance,
        from: &T::AccountId,
        destroy: &T::AccountId,
        channel: &T::AccountId,
    ) -> Result {
        let channel_fee: T::Balance = As::sa((fee.as_() * 2) / 10);
        let destroy_value: T::Balance = fee - channel_fee;
        if let Err(e) = <assets::Module<T>>::move_free_balance(
            &from.clone(),
            &destroy.clone(),
            &token.clone(),
            destroy_value,
        ) {
            return Err(e.info());
        }
        if channel_fee > Zero::zero() {
            if let Err(e) = <assets::Module<T>>::move_free_balance(
                &from.clone(),
                &channel.clone(),
                &token.clone(),
                channel_fee,
            ) {
                return Err(e.info());
            }
        }

        Ok(())
    }

    fn move_token_and_handle_fee(
        token: &Token,
        value: T::Balance,
        fee: T::Balance,
        from: &T::AccountId,
        to: &T::AccountId,
        channel: &T::AccountId,
    ) -> StdResult<(Token, T::Balance), &'static str> {
        /*
        if to == 销毁账户 && token==pcx
            不计算手续费 80%直接销毁 20% 给渠道
        else if from ==销毁账户
            手续费买盘的对手盘，不收手续费 避免死循环
        else {

            if token == pcx
                计算to的折扣后手续费
                折扣后的手续费 扣取 80%直接销毁 20% 给渠道
            else {
                获取平均的pcx价格
                计算等额的pcx手续费
                计算to的折扣后手续费
                if to的pcx余额足够折扣后的手续费
                    直接 折扣后手续费 80% 20%
                else {
                    扣手续费
                    生成购买pcx的买单
                }
            }
        }
        */
        let mut after_fee: T::Balance = fee;
        let mut fee_token: Token = token.clone();
        // 先把钱全部撤回
        if let Err(msg) =
            <assets::Module<T>>::unreserve(from, token, value, assets::AssetType::ReservedDexSpot)
        {
            return Err(msg);
        }

        if to == &<xsystem::Module<T>>::burn_account()
            && token.as_slice() == <assets::Module<T> as ChainT>::TOKEN
        {
            //前面自动生成的buy交易，不计算手续费 80%直接销毁 20% 给渠道
            if let Err(msg) = Self::dispatch_fee(
                token,
                value,
                from,
                &<xsystem::Module<T>>::death_account(),
                &channel.clone(),
            ) {
                return Err(msg);
            };
            after_fee = Zero::zero();
        } else if from == &<xsystem::Module<T>>::burn_account() {
            // 和手续费买盘的对手盘，不收手续费
            if let Err(e) = <assets::Module<T>>::move_free_balance(
                &from.clone(),
                &to.clone(),
                &token.clone(),
                value,
            ) {
                return Err(e.info());
            }
            after_fee = Zero::zero();
        } else {
            if token.as_slice() == <assets::Module<T> as ChainT>::TOKEN {
                let discount_fee: T::Balance =
                    As::sa(Self::discount_fee(&to, &token.clone(), As::sa(fee.as_())).as_());
                if let Err(e) = <assets::Module<T>>::move_free_balance(
                    &from.clone(),
                    &to.clone(),
                    &token.clone(),
                    value - discount_fee,
                ) {
                    return Err(e.info());
                }

                if let Err(msg) = Self::dispatch_fee(
                    token,
                    discount_fee,
                    from,
                    &<xsystem::Module<T>>::death_account(),
                    &channel.clone(),
                ) {
                    return Err(msg);
                };
                after_fee = discount_fee;
            } else {
                let option_average_price = match Self::get_pair_by_second_token(&token.clone()) {
                    Some(pair) => {
                        Self::last_average_price(pair.clone()) //如果能获取到与pcx的平均成交价
                    }
                    None => None,
                };
                match option_average_price {
                    Some(average_price) => {
                        let conversion_fee: T::Balance = match <assets::Module<T>>::asset_info(
                            <assets::Module<T> as ChainT>::TOKEN.to_vec(),
                        ) {
                            Some((asset, _, _)) => {
                                As::sa(
                                    (10_u64.pow(asset.precision().as_()) * fee.as_())
                                        / average_price.as_(),
                                ) //换算pcx手续费
                            }
                            None => Zero::zero(),
                        };

                        let discount_fee: T::Balance = As::sa(
                            Self::discount_fee(&to, &token.clone(), As::sa(conversion_fee.as_()))
                                .as_(),
                        );
                        if <assets::Module<T>>::free_balance(
                            to,
                            &<assets::Module<T> as ChainT>::TOKEN.to_vec(),
                        ) >= discount_fee
                        {
                            // pcx余额足够
                            if let Err(e) = <assets::Module<T>>::move_free_balance(
                                &from.clone(),
                                &to.clone(),
                                &token.clone(),
                                value,
                            ) {
                                return Err(e.info());
                            }
                            if let Err(msg) = Self::dispatch_fee(
                                &<assets::Module<T> as ChainT>::TOKEN.to_vec(),
                                discount_fee,
                                to,
                                &<xsystem::Module<T>>::death_account(),
                                &channel.clone(),
                            ) {
                                return Err(msg);
                            };
                            fee_token = <assets::Module<T> as ChainT>::TOKEN.to_vec();
                            after_fee = discount_fee;
                        } else {
                            if let Err(e) = <assets::Module<T>>::move_free_balance(
                                &from.clone(),
                                &to.clone(),
                                &token.clone(),
                                value - fee,
                            ) {
                                return Err(e.info());
                            }
                            if let Err(e) = <assets::Module<T>>::move_free_balance(
                                &from.clone(),
                                &<xsystem::Module<T>>::burn_account(),
                                &token.clone(),
                                fee,
                            ) {
                                return Err(e.info());
                            }
                            after_fee = fee;
                            //fee 生成购买pcx的订单
                            Self::new_fee_buy_order(token, fee, channel.clone());
                        }
                    }
                    None => {
                        //没有平均成交价，只能直接扣
                        if let Err(e) = <assets::Module<T>>::move_free_balance(
                            &from.clone(),
                            &to.clone(),
                            &token.clone(),
                            value - fee,
                        ) {
                            return Err(e.info());
                        }
                        if let Err(e) = <assets::Module<T>>::move_free_balance(
                            &from.clone(),
                            &<xsystem::Module<T>>::burn_account(),
                            &token.clone(),
                            fee,
                        ) {
                            return Err(e.info());
                        }
                        after_fee = fee;
                        //fee 生成购买pcx的订单
                        Self::new_fee_buy_order(token, fee, channel.clone());
                    }
                }
            }
        }
        Ok((fee_token, after_fee))
    }
    fn new_fee_buy_order(token: &Token, sum: T::Balance, channel: T::AccountId) {
        Self::deposit_event(RawEvent::FeeBuy(token.clone(), sum, channel.clone()));

        match Self::get_pair_by(&<assets::Module<T> as ChainT>::TOKEN.to_vec(), token) {
            Some(pair) => {
                if sum > Zero::zero() {
                    if let Some(last_price) = <LastPrice<T>>::get(pair.clone()) {
                        let amount: T::Amount = As::sa(sum.as_() / last_price.as_());
                        if amount > Zero::zero() {
                            let fee_buy_order_max: u64 = Self::fee_buy_order_max() + 1;

                            <FeeBuyOrder<T>>::insert(
                                fee_buy_order_max,
                                (pair.clone(), amount, last_price, channel.clone()),
                            );
                            <FeeBuyOrderMax<T>>::put(fee_buy_order_max);
                        }
                    } else {
                    }
                }
            }
            None => {}
        }
    }

    pub fn clear_command_and_put_fee_buy_order() {
        <MaxCommandId<T>>::put(0);

        let fee_buy_order_max: u64 = Self::fee_buy_order_max();

        for id in 1..(fee_buy_order_max + 1) {
            if let Some(buy) = <FeeBuyOrder<T>>::get(id) {
                let channel_name =
                    match <xaccounts::Module<T>>::intention_immutable_props_of(&buy.3.clone()) {
                        Some(intention) => intention.name,
                        None => b"".to_vec(),
                    };
                let _ = Self::do_put_order(
                    &<xsystem::Module<T>>::burn_account(),
                    &buy.0,
                    OrderType::Buy,
                    buy.1,
                    buy.2,
                    &channel_name,
                );
            }
        }

        //清空
        <FeeBuyOrderMax<T>>::put(0);
    }
}

pub type Channel = Vec<u8>;

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

#[derive(PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub struct OrderPair {
    pub first: Token,
    pub second: Token,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub struct OrderPairDetail {
    pub precision: u32, //价格精度
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
    maker_fee_token: Token,
    taker_fee: Amount,
    taker_fee_token: Token,
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
    channel: Channel,
    hasfill_amount: Amount,
    price: Price,
    create_time: BlockNumber,
    lastupdate_time: BlockNumber,
    status: OrderStatus,
    fill_index: Vec<u128>,
    // 填充历史记录的索引
    reserve_last: Amount, //未被交易 未被回退
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
        channel: Channel,
        hasfill_amount: Amount,
        price: Price,
        create_time: BlockNumber,
        lastupdate_time: BlockNumber,
        status: OrderStatus,
        fill_index: Vec<u128>,
        reserve_last: Amount,
    ) -> Self {
        return Order {
            pair: pair,
            index: index,
            class: class,
            user: user,
            amount: amount,
            channel: channel,
            hasfill_amount: hasfill_amount,
            price: price,
            create_time: create_time,
            lastupdate_time: lastupdate_time,
            status: status,
            fill_index: fill_index,
            reserve_last: reserve_last,
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
    pub fn channel(&self) -> Channel {
        self.channel.clone()
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
    pub fn reserve_last(&self) -> Amount {
        self.reserve_last
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
}
