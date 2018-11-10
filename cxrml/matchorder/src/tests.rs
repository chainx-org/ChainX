// Copyright 2018 Chainpool.

use substrate_primitives::{Blake2Hasher, H256};

use runtime_io;
use runtime_io::with_externalities;
use runtime_primitives::testing::{Digest, DigestItem, Header};
use runtime_primitives::traits::BlakeTwo256;
use runtime_primitives::BuildStorage;

use super::*;
use pendingorders::{Order, OrderPair, OrderStatus, OrderType};
use std::str;
use tokenbalances::{DescString, Precision, SymbolString, Token};

impl_outer_origin! {
    pub enum Origin for Test {}
}

#[derive(Clone, Eq, PartialEq)]
pub struct Test;

impl system::Trait for Test {
    type Origin = Origin;
    type Index = u64;
    type BlockNumber = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type Digest = Digest;
    type AccountId = u64;
    type Header = Header;
    type Event = ();
    type Log = DigestItem;
}

impl balances::Trait for Test {
    type Balance = u64;
    type AccountIndex = u64;
    type OnFreeBalanceZero = ();
    type EnsureAccountLiquid = ();
    type Event = ();
}

impl cxsupport::Trait for Test {}

impl pendingorders::Trait for Test {
    type Event = ();
    type Amount = u128;
    type Price = u128;
}

// define tokenbalances module type
pub type TokenBalance = u128;

impl tokenbalances::Trait for Test {
    const CHAINX_SYMBOL: SymbolString = b"pcx";
    const CHAINX_PRECISION: Precision = 8;
    const CHAINX_TOKEN_DESC: DescString = b"this is pcx for mock";
    type TokenBalance = TokenBalance;
    type Event = ();
}

// This function basically just builds a genesis storage key/value store according to
// our desired mockup.
pub fn new_test_ext() -> runtime_io::TestExternalities<Blake2Hasher> {
    let mut r = system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap();
    // balance
    r.extend(
        balances::GenesisConfig::<Test> {
            balances: vec![(1, 10000), (2, 10000)],
            transaction_base_fee: 0,
            transaction_byte_fee: 0,
            existential_deposit: 500,
            transfer_fee: 0,
            creation_fee: 0,
            reclaim_rebate: 0,
        }.build_storage()
        .unwrap(),
    );

    r.extend(
        tokenbalances::GenesisConfig::<Test> {
            token_list: vec![],
            transfer_token_fee: 10,
        }.build_storage()
        .unwrap(),
    );

    r.extend(
        pendingorders::GenesisConfig::<Test> {
            order_fee: 10,
            pair_list: vec![],
            max_command_id: 0,
        }.build_storage()
        .unwrap(),
    );

    r.extend(
        GenesisConfig::<Test> { match_fee: 10 }
            .build_storage()
            .unwrap(),
    );
    r.into()
}

impl Trait for Test {
    type Event = ();
}

type MatchOrder = Module<Test>;
type TokenBalances = tokenbalances::Module<Test>;
type Balances = balances::Module<Test>;
type PendingOrders = pendingorders::Module<Test>;

#[test]
fn test_fee() {
    with_externalities(&mut new_test_ext(), || {
        MatchOrder::set_match_fee(20);

        assert_eq!(MatchOrder::match_fee(), 20);
    })
}

