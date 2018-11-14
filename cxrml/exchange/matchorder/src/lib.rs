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
extern crate cxrml_exchange_pendingorders as pendingorders;
extern crate cxrml_support as cxsupport;
extern crate cxrml_tokenbalances as tokenbalances;

#[cfg(test)]
mod tests;

use rstd::prelude::*;
//use runtime_primitives::traits::OnFinalise;
use codec::Codec;
use cxsupport::storage::linked_node::{LinkedNodeCollection, MultiNodeIndex, Node, NodeT};
use pendingorders::{CommandType, OrderPair, OrderType};
use runtime_primitives::traits::OnFinalise;
use runtime_primitives::traits::{As, Zero};
use runtime_support::dispatch::Result;
use runtime_support::{StorageMap, StorageValue};

pub trait Trait: tokenbalances::Trait + pendingorders::Trait {
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn set_match_fee(val: T::Balance) -> Result;

    }
}

decl_event!(
    pub enum Event<T> where
        <T as system::Trait>::AccountId,
        <T as system::Trait>::BlockNumber,
        <T as balances::Trait>::Balance,
        <T as pendingorders::Trait>::Amount,
        <T as pendingorders::Trait>::Price
    {
        SetMatchFee(Balance),
        AddBid(OrderPair,AccountId,u64,Price, Amount,BlockNumber),
        CancelBid(OrderPair,AccountId,u64,BlockNumber),

        MatchFail(BidId,OrderPair,AccountId,AccountId,u64,u64,Price,Amount,Amount,Amount,BlockNumber),
    }
);

decl_storage! {
    trait Store for Module<T: Trait> as MatchOrder {
        pub MatchFee get(match_fee) config(): T::Balance;

        // 维护有序，价格优先，时间优先
        pub BidListHeaderFor get(bidlist_header_for): map (OrderPair,OrderType) => Option<MultiNodeIndex<(OrderPair,OrderType), BidT<T>>>;
        pub BidListTailFor get(bidlist_tail_for): map (OrderPair,OrderType) => Option<MultiNodeIndex<(OrderPair,OrderType), BidT<T>>>;
        pub BidListCache get(bidlist_cache): map u128 => Option<Node<BidT<T>>>;
            NodeId get(nodeid):u128;

        pub BidOf get(bid_of):map BidId => Option<BidDetailT<T>>;
        pub LastBidIndexOf get(last_bid_index_of): BidId;

        pub BidOfUserOrder get( bid_of_user_order) : map (T::AccountId,OrderPair,u64) => BidId; //索引  accountid+orderindex=>bidid
    }
}

pub struct LinkedMultiKey<T: Trait>(runtime_support::storage::generator::PhantomData<T>);
impl<T: Trait> LinkedNodeCollection for LinkedMultiKey<T> {
    type Header = BidListHeaderFor<T>;
    type NodeMap = BidListCache<T>;
    type Tail = BidListTailFor<T>;
}

/// 盘口 记录 聚合
#[derive(PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub struct Bid<Amount, Price>
where
    Amount: Copy,
    Price: Copy,
{
    nodeid: u128,
    price: Price,
    sum: Amount,
    list: Vec<BidId>,
}

pub type BidT<T> = Bid<<T as pendingorders::Trait>::Amount, <T as pendingorders::Trait>::Price>;

impl<Amount, Price> NodeT for Bid<Amount, Price>
where
    Price: Codec + Clone + Eq + PartialEq + Default + Copy,
    Amount: Copy,
{
    type Index = u128;

    fn index(&self) -> Self::Index {
        self.nodeid
    }
}

