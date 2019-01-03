// Copyright 2018 Chainpool.

use substrate_primitives::{Blake2Hasher, H256};

use runtime_io;
use runtime_io::with_externalities;
use runtime_primitives::testing::{Digest, DigestItem, Header};
use runtime_primitives::traits::BlakeTwo256;
use runtime_primitives::BuildStorage;

use super::*;
use assets::assetdef::{Asset, Chain, ChainT, Token};
use std::str;

use mock::{new_test_ext, Assets, Balances, MatchOrder, Origin, PendingOrders, Test};

#[test]
fn test_fee() {
    with_externalities(&mut new_test_ext(0, 3, 3, 0, true, 10), || {
        MatchOrder::set_match_fee(20);

        assert_eq!(MatchOrder::match_fee(), 20);
    })
}

#[test]
fn test_match_part() {
    with_externalities(&mut new_test_ext(0, 3, 3, 0, true, 10), || {
        let t_sym_eos = b"BTC".to_vec();
        let t_desc_eos = b"BTC".to_vec();
        let precision = 4;
        let t_eos: Asset = Asset::new(
            t_sym_eos.clone(),
            Chain::Bitcoin,
            precision,
            t_desc_eos.clone(),
        )
        .unwrap();
        assert_eq!(Assets::add_asset(t_eos, 0, 0), Ok(()));

        let t_sym_eth = b"ETH".to_vec();
        let t_desc_eth = b"ETH".to_vec();
        let precision = 4;
        let t_eth: Asset = Asset::new(
            t_sym_eth.clone(),
            Chain::Ethereum,
            precision,
            t_desc_eth.clone(),
        )
        .unwrap();
        assert_eq!(Assets::add_asset(t_eth, 0, 0), Ok(()));

        let p1 = OrderPair {
            first: t_sym_eos.clone(),
            second: t_sym_eth.clone(),
        };

        // 增加交易对
        PendingOrders::add_pair(p1.clone()).unwrap();

        let a: u64 = 1; // accountid
        let b: u64 = 2;

        // 发放
        Assets::issue(&a, &t_sym_eos.clone(), 10000000).unwrap();
        Assets::issue(&a, &t_sym_eth.clone(), 10000000).unwrap();
        Assets::issue(&b, &t_sym_eos.clone(), 10000000).unwrap();
        Assets::issue(&b, &t_sym_eth.clone(), 10000000).unwrap();

        //挂买单
        let buy = OrderType::Buy;
        let a_order = PendingOrders::put_order(
            Some(a).into(),
            p1.clone(),
            buy,
            1000000,
            5,
            b"imtoken".to_vec(),
        );
        assert_eq!(Assets::free_balance(&(a, t_sym_eos.clone())), 10000000);
        assert_eq!(
            Assets::reserved_balance(&(a, t_sym_eos.clone(), assets::ReservedType::DexSpot)),
            0
        );
        assert_eq!(Assets::free_balance(&(a, t_sym_eth.clone())), 9999500);
        assert_eq!(
            Assets::reserved_balance(&(a, t_sym_eth.clone(), assets::ReservedType::DexSpot)),
            500
        );

        //挂卖单
        let sell = OrderType::Sell;
        let b_order = PendingOrders::put_order(
            Some(b).into(),
            p1.clone(),
            sell,
            500000,
            5,
            b"imtoken".to_vec(),
        );
        assert_eq!(b_order, Ok(()));
        assert_eq!(Assets::free_balance(&(b, t_sym_eos.clone())), 9500000);
        assert_eq!(
            Assets::reserved_balance(&(b, t_sym_eos.clone(), assets::ReservedType::DexSpot)),
            500000
        );
        assert_eq!(Assets::free_balance(&(b, t_sym_eth.clone())), 10000000);
        assert_eq!(
            Assets::reserved_balance(&(b, t_sym_eth.clone(), assets::ReservedType::DexSpot)),
            0
        );

        print_bid(p1.clone(), OrderType::Sell);
        print_bid(p1.clone(), OrderType::Buy);

        //<MatchOrder as OnFinalise<u64>>::on_finalise(1);
        MatchOrder::on_finalise(1);

        assert_eq!(Assets::free_balance(&(a, t_sym_eos.clone())), 10499750);
        assert_eq!(
            Assets::reserved_balance(&(a, t_sym_eos.clone(), assets::ReservedType::DexSpot)),
            0
        );
        //1000-500
        assert_eq!(Assets::free_balance(&(a, t_sym_eth.clone())), 9999500);
        //500-250
        assert_eq!(
            Assets::reserved_balance(&(a, t_sym_eth.clone(), assets::ReservedType::DexSpot)),
            250
        );

        //1000-50
        assert_eq!(Assets::free_balance(&(b, t_sym_eos.clone())), 9500000);
        assert_eq!(
            Assets::reserved_balance(&(b, t_sym_eos.clone(), assets::ReservedType::DexSpot)),
            0
        );
        assert_eq!(Assets::free_balance(&(b, t_sym_eth.clone())), 10000250);
        assert_eq!(
            Assets::reserved_balance(&(b, t_sym_eth.clone(), assets::ReservedType::DexSpot)),
            0
        );

        print_bid(p1.clone(), OrderType::Sell);
        print_bid(p1.clone(), OrderType::Buy);
    })
}