#[test]
fn test_match_part() {
    with_externalities(&mut new_test_ext(), || {
        let t_sym_eos = b"x-eos".to_vec();
        let t_desc_eos = b"eos token".to_vec();
        let precision = 4;
        let t_eos: Token = Token::new(t_sym_eos.clone(), t_desc_eos.clone(), precision);
        assert_eq!(TokenBalances::register_token(t_eos, 0, 0), Ok(()));

        let t_sym_eth = b"x-eth".to_vec();
        let t_desc_eth = b"eth token".to_vec();
        let precision = 4;
        let t_eth: Token = Token::new(t_sym_eth.clone(), t_desc_eth.clone(), precision);
        assert_eq!(TokenBalances::register_token(t_eth, 0, 0), Ok(()));

        let p1 = OrderPair::new(t_sym_eos.clone(), t_sym_eth.clone(), 0);

        // 增加交易对
        PendingOrders::add_pair(p1.clone()).unwrap();

        let a: u64 = 1; // accountid
        let b: u64 = 2;

        // 发放
        TokenBalances::issue(&a, &t_sym_eos.clone(), 1000).unwrap();
        TokenBalances::issue(&a, &t_sym_eth.clone(), 1000).unwrap();
        TokenBalances::issue(&b, &t_sym_eos.clone(), 1000).unwrap();
        TokenBalances::issue(&b, &t_sym_eth.clone(), 1000).unwrap();

        //挂买单
        let buy = OrderType::Buy;
        let a_order = PendingOrders::put_order(Some(a).into(), p1.clone(), buy, 100, 5);
        assert_eq!(TokenBalances::free_token(&(a, t_sym_eos.clone())), 1000);
        assert_eq!(TokenBalances::reserved_token(&(a, t_sym_eos.clone())), 0);
        assert_eq!(TokenBalances::free_token(&(a, t_sym_eth.clone())), 500);
        assert_eq!(TokenBalances::reserved_token(&(a, t_sym_eth.clone())), 500);

        //挂卖单
        let sell = OrderType::Sell;
        let b_order = PendingOrders::put_order(Some(b).into(), p1.clone(), sell, 50, 5);
        assert_eq!(b_order, Ok(()));
        assert_eq!(TokenBalances::free_token(&(b, t_sym_eos.clone())), 950);
        assert_eq!(TokenBalances::reserved_token(&(b, t_sym_eos.clone())), 50);
        assert_eq!(TokenBalances::free_token(&(b, t_sym_eth.clone())), 1000);
        assert_eq!(TokenBalances::reserved_token(&(b, t_sym_eth.clone())), 0);

        print_bid(p1.clone(), OrderType::Sell);
        print_bid(p1.clone(), OrderType::Buy);

        <MatchOrder as OnFinalise<u64>>::on_finalise(1);

        //1000+250
        assert_eq!(TokenBalances::free_token(&(a, t_sym_eos.clone())), 1050);
        assert_eq!(TokenBalances::reserved_token(&(a, t_sym_eos.clone())), 0);
        //1000-500
        assert_eq!(TokenBalances::free_token(&(a, t_sym_eth.clone())), 500);
        //500-250
        assert_eq!(TokenBalances::reserved_token(&(a, t_sym_eth.clone())), 250);

        //1000-50
        assert_eq!(TokenBalances::free_token(&(b, t_sym_eos.clone())), 950);
        assert_eq!(TokenBalances::reserved_token(&(b, t_sym_eos.clone())), 0);
        assert_eq!(TokenBalances::free_token(&(b, t_sym_eth.clone())), 1250);
        assert_eq!(TokenBalances::reserved_token(&(b, t_sym_eth.clone())), 0);

        print_bid(p1.clone(), OrderType::Sell);
        print_bid(p1.clone(), OrderType::Buy);
    })
}