impl<T: Trait> OnFinalise<T::BlockNumber> for Module<T> {
    fn on_finalise(time: T::BlockNumber) {
        //先读取pendingorders模块的所有新挂单

        let max_command_id: u64 = <pendingorders::Module<T>>::max_command_id();
        info!("on_finalise:max_command_id {:?}", max_command_id);
        for command_id in 1..(max_command_id + 1) {
            info!("on_finalise: command id {:?}", command_id);
            if let Some(command) = <pendingorders::Module<T>>::command_of(command_id) {
                if let Some(order) = <pendingorders::Module<T>>::order_of((
                    command.0.clone(),
                    command.1.clone(),
                    command.2,
                )) {
                    if command.3 == CommandType::Cancel {
                        let cancel_bid = Self::bid_of_user_order((
                            command.0.clone(),
                            command.1.clone(),
                            command.2,
                        )); //找出老的bid
                        <pendingorders::Module<T>>::update_command_of(command_id, cancel_bid);
                        // 记录日志
                        Self::deposit_event(RawEvent::CancelBid(
                            order.pair().clone(),
                            order.user().clone(),
                            order.index(),
                            <system::Module<T>>::block_number(),
                        ));
                    } else {
                        let new_last_bid_index = Self::last_bid_index_of() + 1;
                        <LastBidIndexOf<T>>::put(new_last_bid_index);
                        let bid = BidDetail {
                            id: new_last_bid_index,
                            pair: order.pair().clone(),
                            order_type: order.class(),
                            user: order.user().clone(),
                            order_index: order.index(),
                            price: order.price(),
                            amount: order.amount(),
                            time: <system::Module<T>>::block_number(),
                        };
                        <BidOf<T>>::insert(new_last_bid_index, bid.clone());
                        <BidOfUserOrder<T>>::insert(
                            (order.user().clone(), order.pair().clone(), order.index()),
                            new_last_bid_index,
                        ); //建立映射

                        <pendingorders::Module<T>>::update_command_of(
                            command_id,
                            new_last_bid_index,
                        );
                        // 记录日志
                        Self::deposit_event(RawEvent::AddBid(
                            bid.pair.clone(),
                            bid.user.clone(),
                            bid.order_index,
                            bid.price,
                            bid.amount,
                            <system::Module<T>>::block_number(),
                        ));
                    }
                }
            }
        }

        //块最后结束的时候，执行撮合
        Self::handle_match(time);

        //清空
        <pendingorders::Module<T>>::clear_command();
    }
}

impl<T: Trait> Module<T> {
    /// Deposit one of this module's events.
    pub fn deposit_event(event: Event<T>) {
        <system::Module<T>>::deposit_event(<T as Trait>::Event::from(event).into());
    }
    pub fn set_match_fee(val: T::Balance) -> Result {
        <MatchFee<T>>::put(val);
        Self::deposit_event(RawEvent::SetMatchFee(val));
        Ok(())
    }
    fn new_nodeid() -> u128 {
        let mut last_nodeid: u128 = <NodeId<T>>::get();
        last_nodeid = match last_nodeid.checked_add(1_u128) {
            Some(b) => b,
            None => 0,
        };
        <NodeId<T>>::put(last_nodeid);

        last_nodeid
    }
    //处理 撮合
    fn handle_match(_time: T::BlockNumber) {
        let max_command_id: u64 = <pendingorders::Module<T>>::max_command_id();
        info!("handle_match:max_command_id {:?}", max_command_id);
        //遍历每个bid
        for command_id in 1..(max_command_id + 1) {
            if let Some(command) = <pendingorders::Module<T>>::command_of(command_id) {
                info!(
                    "handle_match: command id {:?} command {:?}",
                    command_id, command.3
                );
                if let Some(mut in_bid_detail) = <BidOf<T>>::get(command.4) {
                    //找出该交易对的 目标单 列表
                    let find_type: OrderType = match in_bid_detail.order_type {
                        OrderType::Buy => OrderType::Sell,
                        OrderType::Sell => OrderType::Buy,
                    };

                    // wait_bid_list已经是有序的 价格优先 时间优先
                    //自身是卖单，找买单
                    //自身是买单，找卖单
                    match command.3 {
                        CommandType::Match => {
                            Self::do_match(find_type, &mut in_bid_detail);

                            if in_bid_detail.amount == Zero::zero() {
                                //已被匹配完毕，则删除
                                <BidOf<T>>::remove(in_bid_detail.id);
                            } else {
                                //如果还有剩余，则将其更新到bid_list中
                                Self::insert_bid_list(&in_bid_detail);
                            }
                        }
                        CommandType::Cancel => {
                            // 取消挂单
                            Self::cancel_bid(&in_bid_detail);
                        }
                    }
                }
            }
        }
    }