#[test]
fn test_match_all() {
    with_externalities(&mut new_test_ext(0, 3, 3, 0, true, 10), || {
        let t_sym_eos = b"BTC".to_vec();
        let t_desc_eos = b"BTC".to_vec();
        let precision = 4;
        let t_eos: Asset = Asset::new(
            t_sym_eos.clone(),
            Chain::Bitcoin,
            precision,
            t_desc_eos.clone(),
        )
        .unwrap();
        assert_eq!(Assets::add_asset(t_eos, 0, 0), Ok(()));

        let t_sym_eth = b"ETH".to_vec();
        let t_desc_eth = b"ETH".to_vec();
        let precision = 4;
        let t_eth: Asset = Asset::new(
            t_sym_eth.clone(),
            Chain::Ethereum,
            precision,
            t_desc_eth.clone(),
        )
        .unwrap();
        assert_eq!(Assets::add_asset(t_eth, 0, 0), Ok(()));

        let p1 = OrderPair {
            first: t_sym_eos.clone(),
            second: t_sym_eth.clone(),
        };

        // 增加交易对
        PendingOrders::add_pair(p1.clone()).unwrap();

        let a: u64 = 3; // accountid
        let b: u64 = 4;

        // 发放
        Assets::issue(&a, &t_sym_eos.clone(), 10000000).unwrap();
        Assets::issue(&a, &t_sym_eth.clone(), 10000000).unwrap();
        Assets::issue(&b, &t_sym_eos.clone(), 10000000).unwrap();
        Assets::issue(&b, &t_sym_eth.clone(), 10000000).unwrap();

        //挂买单
        let buy = OrderType::Buy;
        let a_order = PendingOrders::put_order(
            Some(a).into(),
            p1.clone(),
            buy,
            1000000,
            5,
            b"imtoken".to_vec(),
        );
        assert_eq!(Assets::free_balance(&(a, t_sym_eos.clone())), 10000000);
        assert_eq!(
            Assets::reserved_balance(&(a, t_sym_eos.clone(), assets::ReservedType::DexSpot)),
            0
        );
        assert_eq!(Assets::free_balance(&(a, t_sym_eth.clone())), 9999500);
        assert_eq!(
            Assets::reserved_balance(&(a, t_sym_eth.clone(), assets::ReservedType::DexSpot)),
            500
        );

        //挂卖单
        let sell = OrderType::Sell;
        let b_order = PendingOrders::put_order(
            Some(b).into(),
            p1.clone(),
            sell,
            1000000,
            5,
            b"imtoken".to_vec(),
        );
        assert_eq!(b_order, Ok(()));
        assert_eq!(Assets::free_balance(&(b, t_sym_eos.clone())), 9000000);
        assert_eq!(
            Assets::reserved_balance(&(b, t_sym_eos.clone(), assets::ReservedType::DexSpot)),
            1000000
        );
        assert_eq!(Assets::free_balance(&(b, t_sym_eth.clone())), 10000000);
        assert_eq!(
            Assets::reserved_balance(&(b, t_sym_eth.clone(), assets::ReservedType::DexSpot)),
            0
        );

        print_bid(p1.clone(), OrderType::Sell);
        print_bid(p1.clone(), OrderType::Buy);

        MatchOrder::on_finalise(1);

        assert_eq!(Assets::free_balance(&(a, t_sym_eos.clone())), 10999500);
        assert_eq!(
            Assets::reserved_balance(&(a, t_sym_eos.clone(), assets::ReservedType::DexSpot)),
            0
        );
        //1000-500
        assert_eq!(Assets::free_balance(&(a, t_sym_eth.clone())), 9999500);
        //500-250
        assert_eq!(
            Assets::reserved_balance(&(a, t_sym_eth.clone(), assets::ReservedType::DexSpot)),
            0
        );

        //1000-50
        assert_eq!(Assets::free_balance(&(b, t_sym_eos.clone())), 9000000);
        assert_eq!(
            Assets::reserved_balance(&(b, t_sym_eos.clone(), assets::ReservedType::DexSpot)),
            0
        );
        assert_eq!(Assets::free_balance(&(b, t_sym_eth.clone())), 10000500);
        assert_eq!(
            Assets::reserved_balance(&(b, t_sym_eth.clone(), assets::ReservedType::DexSpot)),
            0
        );

        print_bid(p1.clone(), OrderType::Sell);
        print_bid(p1.clone(), OrderType::Buy);
    })
}