#[test]
fn test_match_all() {
    with_externalities(&mut new_test_ext(), || {
        let t_sym_eos = b"x-eos".to_vec();
        let t_desc_eos = b"eos token".to_vec();
        let precision = 4;
        let t_eos: Token = Token::new(t_sym_eos.clone(), t_desc_eos.clone(), precision);
        assert_eq!(TokenBalances::register_token(t_eos, 0, 0), Ok(()));

        let t_sym_eth = b"x-eth".to_vec();
        let t_desc_eth = b"eth token".to_vec();
        let precision = 4;
        let t_eth: Token = Token::new(t_sym_eth.clone(), t_desc_eth.clone(), precision);
        assert_eq!(TokenBalances::register_token(t_eth, 0, 0), Ok(()));

        let p1 = OrderPair::new(t_sym_eos.clone(), t_sym_eth.clone(), 0);

        // 增加交易对
        PendingOrders::add_pair(p1.clone()).unwrap();

        let a: u64 = 1; // accountid
        let b: u64 = 2;

        // 发放
        TokenBalances::issue(&a, &t_sym_eos.clone(), 1000).unwrap();
        TokenBalances::issue(&a, &t_sym_eth.clone(), 1000).unwrap();
        TokenBalances::issue(&b, &t_sym_eos.clone(), 1000).unwrap();
        TokenBalances::issue(&b, &t_sym_eth.clone(), 1000).unwrap();

        //挂买单
        let buy = OrderType::Buy;
        let a_order = PendingOrders::put_order(Some(a).into(), p1.clone(), buy, 100, 5);
        assert_eq!(TokenBalances::free_token(&(a, t_sym_eos.clone())), 1000);
        assert_eq!(TokenBalances::reserved_token(&(a, t_sym_eos.clone())), 0);
        assert_eq!(TokenBalances::free_token(&(a, t_sym_eth.clone())), 500);
        assert_eq!(TokenBalances::reserved_token(&(a, t_sym_eth.clone())), 500);

        //挂卖单
        let sell = OrderType::Sell;
        let b_order = PendingOrders::put_order(Some(b).into(), p1.clone(), sell, 100, 5);
        assert_eq!(b_order, Ok(()));
        assert_eq!(TokenBalances::free_token(&(b, t_sym_eos.clone())), 900);
        assert_eq!(TokenBalances::reserved_token(&(b, t_sym_eos.clone())), 100);
        assert_eq!(TokenBalances::free_token(&(b, t_sym_eth.clone())), 1000);
        assert_eq!(TokenBalances::reserved_token(&(b, t_sym_eth.clone())), 0);

        print_bid(p1.clone(), OrderType::Sell);
        print_bid(p1.clone(), OrderType::Buy);

        <MatchOrder as OnFinalise<u64>>::on_finalise(1);

        //1000+250
        assert_eq!(TokenBalances::free_token(&(a, t_sym_eos.clone())), 1100);
        assert_eq!(TokenBalances::reserved_token(&(a, t_sym_eos.clone())), 0);
        //1000-500
        assert_eq!(TokenBalances::free_token(&(a, t_sym_eth.clone())), 500);
        //500-250
        assert_eq!(TokenBalances::reserved_token(&(a, t_sym_eth.clone())), 0);

        //1000-50
        assert_eq!(TokenBalances::free_token(&(b, t_sym_eos.clone())), 900);
        assert_eq!(TokenBalances::reserved_token(&(b, t_sym_eos.clone())), 0);
        assert_eq!(TokenBalances::free_token(&(b, t_sym_eth.clone())), 1500);
        assert_eq!(TokenBalances::reserved_token(&(b, t_sym_eth.clone())), 0);

        print_bid(p1.clone(), OrderType::Sell);
        print_bid(p1.clone(), OrderType::Buy);
    })
}