    fn do_match(find_type: OrderType, in_bid_detail: &mut BidDetailT<T>) {
        //wait_bid_list 是价格有序 时间有序
        let mut need_fill: T::Amount = in_bid_detail.amount;
        let mut remove_from_wait_bid_list: Vec<BidT<T>> = Vec::new();
        info!("do_match:{:?}", in_bid_detail);

        if let Some(header) = Self::bidlist_header_for((in_bid_detail.pair.clone(), find_type)) {
            let mut index = header.index();

            let mut find_match = false;
            while let Some(mut node) = Self::bidlist_cache(&index) {
                info!("do_match:index={:?} {:?}", index, find_match);
                match in_bid_detail.order_type {
                    OrderType::Sell => {
                        if (need_fill != Zero::zero()) && (in_bid_detail.price <= node.data.price) {
                            find_match = true;
                        }
                    }
                    OrderType::Buy => {
                        if (need_fill != Zero::zero()) && (in_bid_detail.price >= node.data.price) {
                            find_match = true;
                        }
                    }
                }
                //info!("do_match:index={:?} {:?}", index,find_match);
                if find_match == true {
                    //找到匹配的 计算手续费 构建fill order
                    let mut fill_num: T::Amount;
                    if need_fill < node.data.sum {
                        fill_num = need_fill;
                    } else {
                        fill_num = node.data.sum;
                        remove_from_wait_bid_list.push(node.data.clone()); //计入删除
                    }
                    need_fill = need_fill - fill_num;
                    in_bid_detail.amount = in_bid_detail.amount - fill_num;
                    node.data.sum = node.data.sum - fill_num;
                    // 一个个填充
                    let mut remove_from_list: Vec<BidId> = Vec::new();

                    for kk in 0..node.data.list.len() {
                        if let Some(mut match_bid) = Self::bid_of(node.data.list[kk]) {
                            let maker_user = match_bid.user.clone();
                            let taker_user = in_bid_detail.user.clone();
                            let maker_user_order_index = match_bid.order_index;
                            let taker_user_order_index = in_bid_detail.order_index;
                            let order_price = match_bid.price;
                            let mut amount: T::Amount;
                            let maker_fee: T::Amount = As::sa(0); //默认先0 手续费
                            let taker_fee: T::Amount = As::sa(0); //默认先0 手续费

                            if fill_num >= match_bid.amount {
                                amount = match_bid.amount;
                                //被撮合完了，删除
                                <BidOf<T>>::remove(match_bid.id);
                                remove_from_list.push(match_bid.id);
                            } else {
                                amount = fill_num;
                                match_bid.amount = match_bid.amount - amount;
                                <BidOf<T>>::insert(match_bid.id, match_bid.clone());
                            }

                            fill_num = fill_num - amount;
                            //成交
                            if let Err(msg) = <pendingorders::Module<T>>::fill_order(
                                in_bid_detail.pair.clone(),
                                maker_user.clone(),
                                taker_user.clone(),
                                maker_user_order_index,
                                taker_user_order_index,
                                order_price,
                                amount,
                                maker_fee,
                                taker_fee,
                            ) {
                                error!("do_match: match fail {:?}", msg);
                                Self::deposit_event(RawEvent::MatchFail(
                                    match_bid.id,
                                    in_bid_detail.pair.clone(),
                                    maker_user,
                                    taker_user,
                                    maker_user_order_index,
                                    taker_user_order_index,
                                    order_price,
                                    amount,
                                    maker_fee,
                                    taker_fee,
                                    <system::Module<T>>::block_number(),
                                ));
                            }

                            if fill_num == Zero::zero() {
                                break;
                            }
                        }
                    }

                    Self::remove_from_bid_list(&mut node, &remove_from_list);

                    if let Some(next) = node.next() {
                        index = next;
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }
        }

        // 删掉已经被撮合完的
        Self::remove_from_bid(&in_bid_detail.pair, find_type, &remove_from_wait_bid_list);
    }

    fn remove_from_bid_list(
        node: &mut Node<
            Bid<<T as pendingorders::Trait>::Amount, <T as pendingorders::Trait>::Price>,
        >,
        remove_id: &Vec<BidId>,
    ) {
        let mut new_list: Vec<BidId> = Vec::new();
        for mm in 0..node.data.list.len() {
            let mut remove = false;
            for nn in 0..remove_id.len() {
                if node.data.list[mm] == remove_id[nn] {
                    remove = true;
                    <BidOf<T>>::remove(remove_id[nn]);
                    info!("remove_from_bid_list:{:?}", remove_id[nn]);
                }
            }
            if remove == false {
                new_list.push(node.data.list[mm]);
            }
        }

        node.data.list = new_list;
        <BidListCache<T>>::insert(node.index(), node);
        //更新node
    }

    fn remove_from_bid(pair: &OrderPair, order_type: OrderType, remove_bid: &Vec<BidT<T>>) {
        for nn in 0..remove_bid.len() {
            if let Some(header) = Self::bidlist_header_for((pair.clone(), order_type)) {
                let mut index = header.index();

                while let Some(mut node) = Self::bidlist_cache(&index) {
                    if node.data.price == remove_bid[nn].price {
                        info!("remove_from_bid:{:?}", remove_bid[nn].price);

                        let _=node.remove_option_node_withkey::<LinkedMultiKey<T>, (OrderPair,OrderType)>((pair.clone(),order_type));
                        break;
                    }

                    if let Some(next) = node.next() {
                        index = next;
                    } else {
                        break;
                    }
                }
            }
        }
    }

    fn insert_bid_list(in_bid_detail: &BidDetailT<T>) {
        info!("insert_bid_list:{:?}", in_bid_detail);

        <BidOf<T>>::insert(in_bid_detail.id, in_bid_detail.clone());

        let mut finish = false;
        if let Some(header) =
            Self::bidlist_header_for((in_bid_detail.pair.clone(), in_bid_detail.order_type))
        {
            let mut index = header.index();

            while let Some(mut node) = Self::bidlist_cache(&index) {
                if in_bid_detail.price == node.data.price {
                    //累加
                    node.data.sum += in_bid_detail.amount;
                    node.data.list.push(in_bid_detail.id);

                    <BidListCache<T>>::insert(node.index(), node);

                    info!("insert_bid_list: insert add");
                    finish = true;
                    break;
                }
                let mut insert_head = false;

                match in_bid_detail.order_type {
                    OrderType::Sell => {
                        if in_bid_detail.price < node.data.price {
                            //插入当前的 前面
                            insert_head = true;
                        }
                    }
                    OrderType::Buy => {
                        if in_bid_detail.price > node.data.price {
                            insert_head = true;
                        }
                    }
                }

                if insert_head == true {
                    let new_nodeid = Self::new_nodeid();
                    let new_bid = Bid {
                        nodeid: new_nodeid,
                        price: in_bid_detail.price,
                        sum: in_bid_detail.amount,
                        list: vec![in_bid_detail.id],
                    };

                    let n = Node::new(new_bid);
                    n.init_storage_withkey::<LinkedMultiKey<T>, (OrderPair, OrderType)>((
                        in_bid_detail.pair.clone(),
                        in_bid_detail.order_type,
                    ));

                    let _=node.add_option_node_before_withkey::<LinkedMultiKey<T>, (OrderPair,OrderType)>(n,(in_bid_detail.pair.clone(),in_bid_detail.order_type));

                    info!("insert_bid_list: insert head");
                    finish = true;
                    break;
                }

                if let Some(next) = node.next() {
                    index = next;
                } else {
                    break;
                }
            }
        }
        if finish == false {
            //追加在最后
            let new_nodeid = Self::new_nodeid();
            let new_bid = Bid {
                nodeid: new_nodeid,
                price: in_bid_detail.price,
                sum: in_bid_detail.amount,
                list: vec![in_bid_detail.id],
            };
            let n = Node::new(new_bid);
            n.init_storage_withkey::<LinkedMultiKey<T>, (OrderPair, OrderType)>((
                in_bid_detail.pair.clone(),
                in_bid_detail.order_type,
            ));

            if let Some(tail_index) =
                Self::bidlist_tail_for((in_bid_detail.pair.clone(), in_bid_detail.order_type))
            {
                if let Some(mut tail_node) = Self::bidlist_cache(tail_index.index()) {
                    let _ = tail_node
                        .add_option_node_after_withkey::<LinkedMultiKey<T>, (OrderPair, OrderType)>(
                            n,
                            (in_bid_detail.pair.clone(), in_bid_detail.order_type),
                        );
                }
            }

            info!("insert_bid_list: insert tail");
        }
    }

    fn cancel_bid(in_bid_detail: &BidDetailT<T>) {
        <BidOf<T>>::remove(in_bid_detail.id);
        info!("cancel_bid:{:?}", in_bid_detail);

        let mut remove_from_wait_bid_list: Vec<BidT<T>> = Vec::new();
        if let Some(header) =
            Self::bidlist_header_for((in_bid_detail.pair.clone(), in_bid_detail.order_type))
        {
            let mut index = header.index();

            while let Some(mut node) = Self::bidlist_cache(&index) {
                if node.data.price == in_bid_detail.price {
                    node.data.sum = node.data.sum - in_bid_detail.amount;
                    if node.data.sum == Zero::zero() {
                        remove_from_wait_bid_list.push(node.data.clone()); //标记删除
                    }
                    for mm in 0..node.data.list.len() {
                        if in_bid_detail.id == node.data.list[mm] {
                            Self::remove_from_bid_list(&mut node, &vec![in_bid_detail.id]);
                            break;
                        }
                    }

                    break;
                }
                if let Some(next) = node.next() {
                    index = next;
                } else {
                    break;
                }
            }
        }

        Self::remove_from_bid(
            &in_bid_detail.pair,
            in_bid_detail.order_type,
            &remove_from_wait_bid_list,
        ); //最后更新
    }
}

pub type BidId = u128;

/// 盘口 记录 详情
#[derive(PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub struct BidDetail<Pair, AccountId, Amount, Price, BlockNumber>
where
    Pair: Clone,
    AccountId: Clone,
    Amount: Copy,
    Price: Copy,
    BlockNumber: Copy,
{
    id: BidId,
    pair: Pair,
    order_type: OrderType,
    user: AccountId,
    order_index: u64,
    price: Price,
    amount: Amount,
    time: BlockNumber,
}

pub type BidDetailT<T> = BidDetail<
    OrderPair,
    <T as system::Trait>::AccountId,
    <T as pendingorders::Trait>::Amount,
    <T as pendingorders::Trait>::Price,
    <T as system::Trait>::BlockNumber,
>;
