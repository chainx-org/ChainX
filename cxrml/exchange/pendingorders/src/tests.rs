// Copyright 2018 Chainpool.

use substrate_primitives::{Blake2Hasher, H256};

use runtime_io;
use runtime_io::with_externalities;
use runtime_primitives::testing::{Digest, DigestItem, Header};
use runtime_primitives::traits::BlakeTwo256;
use runtime_primitives::BuildStorage;

use super::*;
use std::str;
use tokenbalances::{DescString, SymbolString, Token};

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

impl cxsystem::Trait for Test {}

impl associations::Trait for Test {
    type OnCalcFee = cxsupport::Module<Test>;
    type Event = ();
}
impl cxsupport::Trait for Test {}

// define tokenbalances module type
pub type TokenBalance = u128;

impl tokenbalances::Trait for Test {
    const CHAINX_SYMBOL: SymbolString = b"pcx";
    const CHAINX_TOKEN_DESC: DescString = b"this is pcx for mock";
    type TokenBalance = TokenBalance;
    type Event = ();
    type OnMoveToken = ();
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
            balances: vec![(3, 10000), (4, 10000)],
            transaction_base_fee: 0,
            transaction_byte_fee: 0,
            existential_deposit: 500,
            transfer_fee: 0,
            creation_fee: 0,
            reclaim_rebate: 0,
        }
        .build_storage()
        .unwrap(),
    );

    r.extend(
        GenesisConfig::<Test> {
            order_fee: 10,
            pair_list: vec![],
            max_command_id: 0,
            average_price_len: 10000,
        }
        .build_storage()
        .unwrap(),
    );
    r.into()
}

impl Trait for Test {
    type Event = ();
    type Amount = u128;
    type Price = u128;
}

type PendingOrders = Module<Test>;
type TokenBalances = tokenbalances::Module<Test>;
type Balances = balances::Module<Test>;

#[test]
fn test_fee() {
    with_externalities(&mut new_test_ext(), || {
        PendingOrders::set_order_fee(20);

        assert_eq!(PendingOrders::order_fee(), 20);
    })
}

#[test]
fn test_pair() {
    with_externalities(&mut new_test_ext(), || {
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
        PendingOrders::add_pair(p1.clone()).unwrap();
        PendingOrders::add_pair(p2.clone()).unwrap();

        let r_list = PendingOrders::pair_list();
        assert_eq!(r_list, p_list);

        assert_eq!(PendingOrders::is_valid_pair(&p1), Ok(()));
        assert_eq!(PendingOrders::is_valid_pair(&p2), Ok(()));
    })
}

