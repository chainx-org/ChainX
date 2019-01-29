// Copyright 2018 Chainpool.
use super::*;
use mock::*;
use runtime_io::with_externalities;
use std::str;
use xassets::assetdef::{Asset, Chain, ChainT, Token};
use xassets::AssetType::*;

#[test]
fn test_pair() {
    with_externalities(&mut new_test_ext(0, 3, 3, 0, true, 10), || {
        let a: u64 = 1; // accountid
        let first: Token = b"EOS".to_vec();
        let second: Token = b"ETH".to_vec();
        Spot::add_pair(first.clone(), second.clone(), 2, 1, 100, true).unwrap();
        assert_eq!(Spot::pair_len(), 2);

        let pair = Spot::get_pair_by(&first, &second).unwrap();
        assert_eq!(pair.first, first);
    })
}

#[test]
fn test_order() {
    with_externalities(&mut new_test_ext(0, 3, 3, 0, true, 10), || {
        let a: u64 = 3;
        let b: u64 = 4;
        let BTC = b"BTC".to_vec();
        let PCX = b"PCX".to_vec();

        assert_eq!(Assets::asset_balance(&a, &BTC, Free), 1_000_000_000);
        assert_eq!(Assets::asset_balance(&a, &BTC, ReservedDexSpot), 0);
        assert_eq!(Assets::asset_balance(&a, &PCX, Free), 1_000_000_000);
        assert_eq!(Assets::asset_balance(&a, &PCX, ReservedDexSpot), 0);

        assert_eq!(Assets::asset_balance(&b, &BTC, Free), 1_000_000_000);
        assert_eq!(Assets::asset_balance(&b, &BTC, ReservedDexSpot), 0);
        assert_eq!(Assets::asset_balance(&b, &PCX, Free), 1_000_000_000);
        assert_eq!(Assets::asset_balance(&b, &PCX, ReservedDexSpot), 0);

        //a 第一笔挂买单 1000*100000
        assert_eq!(
            Spot::put_order(
                Some(a).into(),
                0,
                OrderType::Limit,
                OrderDirection::Buy,
                1000,
                100000
            ),
            Ok(())
        );
        assert_eq!(
            Assets::asset_balance(&a, &BTC, Free),
            1_000_000_000 - (1000 * 100000)
        );
        assert_eq!(
            Assets::asset_balance(&a, &BTC, ReservedDexSpot),
            1000 * 100000
        );
        assert_eq!(Assets::asset_balance(&a, &PCX, Free), 1_000_000_000);
        assert_eq!(Assets::asset_balance(&a, &PCX, ReservedDexSpot), 0);

        //a 第二笔挂买单 1000*50
        assert_eq!(
            Spot::put_order(
                Some(a).into(),
                0,
                OrderType::Limit,
                OrderDirection::Buy,
                1000,
                50
            ),
            Ok(())
        );
        assert_eq!(
            Assets::asset_balance(&a, &BTC, Free),
            1_000_000_000 - (1000 * 100000) - (1000 * 50)
        );
        assert_eq!(
            Assets::asset_balance(&a, &BTC, ReservedDexSpot),
            1000 * 100000 + 1000 * 50
        );
        assert_eq!(Assets::asset_balance(&a, &PCX, Free), 1_000_000_000);
        assert_eq!(Assets::asset_balance(&a, &PCX, ReservedDexSpot), 0);

        assert_eq!(Spot::account_orders_len(&a).unwrap(), 2);

        //b 第一笔挂卖单 1000*200000
        assert_eq!(
            Spot::put_order(
                Some(b).into(),
                0,
                OrderType::Limit,
                OrderDirection::Sell,
                1000,
                200000
            ),
            Ok(())
        );
        assert_eq!(Assets::asset_balance(&b, &BTC, Free), 1_000_000_000);
        assert_eq!(Assets::asset_balance(&b, &BTC, ReservedDexSpot), 0);
        assert_eq!(Assets::asset_balance(&b, &PCX, Free), 1_000_000_000 - 1000);
        assert_eq!(Assets::asset_balance(&b, &PCX, ReservedDexSpot), 1000);

        let mut handicapMap = Spot::handicap_map(0).unwrap();

        assert_eq!(handicapMap.buy, 100000);
        assert_eq!(handicapMap.sell, 200000);

        //b 第二笔挂卖单 500*100000
        assert_eq!(
            Spot::put_order(
                Some(b).into(),
                0,
                OrderType::Limit,
                OrderDirection::Sell,
                500,
                100000
            ),
            Ok(())
        );

        assert_eq!(
            Assets::asset_balance(&a, &BTC, Free),
            1_000_000_000 - (1000 * 100000) - (1000 * 50)
        );
        assert_eq!(
            Assets::asset_balance(&a, &BTC, ReservedDexSpot),
            1000 * 100000 + 1000 * 50 - 500 * 100000
        );
        assert_eq!(Assets::asset_balance(&a, &PCX, Free), 1_000_000_000 + 500);
        assert_eq!(Assets::asset_balance(&a, &PCX, ReservedDexSpot), 0);

        assert_eq!(
            Assets::asset_balance(&b, &BTC, Free),
            1_000_000_000 + 500 * 100000
        );
        assert_eq!(Assets::asset_balance(&b, &BTC, ReservedDexSpot), 0);
        assert_eq!(
            Assets::asset_balance(&b, &PCX, Free),
            1_000_000_000 - 1000 - 500
        );
        assert_eq!(Assets::asset_balance(&b, &PCX, ReservedDexSpot), 1000);

        handicapMap = Spot::handicap_map(0).unwrap();

        assert_eq!(handicapMap.buy, 100000);
        assert_eq!(handicapMap.sell, 200000);

        let orderPairPriceOf = Spot::pair_price_of(0).unwrap();
        assert_eq!(orderPairPriceOf.0, 100000);
        assert_eq!(orderPairPriceOf.1, 100000);

        // a 取消第一笔买单
        assert_eq!(Spot::cancel_order(Some(a).into(), 0, 0), Ok(()));

        assert_eq!(
            Assets::asset_balance(&a, &BTC, Free),
            1_000_000_000 - (1000 * 100000) - (1000 * 50) + 500 * 100000
        );
        assert_eq!(
            Assets::asset_balance(&a, &BTC, ReservedDexSpot),
            1000 * 100000 + 1000 * 50 - 500 * 100000 - 500 * 100000
        );
        assert_eq!(Assets::asset_balance(&a, &PCX, Free), 1_000_000_000 + 500);
        assert_eq!(Assets::asset_balance(&a, &PCX, ReservedDexSpot), 0);

        handicapMap = Spot::handicap_map(0).unwrap();

        assert_eq!(handicapMap.buy, 100000 - 1);
        assert_eq!(handicapMap.sell, 200000);

        //a 第三笔挂买单
        assert_eq!(
            Spot::put_order(
                Some(a).into(),
                0,
                OrderType::Limit,
                OrderDirection::Buy,
                1000,
                100000
            ),
            Ok(())
        );
        assert_eq!(
            Assets::asset_balance(&a, &BTC, Free),
            1_000_000_000 - (1000 * 100000) - (1000 * 50) + 500 * 100000 - 1000 * 100000
        );
        assert_eq!(
            Assets::asset_balance(&a, &BTC, ReservedDexSpot),
            1000 * 100000 + 1000 * 50 - 500 * 100000 - 500 * 100000 + 1000 * 100000
        );
        assert_eq!(Assets::asset_balance(&a, &PCX, Free), 1_000_000_000 + 500);
        assert_eq!(Assets::asset_balance(&a, &PCX, ReservedDexSpot), 0);

        assert_eq!(Spot::account_orders_len(&a).unwrap(), 3);

        handicapMap = Spot::handicap_map(0).unwrap();

        assert_eq!(handicapMap.buy, 100000);
        assert_eq!(handicapMap.sell, 200000);
    })
}
