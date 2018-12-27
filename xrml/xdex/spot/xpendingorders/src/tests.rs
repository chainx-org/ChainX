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

use mock::{new_test_ext, Assets, Balances, Origin, Pendingorders, Test};

#[test]
fn test_fee() {
    with_externalities(&mut new_test_ext(0, 3, 3, 0, true, 10), || {
        Pendingorders::set_order_fee(20);

        assert_eq!(Pendingorders::order_fee(), 20);
    })
}

#[test]
fn test_pair() {
    with_externalities(&mut new_test_ext(0, 3, 3, 0, true, 10), || {
        let a: u64 = 1; // accountid

        let p1 = OrderPair {
            first: b"x-btc".to_vec(),
            second: b"x-eth".to_vec(),
        };
        let p2 = OrderPair {
            first: b"x-eos".to_vec(),
            second: b"x-eth".to_vec(),
        };
        let mut p_list = Vec::new();
        p_list.push(p1.clone());
        p_list.push(p2.clone());

        // add_pair
        Pendingorders::add_pair(p1.clone()).unwrap();
        Pendingorders::add_pair(p2.clone()).unwrap();

        let r_list = Pendingorders::pair_list();
        assert_eq!(r_list, p_list);

        assert_eq!(Pendingorders::is_valid_pair(&p1), Ok(()));
        assert_eq!(Pendingorders::is_valid_pair(&p2), Ok(()));
    })
}

#[test]
fn test_order() {
    with_externalities(&mut new_test_ext(0, 3, 3, 0, true, 10), || {
        let t_sym_eos = b"BTC".to_vec();
        let t_desc_eos = b"BTC".to_vec();
        let precision = 3;
        let t_eos: Asset =
            Asset::new(t_sym_eos.clone(), Chain::BTC, precision, t_desc_eos.clone()).unwrap();
        assert_eq!(Assets::add_asset(t_eos, 0, 0), Ok(()));

        let t_sym_eth = b"ETH".to_vec();
        let t_desc_eth = b"ETH".to_vec();
        let precision = 3;
        let t_eth: Asset =
            Asset::new(t_sym_eth.clone(), Chain::ETH, precision, t_desc_eth.clone()).unwrap();
        assert_eq!(Assets::add_asset(t_eth, 0, 0), Ok(()));

        let p1 = OrderPair {
            first: t_sym_eos.clone(),
            second: t_sym_eth.clone(),
        };

        let mut p_list = Vec::new();
        p_list.push(p1.clone());

        // add_pair
        Pendingorders::add_pair(p1.clone()).unwrap();

        let r_list = Pendingorders::pair_list();
        assert_eq!(r_list, p_list);

        assert_eq!(Pendingorders::is_valid_pair(&p1), Ok(()));

        let a: u64 = 3; // accountid

        // 发放
        Assets::issue(&a, &t_sym_eos.clone(), 500).unwrap();
        assert_eq!(Assets::total_balance_of(&a, &t_sym_eos.clone()), 500);
        assert_eq!(Assets::total_balance(&t_sym_eos.clone()), 500);

        Assets::issue(&a, &t_sym_eth.clone(), 500).unwrap();
        assert_eq!(Assets::total_balance_of(&a, &t_sym_eth.clone()), 500);
        assert_eq!(Assets::total_balance(&t_sym_eth.clone()), 500);

        //挂买单
        let buy = OrderType::Buy;
        let order = Pendingorders::put_order(
            Some(a).into(),
            p1.clone(),
            buy,
            100,
            20,
            b"imtoken".to_vec(),
        );
        assert_eq!(order, Ok(()));
        // 10000-10
        //assert_eq!(Balances::free_balance(&a), 9990);

        //500-200
        assert_eq!(Assets::free_balance(&(a, t_sym_eth.clone())), 498);
        assert_eq!(
            Assets::reserved_balance(&(a, t_sym_eth.clone(), assets::ReservedType::DexSpot)),
            2
        );

        //挂卖单
        let sell = OrderType::Sell;
        let order = Pendingorders::put_order(
            Some(a).into(),
            p1.clone(),
            sell,
            100,
            1000,
            b"imtoken".to_vec(),
        );
        assert_eq!(order, Ok(()));

        // 10000-10-10
        //assert_eq!(Balances::free_balance(&a), 9980);

        //500-100
        assert_eq!(Assets::free_balance(&(a, t_sym_eos.clone())), 400);
        assert_eq!(
            Assets::reserved_balance(&(a, t_sym_eos.clone(), assets::ReservedType::DexSpot)),
            100
        );

        let last_order_index_of_eos_eth =
            Pendingorders::last_order_index_of((a.clone(), p1.clone())).unwrap();
        assert_eq!(2, last_order_index_of_eos_eth);

        let order_2 =
            Pendingorders::order_of((a.clone(), p1.clone(), last_order_index_of_eos_eth)).unwrap();
        let order_1 =
            Pendingorders::order_of((a.clone(), p1.clone(), (last_order_index_of_eos_eth - 1)))
                .unwrap();

        print_order(order_1.clone());
        print_order(order_2.clone());

        //取消挂单
        let cancel = Pendingorders::cancel_order(
            Some(a).into(),
            p1.clone(),
            last_order_index_of_eos_eth - 1,
        );
        assert_eq!(Ok(()), cancel);
        //500-200+200
        assert_eq!(Assets::free_balance(&(a, t_sym_eth.clone())), 500);
        assert_eq!(
            Assets::reserved_balance(&(a, t_sym_eth.clone(), assets::ReservedType::DexSpot)),
            0
        );

        let cancel_order_1 =
            Pendingorders::order_of((a.clone(), p1.clone(), last_order_index_of_eos_eth - 1))
                .unwrap();
        assert_eq!(OrderStatus::Cancel, cancel_order_1.status());

        print_order(cancel_order_1.clone());
        print_order(order_2.clone());

        let list = Pendingorders::order_list(&a, &p1.clone());
        assert_eq!(2, list.len());
        println!("-------------------------------------------order list -----------------------------------------");
        for o in list {
            print_order(o);
        }
    })
}

