// Copyright 2018-2019 Chainpool.

#![cfg(test)]

use super::mock::*;
use super::*;

use frame_support::{assert_noop, assert_ok};
use sp_std::collections::btree_map::BTreeMap;
use xpallet_assets::AssetType;

const EOS: AssetId = 8888;
const ETH: AssetId = 9999;

fn t_issue_pcx(to: AccountId, value: Balance) {
    let _ = Balances::deposit_creating(&to, value);
}

fn t_generic_issue(asset_id: AssetId, to: AccountId, value: Balance) {
    if asset_id == xpallet_protocol::PCX {
        t_issue_pcx(to, value);
    } else {
        assert_ok!(XAssets::issue(&asset_id, &to, value));
    }
}

fn t_generic_free_balance(who: AccountId, asset_id: AssetId) -> Balance {
    if asset_id == xpallet_protocol::PCX {
        Balances::free_balance(who)
    } else {
        XAssets::free_balance_of(&who, &asset_id)
    }
}

fn t_trading_pair_of(idx: TradingPairId) -> TradingPairProfile {
    XSpot::trading_pair_of(idx).unwrap()
}

fn t_put_order_buy(
    who: AccountId,
    pair_idx: TradingPairId,
    amount: Balance,
    price: Price,
) -> DispatchResult {
    XSpot::put_order(
        Origin::signed(who),
        pair_idx,
        OrderType::Limit,
        Side::Buy,
        amount,
        price,
    )
}

fn t_put_order_sell(
    who: AccountId,
    pair_idx: TradingPairId,
    amount: Balance,
    price: Price,
) -> DispatchResult {
    XSpot::put_order(
        Origin::signed(who),
        pair_idx,
        OrderType::Limit,
        Side::Sell,
        amount,
        price,
    )
}

fn t_cancel_order(who: AccountId, pair_id: TradingPairId, order_id: OrderId) -> DispatchResult {
    XSpot::cancel_order(Origin::signed(who), pair_id, order_id)
}

fn t_set_handicap(pair_idx: TradingPairId, highest_bid: Price, lowest_offer: Price) {
    assert_ok!(XSpot::set_handicap(
        Origin::root(),
        pair_idx,
        highest_bid,
        lowest_offer
    ));
}

fn t_set_price_fluctution(pair_idx: TradingPairId, new: PriceFluctuation) {
    assert_ok!(XSpot::set_price_fluctuation(Origin::root(), pair_idx, new));
}

fn t_convert_base_to_quote(amount: Balance, price: Price, pair: &TradingPairProfile) -> Balance {
    XSpot::convert_base_to_quote(amount, price, pair).unwrap()
}

#[test]
fn add_trading_pair_should_work() {
    ExtBuilder::default().build_and_execute(|| {
        let pair = CurrencyPair::new(EOS, ETH);
        assert_ok!(XSpot::add_trading_pair(pair.clone(), 2, 1, 100, true));
        assert_eq!(XSpot::trading_pair_count(), 3);
        assert_eq!(
            XSpot::get_trading_pair_by_currency_pair(&pair)
                .unwrap()
                .base(),
            pair.base
        );
    })
}

#[test]
fn update_trading_pair_should_work() {
    ExtBuilder::default().build_and_execute(|| {
        let pair = CurrencyPair::new(EOS, ETH);
        assert_ok!(XSpot::add_trading_pair(pair.clone(), 2, 1, 100, true));
        assert_eq!(t_trading_pair_of(2).tick_precision, 1);
        assert_eq!(t_trading_pair_of(2).online, true);

        assert_ok!(XSpot::update_trading_pair(2, 888, false));
        assert_eq!(t_trading_pair_of(2).tick_precision, 888);
        assert_eq!(t_trading_pair_of(2).online, false);
    })
}

#[test]
fn convert_base_to_quote_should_work() {
    ExtBuilder::default().build_and_execute(|| {
        let trading_pair = XSpot::trading_pair_of(0).unwrap();

        let amount = 1_000u128;
        let price = 1_210_000u128;

        assert_eq!(t_convert_base_to_quote(amount, price, &trading_pair), 1);
    })
}

