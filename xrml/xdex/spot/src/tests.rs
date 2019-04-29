// Copyright 2018-2019 Chainpool.

#![cfg(test)]

use super::mock::*;
use super::*;

use runtime_io::with_externalities;
use support::{assert_noop, assert_ok};

#[test]
fn add_trading_pair_should_work() {
    with_externalities(&mut new_test_ext(), || {
        let pair = CurrencyPair::new(b"EOS".to_vec(), b"ETH".to_vec());
        assert_ok!(XSpot::add_trading_pair(pair.clone(), 2, 1, 100, true));
        assert_eq!(XSpot::trading_pair_count(), 3);
        assert_eq!(
            XSpot::get_trading_pair_by_currency_pair(&pair)
                .unwrap()
                .base(),
            pair.base()
        );
    })
}

#[test]
fn update_trading_pair_should_work() {
    with_externalities(&mut new_test_ext(), || {
        let pair = CurrencyPair::new(b"EOS".to_vec(), b"ETH".to_vec());
        assert_ok!(XSpot::add_trading_pair(pair.clone(), 2, 1, 100, true));
        assert_eq!(XSpot::trading_pair_of(2).unwrap().tick_precision, 1);
        assert_eq!(XSpot::trading_pair_of(2).unwrap().online, true);

        assert_ok!(XSpot::update_trading_pair(2, 888, false));
        assert_eq!(XSpot::trading_pair_of(2).unwrap().tick_precision, 888);
        assert_eq!(XSpot::trading_pair_of(2).unwrap().online, false);
    })
}

#[test]
fn convert_base_to_quote_should_work() {
    with_externalities(&mut new_test_ext(), || {
        let trading_pair = XSpot::trading_pair_of(0).unwrap();

        let amount = 1_000u64;
        let price = 1_210_000u64;

        assert_eq!(
            XSpot::convert_base_to_quote(amount, price, &trading_pair).unwrap(),
            1
        );
    })
}

#[test]
fn put_order_reserve_should_work() {
    with_externalities(&mut new_test_ext(), || {
        let trading_pair = XSpot::trading_pair_of(0).unwrap();
        assert_ok!(XSpot::set_handicap(0, 1_000_000, 1_100_000));
        assert_ok!(XAssets::issue(&trading_pair.quote(), &1, 10));
        assert_eq!(XAssets::free_balance_of(&1, &trading_pair.quote()), 10);
        assert_ok!(XSpot::put_order(
            Origin::signed(1),
            0,
            OrderType::Limit,
            Side::Buy,
            1000,
            1_210_000,
        ));
        assert_eq!(XAssets::free_balance_of(&1, &trading_pair.quote()), 9);
    })
}

#[test]
fn inject_order_should_work() {
    with_externalities(&mut new_test_ext(), || {
        let trading_pair = XSpot::trading_pair_of(0).unwrap();
        assert_ok!(XSpot::set_handicap(0, 1_000_000, 1_100_000));
        assert_ok!(XAssets::issue(&trading_pair.quote(), &1, 10));

        assert_ok!(XSpot::put_order(
            Origin::signed(1),
            0,
            OrderType::Limit,
            Side::Buy,
            1000,
            1_210_000,
        ));
        let order = XSpot::order_info_of(&(1, 0)).unwrap();
        assert_eq!(order.submitter(), 1);
        assert_eq!(order.pair_index(), 0);
        assert_eq!(order.amount(), 1_000);
        assert_eq!(order.price(), 1_210_000);

        assert_ok!(XSpot::put_order(
            Origin::signed(1),
            0,
            OrderType::Limit,
            Side::Buy,
            2000,
            1_000_000,
        ));
        let order = XSpot::order_info_of(&(1, 1)).unwrap();
        assert_eq!(order.submitter(), 1);
        assert_eq!(order.pair_index(), 0);
        assert_eq!(order.amount(), 2_000);
        assert_eq!(order.price(), 1_000_000);
    })
}

