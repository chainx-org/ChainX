// Copyright 2018-2020 Chainpool.

use super::*;
use crate::tests::mock::*;

use runtime_io::with_externalities;

const COUNCIL_ACCOUNT: u64 = 888;

#[test]
fn test09_airdro_distribution_ration_cant_be_zero() {
    with_externalities(&mut new_test_ext(), || {
        assert_noop!(
            XTokens::set_airdrop_distribution_ratio(b"SDOT".to_vec(), 0u32),
            "Shares of AirdropDistributionRatio can not be zero"
        );
    });
}

#[test]
fn test09_cross_chain_asset_power_cant_be_zero() {
    with_externalities(&mut new_test_ext(), || {
        assert_noop!(
            XTokens::set_fixed_cross_chain_asset_power_map(b"BTC".to_vec(), 0u32),
            "Cross chain asset power can not be zero"
        );
    });
}

#[test]
fn test09_calc_global_distribution() {
    with_externalities(&mut new_test_ext(), || {
        let (for_treasury, for_airdrop, for_cross_mining_and_staking) =
            XStaking::calc_global_distribution(4_000_000_000);
        assert_eq!(for_treasury, 480000000);
        assert_eq!(for_airdrop, 320000000);
        assert_eq!(for_cross_mining_and_staking, 3200000000);
    });
}

#[test]
fn test09_global_distribution_ratio_with_zero_xbtc() {
    with_externalities(&mut new_test_ext(), || {
        System::set_block_number(1);
        XSession::check_rotate_session(System::block_number());
        let (sdot_jackpot, lbtc_jackpot, xbtc_jackpot) = token_jackpot_accountids();
        assert_eq!(XAssets::pcx_free_balance(&sdot_jackpot), 160_000_000);
        assert_eq!(XAssets::pcx_free_balance(&lbtc_jackpot), 160_000_000);
        // 4800000000 + 320000000
        // Now xbtc is zero, all cross chain mining belongings goes to the council acccount
        assert_eq!(XAssets::pcx_free_balance(&xbtc_jackpot), 0);
        assert_eq!(
            XAssets::pcx_free_balance(&COUNCIL_ACCOUNT),
            480000000 + 320_000_000
        );
    });
}

#[test]
fn test09_global_distribution_ratio_with_a_few_xbtc() {
    with_externalities(&mut new_test_ext(), || {
        XAssets::issue(&b"BTC".to_vec(), &999, 1250000).unwrap();
        System::set_block_number(1);
        XSession::check_rotate_session(System::block_number());
        let (sdot_jackpot, lbtc_jackpot, xbtc_jackpot) = token_jackpot_accountids();
        assert_eq!(XAssets::pcx_free_balance(&sdot_jackpot), 160_000_000);
        assert_eq!(XAssets::pcx_free_balance(&lbtc_jackpot), 160_000_000);
        // cross_mining_reward_cap: 40*10^8 * 80% * 10% = 320_000_000
        //
        // PCX staking power:               5_000_000_000 => 40* 10^8*72% = 2880000000
        // xbtc mining power: 1250000 * 400 = 500_000_000
        //
        // xbtc    1
        // ---- = ---
        // PCX     9
        //
        // mining_power_cap = 5_000_000_000 * 1 / 9
        //
        //  5_000_000_000 / 9        500_000_000
        //  -----------------   =   ------------
        //  320_000_000                   ?
        //
        //   1         10
        //  --- = -----------------
        //   ?     9 * 320_000_000
        //
        // xbtc_jackpot_free_balance = ? = 288_000_000 = 320_000_000 - 32_000_000
        assert_eq!(
            XAssets::pcx_free_balance(&xbtc_jackpot),
            320_000_000 - 32_000_000
        );
        assert_eq!(
            XAssets::pcx_free_balance(&COUNCIL_ACCOUNT),
            480000000 + 32_000_000
        );
    });
}

#[test]
fn test09_global_distribution_ratio_with_a_lot_of_xbtc() {
    with_externalities(&mut new_test_ext(), || {
        XAssets::issue(&b"BTC".to_vec(), &999, 1_250_000_000).unwrap();
        System::set_block_number(1);
        XSession::check_rotate_session(System::block_number());
        let (sdot_jackpot, lbtc_jackpot, xbtc_jackpot) = token_jackpot_accountids();
        assert_eq!(XAssets::pcx_free_balance(&sdot_jackpot), 160_000_000);
        assert_eq!(XAssets::pcx_free_balance(&lbtc_jackpot), 160_000_000);
        assert_eq!(XAssets::pcx_free_balance(&xbtc_jackpot), 320_000_000);
        assert_eq!(XAssets::pcx_free_balance(&COUNCIL_ACCOUNT), 480000000);
    });
}