fn print_order(
    order: Order<
        OrderPair,
        <tests::Test as system::Trait>::AccountId,
        <tests::Test as Trait>::Amount,
        <tests::Test as Trait>::Price,
        <tests::Test as system::Trait>::BlockNumber,
    >,
) {
    println!(
        "-------------------order {} -----------------",
        order.index()
    );
    println!(
        "pair={}/{}",
        str::from_utf8(&order.pair().first).unwrap(),
        str::from_utf8(&order.pair().second).unwrap()
    );
    println!("index={}", order.index());
    println!("class={:?}", order.class());
    println!("user={}", order.user());
    println!("amount={}", order.amount());
    println!("channel={:?}", order.channel());
    println!("hasfill_amount={}", order.hasfill_amount());
    println!("price={}", order.price());
    println!("create_time={}", order.create_time());
    println!("lastupdate_time={}", order.lastupdate_time());
    println!("status={:?}", order.status());
    println!("reserve_last={:?}", order.reserve_last());
    let fill_index = order.fill_index();

    println!("--fill_index--");
    for index in &fill_index {
        println!("{}", index);
    }
}

fn print_order_list(account: <tests::Test as system::Trait>::AccountId, pair: OrderPair) {
    let list = Pendingorders::order_list(&account.clone(), &pair.clone());
    println!("-------------------------------------------order {} list -----------------------------------------", account);
    for o in list {
        print_order(o);
    }
}