#[test]
fn test_order() {
    with_externalities(&mut new_test_ext(), || {
        let t_sym_eos = b"x-eos".to_vec();
        let t_desc_eos = b"eos token".to_vec();
        let precision = 3;
        let t_eos: Token = Token::new(t_sym_eos.clone(), t_desc_eos.clone(), precision);
        assert_eq!(TokenBalances::register_token(t_eos, 0, 0), Ok(()));

        let t_sym_eth = b"x-eth".to_vec();
        let t_desc_eth = b"eth token".to_vec();
        let precision = 3;
        let t_eth: Token = Token::new(t_sym_eth.clone(), t_desc_eth.clone(), precision);
        assert_eq!(TokenBalances::register_token(t_eth, 0, 0), Ok(()));

        let p1 = OrderPair {
            first: t_sym_eos.clone(),
            second: t_sym_eth.clone(),
        };

        let mut p_list = Vec::new();
        p_list.push(p1.clone());

        // add_pair
        PendingOrders::add_pair(p1.clone()).unwrap();

        let r_list = PendingOrders::pair_list();
        assert_eq!(r_list, p_list);

        assert_eq!(PendingOrders::is_valid_pair(&p1), Ok(()));

        let a: u64 = 3; // accountid

        // 发放
        TokenBalances::issue(&a, &t_sym_eos.clone(), 500).unwrap();
        assert_eq!(TokenBalances::total_token_of(&a, &t_sym_eos.clone()), 500);
        assert_eq!(TokenBalances::total_token(&t_sym_eos.clone()), 500);

        TokenBalances::issue(&a, &t_sym_eth.clone(), 500).unwrap();
        assert_eq!(TokenBalances::total_token_of(&a, &t_sym_eth.clone()), 500);
        assert_eq!(TokenBalances::total_token(&t_sym_eth.clone()), 500);

        //挂买单
        let buy = OrderType::Buy;
        let order =
            PendingOrders::put_order(Some(a).into(), p1.clone(), buy, 100, 20, b"imtoken".to_vec());
        assert_eq!(order, Ok(()));
        // 10000-10
        assert_eq!(Balances::free_balance(&a), 9990);

        //500-200
        assert_eq!(TokenBalances::free_token(&(a, t_sym_eth.clone())), 498);
        assert_eq!(
            TokenBalances::reserved_token(&(a, t_sym_eth.clone(), ReservedType::Exchange)),
            2
        );

        //挂卖单
        let sell = OrderType::Sell;
        let order = PendingOrders::put_order(
            Some(a).into(),
            p1.clone(),
            sell,
            100,
            1000,
            b"imtoken".to_vec(),
        );
        assert_eq!(order, Ok(()));

        // 10000-10-10
        assert_eq!(Balances::free_balance(&a), 9980);

        //500-100
        assert_eq!(TokenBalances::free_token(&(a, t_sym_eos.clone())), 400);
        assert_eq!(
            TokenBalances::reserved_token(&(a, t_sym_eos.clone(), ReservedType::Exchange)),
            100
        );

        let last_order_index_of_eos_eth =
            PendingOrders::last_order_index_of((a.clone(), p1.clone())).unwrap();
        assert_eq!(2, last_order_index_of_eos_eth);

        let order_2 =
            PendingOrders::order_of((a.clone(), p1.clone(), last_order_index_of_eos_eth)).unwrap();
        let order_1 =
            PendingOrders::order_of((a.clone(), p1.clone(), (last_order_index_of_eos_eth - 1)))
                .unwrap();

        print_order(order_1.clone());
        print_order(order_2.clone());

        //取消挂单
        let cancel = PendingOrders::cancel_order(
            Some(a).into(),
            p1.clone(),
            last_order_index_of_eos_eth - 1,
        );
        assert_eq!(Ok(()), cancel);
        //500-200+200
        assert_eq!(TokenBalances::free_token(&(a, t_sym_eth.clone())), 500);
        assert_eq!(
            TokenBalances::reserved_token(&(a, t_sym_eth.clone(), ReservedType::Exchange)),
            0
        );

        let cancel_order_1 =
            PendingOrders::order_of((a.clone(), p1.clone(), last_order_index_of_eos_eth - 1))
                .unwrap();
        assert_eq!(OrderStatus::Cancel, cancel_order_1.status());

        print_order(cancel_order_1.clone());
        print_order(order_2.clone());

        let list = PendingOrders::order_list(&a, &p1.clone());
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
    let list = PendingOrders::order_list(&account.clone(), &pair.clone());
    println!("-------------------------------------------order {} list -----------------------------------------", account);
    for o in list {
        print_order(o);
    }
}

#[test]
fn test_fill_no_fee() {
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

        let p1 = OrderPair {
            first: t_sym_eos.clone(),
            second: t_sym_eth.clone(),
        };

        // 增加交易对
        PendingOrders::add_pair(p1.clone()).unwrap();

        let a: u64 = 3; // accountid
        let b: u64 = 4;

        // 发放
        TokenBalances::issue(&a, &t_sym_eos.clone(), 10000000).unwrap();
        TokenBalances::issue(&a, &t_sym_eth.clone(), 10000000).unwrap();
        TokenBalances::issue(&b, &t_sym_eos.clone(), 10000000).unwrap();
        TokenBalances::issue(&b, &t_sym_eth.clone(), 10000000).unwrap();

        //挂买单
        let buy = OrderType::Buy;
        let a_order =
            PendingOrders::put_order(Some(a).into(), p1.clone(), buy, 1000000, 5, b"imtoken".to_vec());
        assert_eq!(TokenBalances::free_token(&(a, t_sym_eos.clone())), 10000000);
        assert_eq!(
            TokenBalances::reserved_token(&(a, t_sym_eos.clone(), ReservedType::Exchange)),
            0
        );
        assert_eq!(TokenBalances::free_token(&(a, t_sym_eth.clone())), 9999500);
        assert_eq!(
            TokenBalances::reserved_token(&(a, t_sym_eth.clone(), ReservedType::Exchange)),
            500
        );

        //挂卖单
        let sell = OrderType::Sell;
        let b_order =
            PendingOrders::put_order(Some(b).into(), p1.clone(), sell, 500000, 5, b"imtoken".to_vec());
        assert_eq!(b_order, Ok(()));
        assert_eq!(TokenBalances::free_token(&(b, t_sym_eos.clone())), 9500000);
        assert_eq!(
            TokenBalances::reserved_token(&(b, t_sym_eos.clone(), ReservedType::Exchange)),
            500000
        );
        assert_eq!(TokenBalances::free_token(&(b, t_sym_eth.clone())), 10000000);
        assert_eq!(
            TokenBalances::reserved_token(&(b, t_sym_eth.clone(), ReservedType::Exchange)),
            0
        );

        print_order_list(a, p1.clone());
        print_order_list(b, p1.clone());

        let r_fill = PendingOrders::fill_order(p1.clone(), a.clone(), b.clone(), 1, 1, 5, 500000, 0, 0);
        assert_eq!(Ok(()), r_fill);

        //1000+250
        assert_eq!(TokenBalances::free_token(&(a, t_sym_eos.clone())), 10500000);
        assert_eq!(
            TokenBalances::reserved_token(&(a, t_sym_eos.clone(), ReservedType::Exchange)),
            0
        );
        //1000-500
        assert_eq!(TokenBalances::free_token(&(a, t_sym_eth.clone())), 9999500);
        //500-250
        assert_eq!(
            TokenBalances::reserved_token(&(a, t_sym_eth.clone(), ReservedType::Exchange)),
            250
        );

        //1000-50
        assert_eq!(TokenBalances::free_token(&(b, t_sym_eos.clone())), 9500000);
        assert_eq!(
            TokenBalances::reserved_token(&(b, t_sym_eos.clone(), ReservedType::Exchange)),
            0
        );
        assert_eq!(TokenBalances::free_token(&(b, t_sym_eth.clone())), 10000250);
        assert_eq!(
            TokenBalances::reserved_token(&(b, t_sym_eth.clone(), ReservedType::Exchange)),
            0
        );

        assert_eq!(1, PendingOrders::last_fill_index_of_pair(&p1.clone()));
        let last_fill = PendingOrders::fill_of((p1.clone(), 1)).unwrap();

        print_fill(last_fill.clone());

        print_order_list(a, p1.clone());
        print_order_list(b, p1.clone());

        let last_order_index_of_eos_eth_alice =
            PendingOrders::last_order_index_of((a.clone(), p1.clone())).unwrap();
        let a_order_1 =
            PendingOrders::order_of((a.clone(), p1.clone(), last_order_index_of_eos_eth_alice))
                .unwrap();
        assert_eq!(500000, a_order_1.hasfill_amount());
        assert_eq!(OrderStatus::FillPart, a_order_1.status);

        let last_order_index_of_eos_eth_bob =
            PendingOrders::last_order_index_of((b.clone(), p1.clone())).unwrap();
        let b_order_1 =
            PendingOrders::order_of((b.clone(), p1.clone(), last_order_index_of_eos_eth_bob))
                .unwrap();
        assert_eq!(500000, b_order_1.hasfill_amount());
        assert_eq!(OrderStatus::FillAll, b_order_1.status);
    })
}