#[test]
fn price_too_high_or_too_low_should_not_work() {
    with_externalities(&mut new_test_ext(), || {
        let trading_pair = XSpot::trading_pair_of(0).unwrap();

        //  Buy: (~, 1_100_000 + 1_100_000 * 10%) = 1_210_000]
        // Sell: [1_000_000 * (1 - 10%) = 900_000, ~)
        assert_ok!(XSpot::set_handicap(0, 1_000_000, 1_100_000));

        assert_ok!(XAssets::issue(&trading_pair.quote(), &1, 10));

        // put order without matching
        assert_ok!(XSpot::put_order(
            Origin::signed(1),
            0,
            OrderType::Limit,
            Side::Buy,
            1000,
            1_210_000,
        ));

        assert_ok!(XSpot::set_handicap(0, 1_000_000, 1_100_000));

        assert_noop!(
            XSpot::put_order(
                Origin::signed(1),
                0,
                OrderType::Limit,
                Side::Buy,
                1000,
                2_210_000,
            ),
            "The bid price can not higher than the PriceVolatility of current lowest_offer."
        );

        assert_ok!(XSpot::set_handicap(0, 1_000_000, 1_100_000));
        assert_ok!(XAssets::pcx_issue(&1, 1000));

        assert_ok!(XSpot::put_order(
            Origin::signed(1),
            0,
            OrderType::Limit,
            Side::Sell,
            1000,
            1_210_000,
        ));

        assert_noop!(
            XSpot::put_order(
                Origin::signed(1),
                0,
                OrderType::Limit,
                Side::Sell,
                1000,
                890_000,
            ),
            "The ask price can not lower than the PriceVolatility of current highest_bid."
        );
    })
}

#[test]
fn update_handicap_should_work() {
    with_externalities(&mut new_test_ext(), || {
        let trading_pair = XSpot::trading_pair_of(0).unwrap();

        assert_ok!(XAssets::issue(&trading_pair.quote(), &1, 10));
        assert_ok!(XAssets::pcx_issue(&2, 2000));
        assert_ok!(XAssets::pcx_issue(&3, 2000));

        assert_ok!(XSpot::put_order(
            Origin::signed(1),
            0,
            OrderType::Limit,
            Side::Buy,
            1000,
            1_210_000,
        ));

        assert_eq!(XSpot::handicap_of(0).highest_bid, 1_210_000);

        assert_ok!(XSpot::put_order(
            Origin::signed(1),
            0,
            OrderType::Limit,
            Side::Buy,
            1000,
            1_310_000,
        ));

        assert_eq!(XSpot::handicap_of(0).highest_bid, 1_310_000);

        assert_ok!(XSpot::put_order(
            Origin::signed(2),
            0,
            OrderType::Limit,
            Side::Sell,
            500,
            1_200_000
        ));

        assert_eq!(XSpot::handicap_of(0).lowest_offer, 0);

        assert_ok!(XSpot::put_order(
            Origin::signed(2),
            0,
            OrderType::Limit,
            Side::Sell,
            800,
            1_3200_000
        ));

        assert_eq!(XSpot::handicap_of(0).lowest_offer, 1_3200_000);
    })
}

#[test]
fn match_order_should_work() {
    with_externalities(&mut new_test_ext(), || {
        let trading_pair = XSpot::trading_pair_of(0).unwrap();

        assert_ok!(XSpot::set_handicap(0, 1_000_000, 1_100_000));

        assert_ok!(XAssets::issue(&trading_pair.quote(), &1, 10));
        assert_ok!(XAssets::pcx_issue(&2, 2000));
        assert_ok!(XAssets::pcx_issue(&3, 2000));

        assert_ok!(XSpot::put_order(
            Origin::signed(1),
            0,
            OrderType::Limit,
            Side::Buy,
            1000,
            1_000_000,
        ));

        assert_ok!(XSpot::put_order(
            Origin::signed(1),
            0,
            OrderType::Limit,
            Side::Buy,
            1000,
            1_200_000,
        ));

        assert_ok!(XSpot::put_order(
            Origin::signed(2),
            0,
            OrderType::Limit,
            Side::Sell,
            500,
            1_200_000
        ));

        assert_eq!(XSpot::order_info_of((2, 0)), None);

        let order_1_1 = XSpot::order_info_of((1, 1)).unwrap();

        assert_eq!(order_1_1.already_filled, 500);
        assert_eq!(order_1_1.status, OrderStatus::ParitialFill);
        assert_eq!(order_1_1.executed_indices, vec![0]);

        assert_ok!(XSpot::put_order(
            Origin::signed(2),
            0,
            OrderType::Limit,
            Side::Sell,
            700,
            1_200_000
        ));

        assert_eq!(XSpot::order_info_of((1, 1)), None);
        let order_2_1 = XSpot::order_info_of((2, 1)).unwrap();
        assert_eq!(order_2_1.status, OrderStatus::ParitialFill);
        assert_eq!(order_2_1.already_filled, 500);
        assert_eq!(order_2_1.remaining, 200);
        assert_eq!(order_2_1.executed_indices, vec![1]);
    })
}