#[test]
fn test_match_no() {
    with_externalities(&mut new_test_ext(), || {
        let t_sym_eos = b"x-eos".to_vec();
        let t_desc_eos = b"eos token".to_vec();
        let precision = 4;
        let t_eos: Token = Token::new(t_sym_eos.clone(), t_desc_eos.clone(), precision);
        assert_eq!(TokenBalances::register_token(t_eos, 0, 0), Ok(()));

        let t_sym_eth = b"x-eth".to_vec();
        let t_desc_eth = b"eth token".to_vec();
        let precision = 4;
        let t_eth: Token = Token::new(t_sym_eth.clone(), t_desc_eth.clone(), precision);
        assert_eq!(TokenBalances::register_token(t_eth, 0, 0), Ok(()));

        let p1 = OrderPair::new(t_sym_eos.clone(), t_sym_eth.clone(), 0);

        // 增加交易对
        PendingOrders::add_pair(p1.clone()).unwrap();

        let a: u64 = 1; // accountid
        let b: u64 = 2;

        // 发放
        TokenBalances::issue(&a, &t_sym_eos.clone(), 1000).unwrap();
        TokenBalances::issue(&a, &t_sym_eth.clone(), 1000).unwrap();
        TokenBalances::issue(&b, &t_sym_eos.clone(), 1000).unwrap();
        TokenBalances::issue(&b, &t_sym_eth.clone(), 1000).unwrap();

        //挂买单
        let buy = OrderType::Buy;
        let a_order = PendingOrders::put_order(Some(a).into(), p1.clone(), buy, 100, 5);

        //挂卖单
        let buy = OrderType::Sell;
        let a_order = PendingOrders::put_order(Some(a).into(), p1.clone(), buy, 100, 7);

        assert_eq!(TokenBalances::free_token(&(a, t_sym_eos.clone())), 900);
        assert_eq!(TokenBalances::reserved_token(&(a, t_sym_eos.clone())), 100);
        assert_eq!(TokenBalances::free_token(&(a, t_sym_eth.clone())), 500);
        assert_eq!(TokenBalances::reserved_token(&(a, t_sym_eth.clone())), 500);

        //挂卖单
        let sell = OrderType::Sell;
        let b_order = PendingOrders::put_order(Some(b).into(), p1.clone(), sell, 50, 6);

        //挂卖单
        let sell = OrderType::Sell;
        let b_order = PendingOrders::put_order(Some(b).into(), p1.clone(), sell, 50, 7);

        assert_eq!(TokenBalances::free_token(&(b, t_sym_eos.clone())), 900);
        assert_eq!(TokenBalances::reserved_token(&(b, t_sym_eos.clone())), 100);
        assert_eq!(TokenBalances::free_token(&(b, t_sym_eth.clone())), 1000);
        assert_eq!(TokenBalances::reserved_token(&(b, t_sym_eth.clone())), 0);

        //取消挂单
        let cancel = PendingOrders::cancel_order(Some(b).into(), p1.clone(), 2);
        assert_eq!(Ok(()), cancel);

        assert_eq!(TokenBalances::free_token(&(b, t_sym_eos.clone())), 950);
        assert_eq!(TokenBalances::reserved_token(&(b, t_sym_eos.clone())), 50);
        assert_eq!(TokenBalances::free_token(&(b, t_sym_eth.clone())), 1000);
        assert_eq!(TokenBalances::reserved_token(&(b, t_sym_eth.clone())), 0);

        <MatchOrder as OnFinalise<u64>>::on_finalise(1);

        print_bid(p1.clone(), OrderType::Sell);
        print_bid(p1.clone(), OrderType::Buy);
    })
}

fn print_bid(pair: OrderPair, order_type: OrderType) {
    println!(
        "-------------------{:?} {:?} -----------------",
        pair, order_type
    );

    if let Some(header) = MatchOrder::bidlist_header_for((pair, order_type)) {
        let mut index = header.index();

        while let Some(mut node) = MatchOrder::bidlist_cache(&index) {
            println!("---{}---", index);
            println!("price:{:?}", node.data.price);
            println!("sum:{:?}", node.data.sum);
            println!("list:{:?}", node.data.list.len());
            for j in 0..node.data.list.len() {
                println!("      [{:?}] {:?}", j, node.data.list[j]);
                let bid_detail = MatchOrder::bid_of(node.data.list[j]).unwrap();
                println!("      id:{:?}", bid_detail.id);
                println!("      pair:{:?}", bid_detail.pair);
                println!("      order_type:{:?}", bid_detail.order_type);
                println!("      user:{:?}", bid_detail.user);
                println!("      order_index:{:?}", bid_detail.order_index);
                println!("      price:{:?}", bid_detail.price);
                println!("      amount:{:?}", bid_detail.amount);
                println!("      time:{:?}", bid_detail.time);
            }
            if let Some(next) = node.next() {
                index = next;
            } else {
                break;
            }
        }
    } else {
        println!("-------------------end -----------------");
    }
}