#[test]
fn put_order_reserve_should_work() {
    ExtBuilder::default().build_and_execute(|| {
        let pair_id = 0;
        let who = 1;
        let trading_pair = XSpot::trading_pair_of(pair_id).unwrap();

        t_set_handicap(pair_id, 1_000_000, 1_100_000);

        // Reserve asset.
        t_generic_issue(trading_pair.quote(), who, 10);
        assert_eq!(t_generic_free_balance(who, trading_pair.quote()), 10);
        assert_ok!(t_put_order_buy(who, pair_id, 1000, 1_000_200));
        assert_eq!(t_generic_free_balance(who, trading_pair.quote()), 9);

        // Reserve native coin, 100 native coins should be reserved.
        t_issue_pcx(who, 1000);
        assert_ok!(t_put_order_sell(who, pair_id, 100, 1_210_000));
        assert_eq!(
            frame_system::Account::<Test>::get(&who).data,
            pallet_balances::AccountData {
                free: 900,
                reserved: 100,
                misc_frozen: 0,
                fee_frozen: 0
            }
        );
        assert_eq!(XSpot::native_reserves(&who), 100);

        // Reserve native coin, 200 more native coins should be reserved.
        assert_ok!(t_put_order_sell(who, pair_id, 200, 1_210_000));
        assert_eq!(
            frame_system::Account::<Test>::get(&who).data,
            pallet_balances::AccountData {
                free: 700,
                reserved: 300,
                misc_frozen: 0,
                fee_frozen: 0
            }
        );
        assert_eq!(XSpot::native_reserves(&who), 300);

        // 100 native coins should be unreserved.
        assert_ok!(t_cancel_order(who, pair_id, 2));
        assert_eq!(
            frame_system::Account::<Test>::get(&1).data,
            pallet_balances::AccountData {
                free: 900,
                reserved: 100,
                misc_frozen: 0,
                fee_frozen: 0
            }
        );
        assert_eq!(XSpot::native_reserves(&1), 100);
    })
}

#[test]
fn inject_order_should_work() {
    ExtBuilder::default().build_and_execute(|| {
        let trading_pair = XSpot::trading_pair_of(0).unwrap();
        t_set_handicap(0, 1_000_000, 1_100_000);
        assert_ok!(XAssets::issue(&trading_pair.quote(), &1, 10));

        assert_ok!(t_put_order_buy(1, 0, 1000, 1_000_100,));
        let order = XSpot::order_info_of(1, 0).unwrap();
        assert_eq!(order.submitter(), 1);
        assert_eq!(order.pair_id(), 0);
        assert_eq!(order.amount(), 1_000);
        assert_eq!(order.price(), 1_000_100);

        assert_ok!(t_put_order_buy(1, 0, 2000, 1_000_000,));
        let order = XSpot::order_info_of(1, 1).unwrap();
        assert_eq!(order.submitter(), 1);
        assert_eq!(order.pair_id(), 0);
        assert_eq!(order.amount(), 2_000);
        assert_eq!(order.price(), 1_000_000);
    })
}

#[test]
fn price_too_high_or_too_low_should_not_work() {
    ExtBuilder::default().build_and_execute(|| {
        let trading_pair = XSpot::trading_pair_of(0).unwrap();

        //  Buy: (~, 1_100_000 + 1_100_000 * 10%) = 1_210_000]
        // Sell: [1_000_000 * (1 - 10%) = 900_000, ~)
        t_set_handicap(0, 1_000_000, 1_100_000);

        assert_ok!(XAssets::issue(&trading_pair.quote(), &1, 10));

        // put order without matching
        assert_ok!(t_put_order_buy(1, 0, 1000, 1_000_200));

        t_set_handicap(0, 1_000_000, 1_100_000);

        assert_noop!(
            t_put_order_buy(1, 0, 1000, 2_210_000,),
            Error::<Test>::TooHighBidPrice
        );

        t_set_handicap(0, 1_000_000, 1_100_000);
        t_issue_pcx(1, 1000);

        assert_ok!(t_put_order_sell(1, 0, 1000, 1_210_000));

        assert_noop!(
            t_put_order_sell(1, 0, 1000, 890_000,),
            Error::<Test>::TooLowAskPrice
        );

        assert_eq!(XSpot::price_fluctuation_of(0), 100);
        t_set_price_fluctution(0, 200);
        assert_eq!(XSpot::price_fluctuation_of(0), 200);
    })
}

#[test]
fn update_handicap_should_work() {
    ExtBuilder::default().build_and_execute(|| {
        let trading_pair = XSpot::trading_pair_of(0).unwrap();

        t_generic_issue(trading_pair.quote(), 1, 10);
        t_issue_pcx(2, 2000);
        t_issue_pcx(3, 2000);

        assert_ok!(t_put_order_buy(1, 0, 1000, 1_210_000,));

        assert_eq!(XSpot::handicap_of(0).highest_bid, 1_210_000);

        assert_ok!(t_put_order_buy(1, 0, 1000, 1_310_000,));

        assert_eq!(XSpot::handicap_of(0).highest_bid, 1_310_000);

        assert_ok!(t_put_order_sell(2, 0, 500, 1_310_000 - 100));

        assert_eq!(XSpot::handicap_of(0).lowest_offer, 0);

        assert_ok!(t_put_order_sell(2, 0, 800, 1_3200_000));

        assert_eq!(XSpot::handicap_of(0).lowest_offer, 1_3200_000);
    })
}