#[test]
fn test_match_no() {
    with_externalities(&mut new_test_ext(0, 3, 3, 0, true, 10), || {
        let t_sym_eos = b"BTC".to_vec();
        let t_desc_eos = b"BTC".to_vec();
        let precision = 4;
        let t_eos: Asset = Asset::new(
            t_sym_eos.clone(),
            Chain::Bitcoin,
            precision,
            t_desc_eos.clone(),
        )
        .unwrap();
        assert_eq!(Assets::add_asset(t_eos, 0, 0), Ok(()));

        let t_sym_eth = b"ETH".to_vec();
        let t_desc_eth = b"ETH".to_vec();
        let precision = 4;
        let t_eth: Asset = Asset::new(
            t_sym_eth.clone(),
            Chain::Ethereum,
            precision,
            t_desc_eth.clone(),
        )
        .unwrap();
        assert_eq!(Assets::add_asset(t_eth, 0, 0), Ok(()));

        let p1 = OrderPair {
            first: t_sym_eos.clone(),
            second: t_sym_eth.clone(),
        };

        // 增加交易对
        PendingOrders::add_pair(p1.clone()).unwrap();

        let a: u64 = 3; // accountid
        let b: u64 = 4;

        // 发放
        Assets::issue(&a, &t_sym_eos.clone(), 10000000).unwrap();
        Assets::issue(&a, &t_sym_eth.clone(), 10000000).unwrap();
        Assets::issue(&b, &t_sym_eos.clone(), 10000000).unwrap();
        Assets::issue(&b, &t_sym_eth.clone(), 10000000).unwrap();

        //挂买单
        let buy = OrderType::Buy;
        let a_order = PendingOrders::put_order(
            Some(a).into(),
            p1.clone(),
            buy,
            1000000,
            5,
            b"imtoken".to_vec(),
        );

        //挂卖单
        let buy = OrderType::Sell;
        let a_order = PendingOrders::put_order(
            Some(a).into(),
            p1.clone(),
            buy,
            1000000,
            7,
            b"imtoken".to_vec(),
        );

        assert_eq!(Assets::free_balance(&(a, t_sym_eos.clone())), 9000000);
        assert_eq!(
            Assets::reserved_balance(&(a, t_sym_eos.clone(), assets::ReservedType::DexSpot)),
            1000000
        );
        assert_eq!(Assets::free_balance(&(a, t_sym_eth.clone())), 9999500);
        assert_eq!(
            Assets::reserved_balance(&(a, t_sym_eth.clone(), assets::ReservedType::DexSpot)),
            500
        );

        //挂卖单
        let sell = OrderType::Sell;
        let b_order = PendingOrders::put_order(
            Some(b).into(),
            p1.clone(),
            sell,
            500000,
            6,
            b"imtoken".to_vec(),
        );

        //挂卖单
        let sell = OrderType::Sell;
        let b_order = PendingOrders::put_order(
            Some(b).into(),
            p1.clone(),
            sell,
            500000,
            7,
            b"imtoken".to_vec(),
        );

        assert_eq!(Assets::free_balance(&(b, t_sym_eos.clone())), 9000000);
        assert_eq!(
            Assets::reserved_balance(&(b, t_sym_eos.clone(), assets::ReservedType::DexSpot)),
            1000000
        );
        assert_eq!(Assets::free_balance(&(b, t_sym_eth.clone())), 10000000);
        assert_eq!(
            Assets::reserved_balance(&(b, t_sym_eth.clone(), assets::ReservedType::DexSpot)),
            0
        );

        //取消挂单
        let cancel = PendingOrders::cancel_order(Some(b).into(), p1.clone(), 2);
        assert_eq!(Ok(()), cancel);

        assert_eq!(Assets::free_balance(&(b, t_sym_eos.clone())), 9500000);
        assert_eq!(
            Assets::reserved_balance(&(b, t_sym_eos.clone(), assets::ReservedType::DexSpot)),
            500000
        );
        assert_eq!(Assets::free_balance(&(b, t_sym_eth.clone())), 10000000);
        assert_eq!(
            Assets::reserved_balance(&(b, t_sym_eth.clone(), assets::ReservedType::DexSpot)),
            0
        );

        MatchOrder::on_finalise(1);

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