#[test]
fn test_fill_no_fee() {
    with_externalities(&mut new_test_ext(0, 3, 3, 0, true, 10), || {
        let t_sym_eos = b"BTC".to_vec();
        let t_desc_eos = b"BTC".to_vec();
        let precision = 4;
        let t_eos: Asset =
            Asset::new(t_sym_eos.clone(), Chain::BTC, precision, t_desc_eos.clone()).unwrap();
        assert_eq!(Assets::add_asset(t_eos, 0, 0), Ok(()));

        let t_sym_eth = b"ETH".to_vec();
        let t_desc_eth = b"ETH".to_vec();
        let precision = 4;
        let t_eth: Asset =
            Asset::new(t_sym_eth.clone(), Chain::ETH, precision, t_desc_eth.clone()).unwrap();
        assert_eq!(Assets::add_asset(t_eth, 0, 0), Ok(()));

        let p1 = OrderPair {
            first: t_sym_eos.clone(),
            second: t_sym_eth.clone(),
        };

        // 增加交易对
        Pendingorders::add_pair(p1.clone()).unwrap();

        let a: u64 = 3; // accountid
        let b: u64 = 4;

        // 发放
        Assets::issue(&a, &t_sym_eos.clone(), 10000000).unwrap();
        Assets::issue(&a, &t_sym_eth.clone(), 10000000).unwrap();
        Assets::issue(&b, &t_sym_eos.clone(), 10000000).unwrap();
        Assets::issue(&b, &t_sym_eth.clone(), 10000000).unwrap();

        //挂买单
        let buy = OrderType::Buy;
        let a_order = Pendingorders::put_order(
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
        let b_order = Pendingorders::put_order(
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

        print_order_list(a, p1.clone());
        print_order_list(b, p1.clone());

        let r_fill =
            Pendingorders::fill_order(p1.clone(), a.clone(), b.clone(), 1, 1, 5, 500000, 0, 0);
        assert_eq!(Ok(()), r_fill);

        //1000+250
        assert_eq!(Assets::free_balance(&(a, t_sym_eos.clone())), 10500000);
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

        assert_eq!(1, Pendingorders::last_fill_index_of_pair(&p1.clone()));
        // let last_fill = Pendingorders::fill_of((p1.clone(), 1)).unwrap();

        // print_fill(last_fill.clone());

        print_order_list(a, p1.clone());
        print_order_list(b, p1.clone());

        let last_order_index_of_eos_eth_alice =
            Pendingorders::last_order_index_of((a.clone(), p1.clone())).unwrap();
        let a_order_1 =
            Pendingorders::order_of((a.clone(), p1.clone(), last_order_index_of_eos_eth_alice))
                .unwrap();
        assert_eq!(500000, a_order_1.hasfill_amount());
        assert_eq!(OrderStatus::FillPart, a_order_1.status);

        let last_order_index_of_eos_eth_bob =
            Pendingorders::last_order_index_of((b.clone(), p1.clone())).unwrap();
        let b_order_1 =
            Pendingorders::order_of((b.clone(), p1.clone(), last_order_index_of_eos_eth_bob))
                .unwrap();
        assert_eq!(500000, b_order_1.hasfill_amount());
        assert_eq!(OrderStatus::FillAll, b_order_1.status);
    })
}

#[test]
fn test_fill_fee() {
    with_externalities(&mut new_test_ext(0, 3, 3, 0, true, 10), || {
        let t_sym_eos = b"BTC".to_vec();
        let t_desc_eos = b"BTC".to_vec();
        let precision = 3;
        let t_eos: Asset =
            Asset::new(t_sym_eos.clone(), Chain::BTC, precision, t_desc_eos.clone()).unwrap();
        assert_eq!(Assets::add_asset(t_eos, 0, 0), Ok(()));

        let t_sym_eth = b"ETH".to_vec();
        let t_desc_eth = b"ETH".to_vec();
        let precision = 3;
        let t_eth: Asset =
            Asset::new(t_sym_eth.clone(), Chain::ETH, precision, t_desc_eth.clone()).unwrap();
        assert_eq!(Assets::add_asset(t_eth, 0, 0), Ok(()));

        let p1 = OrderPair {
            first: t_sym_eos.clone(),
            second: t_sym_eth.clone(),
        };

        // 增加交易对
        Pendingorders::add_pair(p1.clone()).unwrap();

        let a: u64 = 3; // accountid
        let b: u64 = 4;

        // 发放
        Assets::issue(&a, &t_sym_eos.clone(), 1001).unwrap();
        Assets::issue(&a, &t_sym_eth.clone(), 1001).unwrap();
        Assets::issue(&b, &t_sym_eos.clone(), 1001).unwrap();
        Assets::issue(&b, &t_sym_eth.clone(), 1001).unwrap();

        //挂买单
        let buy = OrderType::Buy;
        let a_order = Pendingorders::put_order(
            Some(a).into(),
            p1.clone(),
            buy,
            1001,
            5,
            b"imtoken".to_vec(),
        );
        assert_eq!(Assets::free_balance(&(a, t_sym_eos.clone())), 1001);
        assert_eq!(
            Assets::reserved_balance(&(a, t_sym_eos.clone(), assets::ReservedType::DexSpot)),
            0
        );
        assert_eq!(Assets::free_balance(&(a, t_sym_eth.clone())), 996);
        assert_eq!(
            Assets::reserved_balance(&(a, t_sym_eth.clone(), assets::ReservedType::DexSpot)),
            5
        );

        //挂卖单
        let sell = OrderType::Sell;
        let b_order = Pendingorders::put_order(
            Some(b).into(),
            p1.clone(),
            sell,
            500,
            5,
            b"imtoken".to_vec(),
        );
        assert_eq!(b_order, Ok(()));
        assert_eq!(Assets::free_balance(&(b, t_sym_eos.clone())), 501);
        assert_eq!(
            Assets::reserved_balance(&(b, t_sym_eos.clone(), assets::ReservedType::DexSpot)),
            500
        );
        assert_eq!(Assets::free_balance(&(b, t_sym_eth.clone())), 1001);
        assert_eq!(
            Assets::reserved_balance(&(b, t_sym_eth.clone(), assets::ReservedType::DexSpot)),
            0
        );

        print_order_list(a, p1.clone());
        print_order_list(b, p1.clone());

        let r_fill =
            Pendingorders::fill_order(p1.clone(), a.clone(), b.clone(), 1, 1, 5, 500, 5, 5);
        assert_eq!(Ok(()), r_fill);

        //1000+250
        assert_eq!(Assets::free_balance(&(a, t_sym_eos.clone())), 1496);
        assert_eq!(
            Assets::reserved_balance(&(a, t_sym_eos.clone(), assets::ReservedType::DexSpot)),
            0
        );
        //1000-500
        assert_eq!(Assets::free_balance(&(a, t_sym_eth.clone())), 996);
        //500-250
        assert_eq!(
            Assets::reserved_balance(&(a, t_sym_eth.clone(), assets::ReservedType::DexSpot)),
            3
        );

        //1000-50
        assert_eq!(Assets::free_balance(&(b, t_sym_eos.clone())), 501);
        assert_eq!(
            Assets::reserved_balance(&(b, t_sym_eos.clone(), assets::ReservedType::DexSpot)),
            0
        );
        assert_eq!(Assets::free_balance(&(b, t_sym_eth.clone())), 1003);
        assert_eq!(
            Assets::reserved_balance(&(b, t_sym_eth.clone(), assets::ReservedType::DexSpot)),
            0
        );

        assert_eq!(
            Assets::free_balance(&(xsystem::Module::<Test>::burn_account(), t_sym_eth.clone())),
            0
        );
        assert_eq!(
            Assets::free_balance(&(xsystem::Module::<Test>::burn_account(), t_sym_eos.clone())),
            5
        );
        Pendingorders::clear_command_and_put_fee_buy_order();
        // assert_eq!(Assets::free_balance(&(Test::burn_account, t_sym_eth.clone())), 0);
        // assert_eq!(Assets::free_balance(&(Test::burn_account, t_sym_eos.clone())), 0);
        // assert_eq!(Assets::reserved_balance(&(Test::burn_account, t_sym_eth.clone(), assets::ReservedType::DexSpot)), 25);
        // assert_eq!(Assets::reserved_balance(&(Test::burn_account, t_sym_eos.clone(), assets::ReservedType::DexSpot)), 5);

        assert_eq!(1, Pendingorders::last_fill_index_of_pair(&p1.clone()));
        // let last_fill = Pendingorders::fill_of((p1.clone(), 1)).unwrap();

        // print_fill(last_fill.clone());

        print_order_list(a, p1.clone());
        print_order_list(b, p1.clone());

        let last_order_index_of_eos_eth_alice =
            Pendingorders::last_order_index_of((a.clone(), p1.clone())).unwrap();
        let a_order_1 =
            Pendingorders::order_of((a.clone(), p1.clone(), last_order_index_of_eos_eth_alice))
                .unwrap();
        assert_eq!(500, a_order_1.hasfill_amount());
        assert_eq!(OrderStatus::FillPart, a_order_1.status);

        let last_order_index_of_eos_eth_bob =
            Pendingorders::last_order_index_of((b.clone(), p1.clone())).unwrap();
        let b_order_1 =
            Pendingorders::order_of((b.clone(), p1.clone(), last_order_index_of_eos_eth_bob))
                .unwrap();
        assert_eq!(500, b_order_1.hasfill_amount());
        assert_eq!(OrderStatus::FillAll, b_order_1.status);

        let cancel = Pendingorders::cancel_order(Some(a).into(), p1.clone(), 1);

        let cancel_order_1 = Pendingorders::order_of((a.clone(), p1.clone(), 1)).unwrap();
        assert_eq!(OrderStatus::FillPartAndCancel, cancel_order_1.status());
        //1000-500
        assert_eq!(Assets::free_balance(&(a, t_sym_eth.clone())), 999);
        //500-250
        assert_eq!(
            Assets::reserved_balance(&(a, t_sym_eth.clone(), assets::ReservedType::DexSpot)),
            0
        );
        print_order_list(a, p1.clone());
        print_order_list(b, p1.clone());
    })
}

fn print_fill(
    fill: Fill<
        OrderPair,
        <tests::Test as system::Trait>::AccountId,
        <tests::Test as Trait>::Amount,
        <tests::Test as Trait>::Price,
        <tests::Test as system::Trait>::BlockNumber,
    >,
) {
    println!("-------------------fill {} -----------------", fill.index());
    println!(
        "pair={}/{}",
        str::from_utf8(&fill.pair().first).unwrap(),
        str::from_utf8(&fill.pair().second).unwrap()
    );
    println!("index={}", fill.index());
    println!("maker_user={:?}", fill.maker_user());
    println!("taker_user={}", fill.taker_user());
    println!("maker_user_order_index={}", fill.maker_user_order_index());
    println!("taker_user_order_index={}", fill.taker_user_order_index());
    println!("price={}", fill.price());
    println!("amount={}", fill.amount());
    println!("maker_fee={:?}", fill.maker_fee());
    println!("taker_fee={:?}", fill.taker_fee());
    println!("time={:?}", fill.time());
}