#[test]
fn match_order_should_work() {
    ExtBuilder::default().build_and_execute(|| {
        let trading_pair = XSpot::trading_pair_of(0).unwrap();

        t_set_handicap(0, 1_000_000, 1_100_000);

        t_generic_issue(trading_pair.quote(), 1, 10);
        t_issue_pcx(2, 2000);
        t_issue_pcx(3, 2000);

        assert_ok!(t_put_order_buy(1, 0, 1000, 1_000_000));

        assert_ok!(t_put_order_buy(1, 0, 1000, 1_000_100));

        assert_ok!(t_put_order_sell(2, 0, 500, 1_000_100));

        assert_eq!(XSpot::order_info_of(2, 0), None);

        let order_1_1 = XSpot::order_info_of(1, 1).unwrap();

        assert_eq!(order_1_1.already_filled, 500);
        assert_eq!(order_1_1.status, OrderStatus::ParitialFill);
        assert_eq!(order_1_1.executed_indices, vec![0]);

        assert_ok!(t_put_order_sell(2, 0, 700, 1_000_100));

        assert_eq!(XSpot::order_info_of(1, 1), None);
        let order_2_1 = XSpot::order_info_of(2, 1).unwrap();
        assert_eq!(order_2_1.status, OrderStatus::ParitialFill);
        assert_eq!(order_2_1.already_filled, 500);
        assert_eq!(order_2_1.remaining, 200);
        assert_eq!(order_2_1.executed_indices, vec![1]);
    })
}

#[test]
fn cancel_order_should_work() {
    ExtBuilder::default().build_and_execute(|| {
        let trading_pair = XSpot::trading_pair_of(0).unwrap();

        t_set_handicap(0, 1_000_000, 1_100_000);

        assert_ok!(XAssets::issue(&trading_pair.quote(), &1, 10));
        t_issue_pcx(1, 2000);
        t_issue_pcx(2, 2000);
        t_issue_pcx(3, 2000);

        assert_ok!(t_put_order_buy(1, 0, 1000, 1_000_000));
        assert_ok!(t_put_order_buy(1, 0, 1000, 1_000_100));

        assert_ok!(t_put_order_sell(2, 0, 500, 1_000_200));

        assert_eq!(XSpot::quotations_of(0, 1_000_100), vec![(1, 1)]);
        assert_ok!(XSpot::cancel_order(Origin::signed(1), 0, 1));

        assert_eq!(XSpot::quotations_of(0, 1_200_000), vec![]);
        assert_eq!(XSpot::order_info_of(1, 1), None);
    })
}

#[test]
fn reap_orders_should_work() {
    ExtBuilder::default().build_and_execute(|| {
        let trading_pair = XSpot::trading_pair_of(0).unwrap();

        t_generic_issue(trading_pair.quote(), 1, 10);
        t_generic_issue(trading_pair.quote(), 2, 10);
        t_generic_issue(trading_pair.quote(), 3, 10);
        t_issue_pcx(2, 20000);
        t_issue_pcx(3, 20000);
        t_issue_pcx(4, 20000);

        assert_eq!(t_generic_free_balance(1, trading_pair.base()), 0);

        assert_ok!(t_put_order_buy(1, 0, 1000, 1_000_000));
        assert_ok!(t_put_order_buy(1, 0, 5000, 1_200_000));
        assert_ok!(t_put_order_buy(2, 0, 2000, 2_000_000));
        assert_ok!(t_put_order_buy(3, 0, 1000, 2_100_000));
        assert_ok!(t_put_order_buy(3, 0, 3000, 900_000));

        assert_ok!(t_put_order_sell(4, 0, 20_000, 2_100_000 - 100));

        assert_eq!(t_generic_free_balance(1, trading_pair.quote()), 3);
        assert_eq!(t_generic_free_balance(1, trading_pair.base()), 0);
        assert_eq!(t_generic_free_balance(2, trading_pair.quote()), 6);
        assert_eq!(t_generic_free_balance(3, trading_pair.quote()), 6);
        assert_eq!(XSpot::order_info_of(4, 0).unwrap().already_filled, 1_000);
    })
}