#[test]
fn test_fill_fee() {
    with_externalities(&mut new_test_ext(), || {
        let t_sym_eos = b"x-eos".to_vec();
        let t_desc_eos = b"eos token".to_vec();
        let precision = 3;
        let t_eos: Token = Token::new(t_sym_eos.clone(), t_desc_eos.clone(), precision);
        assert_eq!(TokenBalances::register_token(t_eos, 0, 0), Ok(()));

        let t_sym_eth = b"x-eth".to_vec();
        let t_desc_eth = b"eth token".to_vec();
        let precision = 3;
        let t_eth: Token = Token::new(t_sym_eth.clone(), t_desc_eth.clone(), precision);
        assert_eq!(TokenBalances::register_token(t_eth, 0, 0), Ok(()));

        let p1 = OrderPair {
            first: t_sym_eos.clone(),
            second: t_sym_eth.clone(),
        };

        // 增加交易对
        PendingOrders::add_pair(p1.clone()).unwrap();

        let a: u64 = 3; // accountid
        let b: u64 = 4;

        // 发放
        TokenBalances::issue(&a, &t_sym_eos.clone(), 1001).unwrap();
        TokenBalances::issue(&a, &t_sym_eth.clone(), 1001).unwrap();
        TokenBalances::issue(&b, &t_sym_eos.clone(), 1001).unwrap();
        TokenBalances::issue(&b, &t_sym_eth.clone(), 1001).unwrap();

        //挂买单
        let buy = OrderType::Buy;
        let a_order =
            PendingOrders::put_order(Some(a).into(), p1.clone(), buy, 1001, 5, b"imtoken".to_vec());
        assert_eq!(TokenBalances::free_token(&(a, t_sym_eos.clone())), 1001);
        assert_eq!(
            TokenBalances::reserved_token(&(a, t_sym_eos.clone(), ReservedType::Exchange)),
            0
        );
        assert_eq!(TokenBalances::free_token(&(a, t_sym_eth.clone())), 996);
        assert_eq!(
            TokenBalances::reserved_token(&(a, t_sym_eth.clone(), ReservedType::Exchange)),
            5
        );

        //挂卖单
        let sell = OrderType::Sell;
        let b_order =
            PendingOrders::put_order(Some(b).into(), p1.clone(), sell, 500, 5, b"imtoken".to_vec());
        assert_eq!(b_order, Ok(()));
        assert_eq!(TokenBalances::free_token(&(b, t_sym_eos.clone())), 501);
        assert_eq!(
            TokenBalances::reserved_token(&(b, t_sym_eos.clone(), ReservedType::Exchange)),
            500
        );
        assert_eq!(TokenBalances::free_token(&(b, t_sym_eth.clone())), 1001);
        assert_eq!(
            TokenBalances::reserved_token(&(b, t_sym_eth.clone(), ReservedType::Exchange)),
            0
        );

        print_order_list(a, p1.clone());
        print_order_list(b, p1.clone());

        let r_fill = PendingOrders::fill_order(p1.clone(), a.clone(), b.clone(), 1, 1, 5, 500, 5, 5);
        assert_eq!(Ok(()), r_fill);

        //1000+250
        assert_eq!(TokenBalances::free_token(&(a, t_sym_eos.clone())), 1496);
        assert_eq!(
            TokenBalances::reserved_token(&(a, t_sym_eos.clone(), ReservedType::Exchange)),
            0
        );
        //1000-500
        assert_eq!(TokenBalances::free_token(&(a, t_sym_eth.clone())), 996);
        //500-250
        assert_eq!(
            TokenBalances::reserved_token(&(a, t_sym_eth.clone(), ReservedType::Exchange)),
            3
        );



        //1000-50
        assert_eq!(TokenBalances::free_token(&(b, t_sym_eos.clone())), 501);
        assert_eq!(
            TokenBalances::reserved_token(&(b, t_sym_eos.clone(), ReservedType::Exchange)),
            0
        );
        assert_eq!(TokenBalances::free_token(&(b, t_sym_eth.clone())), 1003);
        assert_eq!(
            TokenBalances::reserved_token(&(b, t_sym_eth.clone(), ReservedType::Exchange)),
            0
        );

        assert_eq!(
            TokenBalances::free_token(&(
                cxsystem::Module::<Test>::fee_buy_account(),
                t_sym_eth.clone()
            )),
            0
        );
        assert_eq!(
            TokenBalances::free_token(&(
                cxsystem::Module::<Test>::fee_buy_account(),
                t_sym_eos.clone()
            )),
            5
        );
        PendingOrders::clear_command_and_put_fee_buy_order();
        // assert_eq!(TokenBalances::free_token(&(Test::FEE_BUY_ACCOUNT, t_sym_eth.clone())), 0);
        // assert_eq!(TokenBalances::free_token(&(Test::FEE_BUY_ACCOUNT, t_sym_eos.clone())), 0);
        // assert_eq!(TokenBalances::reserved_token(&(Test::FEE_BUY_ACCOUNT, t_sym_eth.clone(), ReservedType::Exchange)), 25);
        // assert_eq!(TokenBalances::reserved_token(&(Test::FEE_BUY_ACCOUNT, t_sym_eos.clone(), ReservedType::Exchange)), 5);

        assert_eq!(1, PendingOrders::last_fill_index_of_pair(&p1.clone()));
        let last_fill = PendingOrders::fill_of((p1.clone(), 1)).unwrap();

        print_fill(last_fill.clone());

        print_order_list(a, p1.clone());
        print_order_list(b, p1.clone());

        let last_order_index_of_eos_eth_alice =
            PendingOrders::last_order_index_of((a.clone(), p1.clone())).unwrap();
        let a_order_1 =
            PendingOrders::order_of((a.clone(), p1.clone(), last_order_index_of_eos_eth_alice))
                .unwrap();
        assert_eq!(500, a_order_1.hasfill_amount());
        assert_eq!(OrderStatus::FillPart, a_order_1.status);

        let last_order_index_of_eos_eth_bob =
            PendingOrders::last_order_index_of((b.clone(), p1.clone())).unwrap();
        let b_order_1 =
            PendingOrders::order_of((b.clone(), p1.clone(), last_order_index_of_eos_eth_bob))
                .unwrap();
        assert_eq!(500, b_order_1.hasfill_amount());
        assert_eq!(OrderStatus::FillAll, b_order_1.status);

        let cancel = PendingOrders::cancel_order(
            Some(a).into(),
            p1.clone(),
            1,
        );

        let cancel_order_1 =
            PendingOrders::order_of((a.clone(), p1.clone(), 1))
                .unwrap();
        assert_eq!(OrderStatus::FillPartAndCancel, cancel_order_1.status());
        //1000-500
        assert_eq!(TokenBalances::free_token(&(a, t_sym_eth.clone())), 999);
        //500-250
        assert_eq!(
            TokenBalances::reserved_token(&(a, t_sym_eth.clone(), ReservedType::Exchange)),
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