#[test]
fn cancel_order_should_work() {
    with_externalities(&mut new_test_ext(), || {
        let trading_pair = XSpot::trading_pair_of(0).unwrap();

        assert_ok!(XSpot::set_handicap(0, 1_000_000, 1_100_000));

        assert_ok!(XAssets::issue(&trading_pair.quote(), &1, 10));
        assert_ok!(XAssets::pcx_issue(&2, 2000));
        assert_ok!(XAssets::pcx_issue(&3, 2000));

        assert_ok!(XSpot::put_order(
            Origin::signed(1),
            0,
            OrderType::Limit,
            Side::Buy,
            1000,
            1_000_000,
        ));

        assert_ok!(XSpot::put_order(
            Origin::signed(1),
            0,
            OrderType::Limit,
            Side::Buy,
            1000,
            1_200_000,
        ));

        assert_ok!(XSpot::put_order(
            Origin::signed(2),
            0,
            OrderType::Limit,
            Side::Sell,
            500,
            1_200_000
        ));

        assert_eq!(XSpot::quotations_of((0, 1_200_000)), vec![(1, 1)]);
        assert_ok!(XSpot::cancel_order(Origin::signed(1), 0, 1));

        assert_eq!(XSpot::quotations_of((0, 1_200_000)), vec![]);
        assert_eq!(XSpot::order_info_of((1, 1)), None);
    })
}

#[test]
fn reap_orders_should_work() {
    with_externalities(&mut new_test_ext(), || {
        let trading_pair = XSpot::trading_pair_of(0).unwrap();

        assert_ok!(XAssets::issue(&trading_pair.quote(), &1, 10));
        assert_ok!(XAssets::issue(&trading_pair.quote(), &2, 10));
        assert_ok!(XAssets::issue(&trading_pair.quote(), &3, 10));
        assert_ok!(XAssets::pcx_issue(&2, 20000));
        assert_ok!(XAssets::pcx_issue(&3, 20000));
        assert_ok!(XAssets::pcx_issue(&4, 20000));

        assert_eq!(XAssets::free_balance_of(&1, &trading_pair.base()), 0);

        assert_ok!(XSpot::put_order(
            Origin::signed(1),
            0,
            OrderType::Limit,
            Side::Buy,
            1000,
            1_000_000,
        ));

        assert_ok!(XSpot::put_order(
            Origin::signed(1),
            0,
            OrderType::Limit,
            Side::Buy,
            5000,
            1_200_000,
        ));

        assert_ok!(XSpot::put_order(
            Origin::signed(2),
            0,
            OrderType::Limit,
            Side::Buy,
            2000,
            2_000_000
        ));

        assert_ok!(XSpot::put_order(
            Origin::signed(3),
            0,
            OrderType::Limit,
            Side::Buy,
            1000,
            2_100_000
        ));

        assert_ok!(XSpot::put_order(
            Origin::signed(3),
            0,
            OrderType::Limit,
            Side::Buy,
            3000,
            900_000
        ));

        assert_ok!(XSpot::put_order(
            Origin::signed(4),
            0,
            OrderType::Limit,
            Side::Sell,
            20_000,
            900_000
        ));

        assert_eq!(XAssets::free_balance_of(&1, &trading_pair.quote()), 3);
        assert_eq!(XAssets::free_balance_of(&1, &trading_pair.base()), 6000);
        assert_eq!(XAssets::free_balance_of(&2, &trading_pair.quote()), 6);
        assert_eq!(XAssets::free_balance_of(&3, &trading_pair.quote()), 6);
        assert_eq!(XSpot::order_info_of((4, 0)).unwrap().already_filled, 12_000);
    })
}