#[test]
fn refund_remaining_of_taker_order_should_work() {
    ExtBuilder::default().build_and_execute(|| {
        let trading_pair = XSpot::trading_pair_of(0).unwrap();

        let base = trading_pair.base();
        let quote = trading_pair.quote();

        t_issue_pcx(1, 1000000);
        t_issue_pcx(2, 237000000);

        t_generic_issue(trading_pair.quote(), 3, 489994);

        assert_ok!(t_put_order_sell(1, 0, 1000000, 2058800,));
        // 2058
        let btc_for_seller1 = t_convert_base_to_quote(1_000_000, 2058800, &trading_pair);

        assert_ok!(t_put_order_sell(2, 0, 237000000, 2058800,));
        // 487935
        let btc_for_seller2 = t_convert_base_to_quote(237000000, 2058800, &trading_pair);

        assert_ok!(t_put_order_buy(3, 0, 238000000, 2058800));

        // 489994
        let btc_reserved_for_buyer = t_convert_base_to_quote(238000000, 2058800, &trading_pair);

        // remaining is 1
        let remaining = btc_reserved_for_buyer - btc_for_seller1 - btc_for_seller2;

        let mut bmap = BTreeMap::new();
        bmap.insert(AssetType::Free, btc_for_seller1);
        assert_eq!(XAssets::asset_balance(1, quote.clone()), bmap);

        let mut bmap = BTreeMap::new();
        bmap.insert(AssetType::Free, btc_for_seller2);
        assert_eq!(XAssets::asset_balance(2, quote.clone()), bmap);

        let mut bmap = BTreeMap::new();
        bmap.insert(AssetType::Free, remaining);
        assert_eq!(XAssets::asset_balance(3, quote.clone()), bmap);

        assert_eq!(t_generic_free_balance(1, base.clone()), 0);
        assert_eq!(t_generic_free_balance(2, base.clone()), 0);
        assert_eq!(t_generic_free_balance(3, base.clone()), 238000000);
    })
}

#[test]
fn refund_remaining_of_maker_order_should_work() {
    ExtBuilder::default().build_and_execute(|| {
        let trading_pair = XSpot::trading_pair_of(0).unwrap();
        // PCX
        let base = trading_pair.base();
        // BTC
        let quote = trading_pair.quote();

        t_issue_pcx(1, 1_000_000);
        t_issue_pcx(2, 237000000);
        t_generic_issue(quote, 3, 489994);

        assert_ok!(t_put_order_buy(3, 0, 238_000_000, 2_058_800));

        // 489994
        let btc_reserved_for_buyer = t_convert_base_to_quote(238_000_000, 2_058_800, &trading_pair);

        assert_ok!(t_put_order_sell(1, 0, 1_000_000, 2058800));
        // 2058
        let btc_for_seller1 = t_convert_base_to_quote(1_000_000, 2_058_800, &trading_pair);

        assert_ok!(t_put_order_sell(2, 0, 237_000_000, 2_058_800));
        // 487935
        let btc_for_seller2 = t_convert_base_to_quote(237_000_000, 2_058_800, &trading_pair);

        // remaining is 1
        let remaining = btc_reserved_for_buyer - btc_for_seller1 - btc_for_seller2;

        let mut bmap = BTreeMap::new();
        bmap.insert(AssetType::Free, btc_for_seller1);
        assert_eq!(XAssets::asset_balance(1, quote), bmap);

        let mut bmap = BTreeMap::new();
        bmap.insert(AssetType::Free, btc_for_seller2);
        assert_eq!(XAssets::asset_balance(2, quote), bmap);

        let mut bmap = BTreeMap::new();
        bmap.insert(AssetType::Free, remaining);
        assert_eq!(XAssets::asset_balance(3, quote), bmap);

        assert_eq!(t_generic_free_balance(1, base), 0);
        assert_eq!(t_generic_free_balance(2, base), 0);
        assert_eq!(t_generic_free_balance(3, base), 238_000_000);
    })
}

#[test]
fn quotations_order_should_be_preserved_when_removing_orders_and_quotations() {
    ExtBuilder::default().build_and_execute(|| {
        let trading_pair = XSpot::trading_pair_of(0).unwrap();

        t_generic_issue(trading_pair.quote(), 1, 100);
        t_generic_issue(trading_pair.quote(), 2, 100);
        t_generic_issue(trading_pair.quote(), 3, 100);

        t_issue_pcx(2, 20000);
        t_issue_pcx(3, 20000);
        t_issue_pcx(4, 20000);
        t_issue_pcx(5, 20000);
        t_issue_pcx(6, 20000);

        assert_eq!(t_generic_free_balance(1, trading_pair.base()), 0);

        assert_ok!(t_put_order_buy(1, 0, 1_000, 1_000_000));
        assert_ok!(t_put_order_buy(2, 0, 5_000, 1_100_000));
        assert_ok!(t_put_order_buy(3, 0, 2_000, 2_000_000));

        assert_ok!(t_put_order_sell(4, 0, 4_000, 2_000_000));
        assert_ok!(t_put_order_sell(2, 0, 2_000, 2_000_000));
        assert_ok!(t_put_order_sell(5, 0, 500, 2_000_000));
        assert_ok!(t_put_order_sell(6, 0, 600, 2_000_000));

        assert_ok!(t_put_order_buy(3, 0, 3_500, 2_000_000));

        assert_eq!(XSpot::quotations_of(0, 2_000_000), [(2, 1), (5, 0), (6, 0)]);
    })
}