#[test]
fn test09_airdrop_asset_power() {
    with_externalities(&mut new_test_ext(), || {
        // XBTC(0) + PCX(5_000_000_000) = 80%
        //
        // sdot + lbtc = 8%
        //   sdot:lbtc = 1:1
        //   sdot = 4% => equialent to have the mining power of 5_000_000_000 / (80 / 4)
        //   lbtc = 4%
        let (sdot, lbtc, xbtc) = tokens();

        assert_ok!(XAssets::issue(&sdot, &1, 1_000_000));
        assert_ok!(XAssets::issue(&lbtc, &1, 1_000_000));

        let sdot_asset_power = XTokens::airdrop_asset_power(&sdot).unwrap();
        let lbtc_asset_power = XTokens::airdrop_asset_power(&lbtc).unwrap();
        let xbtc_asset_power = XTokens::cross_chain_asset_power(&xbtc).unwrap();
        assert_eq!(sdot_asset_power, 250);
        assert_eq!(lbtc_asset_power, 250);
        assert_eq!(xbtc_asset_power, 400);

        // Double the issuanxe of airdrop assets
        assert_ok!(XAssets::issue(&sdot, &1, 1_000_000));
        assert_ok!(XAssets::issue(&lbtc, &1, 1_000_000));
        assert_ok!(XAssets::issue(&xbtc, &1, 100));

        let sdot_asset_power = XTokens::airdrop_asset_power(&sdot).unwrap();
        let lbtc_asset_power = XTokens::airdrop_asset_power(&lbtc).unwrap();
        assert_eq!(sdot_asset_power, 125);
        assert_eq!(lbtc_asset_power, 125);

        assert_ok!(XAssets::issue(&sdot, &1, 2_000_000));
        assert_ok!(XAssets::issue(&lbtc, &1, 2_000_000));

        let sdot_asset_power = XTokens::airdrop_asset_power(&sdot).unwrap();
        let lbtc_asset_power = XTokens::airdrop_asset_power(&lbtc).unwrap();
        assert_eq!(sdot_asset_power, 62);
        assert_eq!(lbtc_asset_power, 62);
    });
}

#[test]
fn test09_cross_chain_asset_power() {
    with_externalities(&mut new_test_ext(), || {
        let (_, _, xbtc) = tokens();

        // XBTC(0) + PCX(5_000_000_000) = 80%
        //   XBTC:PCX = 1:9
        //   XBTC: 10% * 80% = 8%
        //   PCX: 72%
        //
        // Issue more for computing easier.
        assert_ok!(XAssets::pcx_issue(&1, 4_000_000_000u64));
        assert_ok!(XStaking::nominate(
            Origin::signed(1),
            1.into(),
            4_000_000_000u64,
            vec![]
        ));

        let xbtc_asset_power = XTokens::cross_chain_asset_power(&xbtc).unwrap();
        assert_eq!(xbtc_asset_power, 400);

        assert_ok!(XAssets::issue(&xbtc, &1, 100));

        let xbtc_asset_power = XTokens::cross_chain_asset_power(&xbtc).unwrap();
        assert_eq!(xbtc_asset_power, 400);

        assert_ok!(XAssets::issue(&xbtc, &1, 10_000_000 - 100));

        // xbtc raw mining power: 4_000_000_000u64
        // xbtc mining power threshold: 9_000_000_000u64 / 9 = 1_000_000_000
        let xbtc_asset_power = XTokens::cross_chain_asset_power(&xbtc).unwrap();
        assert_eq!(xbtc_asset_power, 100);
    });
}

#[test]
fn test09_test_internal_cross_chain_assets_distribution() {
    with_externalities(&mut new_test_ext(), || {
        XAssets::issue(&b"BTC".to_vec(), &999, 1_250_000_000).unwrap();

        let fake_btc_asset = xassets::Asset::new(
            b"F-BTC".to_vec(),
            b"F-BTC".to_vec(),
            Chain::Bitcoin,
            8, // bitcoin precision
            b"ChainX's Cross-chain Bitcoin".to_vec(),
        )
        .unwrap();

        XAssets::bootstrap_register_asset(fake_btc_asset, true, true).unwrap();
        XTokens::set_fixed_cross_chain_asset_power_map(b"F-BTC".to_vec(), 100u32).unwrap();
        XAssets::issue(&b"F-BTC".to_vec(), &999, 1_250_000_000).unwrap();

        System::set_block_number(1);
        XSession::check_rotate_session(System::block_number());
        let (sdot_jackpot, lbtc_jackpot, xbtc_jackpot) = token_jackpot_accountids();
        assert_eq!(XAssets::pcx_free_balance(&sdot_jackpot), 160_000_000);
        assert_eq!(XAssets::pcx_free_balance(&lbtc_jackpot), 160_000_000);

        // total mining power of X-BTC: F-BTC = 4:1
        assert_eq!(
            XAssets::pcx_free_balance(&xbtc_jackpot),
            320_000_000 * 4 / 5
        );

        let fbtc_jackpot = XTokens::token_jackpot_accountid_for_unsafe(&b"F-BTC".to_vec());
        assert_eq!(
            XAssets::pcx_free_balance(&fbtc_jackpot),
            320_000_000 * 1 / 5
        );

        assert_eq!(XAssets::pcx_free_balance(&COUNCIL_ACCOUNT), 480000000);

        XAssets::issue(&b"F-BTC".to_vec(), &999, 1_250_000_000).unwrap();
        let issue_reward = 100_000;

        System::set_block_number(1);
        XSession::check_rotate_session(System::block_number());

        // now total mining power of X-BTC: F-BTC = 2:1
        assert_eq!(
            XAssets::pcx_free_balance(&fbtc_jackpot),
            320_000_000 * 1 / 5 + 320_000_000 * 1 / 3 - issue_reward
        );
        assert_eq!(
            XAssets::pcx_free_balance(&xbtc_jackpot),
            320_000_000 * 4 / 5 + 320_000_000 - 320_000_000 * 1 / 3
        );
    });
}
