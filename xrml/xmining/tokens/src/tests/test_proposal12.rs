// Copyright 2018-2020 Chainpool.

use super::*;
use crate::tests::mock::*;
use runtime_io::with_externalities;

#[test]
fn test12_lbtc_and_sdot_claim_not_allowed() {
    with_externalities(&mut new_test_ext(), || {
        System::set_block_number(3);
        XSession::check_rotate_session(System::block_number());

        let sdot = <XSdot as ChainT>::TOKEN.to_vec();
        assert_ok!(XAssets::issue(&sdot, &100, 100));
        assert_ok!(XTokens::set_claim_restriction(sdot.clone(), (0, 0)));

        System::set_block_number(4);
        XSession::check_rotate_session(System::block_number());
        let sdot = <XSdot as ChainT>::TOKEN.to_vec();
        assert_ok!(XAssets::issue(&sdot, &200, 100));

        assert_noop!(
            XTokens::claim(Origin::signed(100), sdot.clone()),
            "Cannot claim from LBTC and SDOT since Proposal 12 removed these airdrop assets"
        );
    });
}

#[test]
fn test12_global_distribution_ratio_with_zero_xbtc() {
    with_externalities(&mut new_test_ext(), || {
        System::set_block_number(1);
        XSession::check_rotate_session(System::block_number());
        let (sdot_jackpot, lbtc_jackpot, xbtc_jackpot) = token_jackpot_accountids();
        // Since Proposal 12, sdot and lbtc's rewards are belong to the treasury.
        assert_eq!(XAssets::pcx_free_balance(&sdot_jackpot), 0);
        assert_eq!(XAssets::pcx_free_balance(&lbtc_jackpot), 0);
        // 4800000000 + 320000000
        // Now xbtc is zero, all cross chain mining belongings goes to the council acccount
        assert_eq!(XAssets::pcx_free_balance(&xbtc_jackpot), 0);
        assert_eq!(
            XAssets::pcx_free_balance(&COUNCIL_ACCOUNT),
            480000000 + 320_000_000 + 160_000_000 * 2
        );
    });
}

#[test]
fn test12_global_distribution_ratio_with_a_few_xbtc() {
    with_externalities(&mut new_test_ext(), || {
        XAssets::issue(&b"BTC".to_vec(), &999, 1250000).unwrap();
        System::set_block_number(1);
        XSession::check_rotate_session(System::block_number());
        let (sdot_jackpot, lbtc_jackpot, xbtc_jackpot) = token_jackpot_accountids();
        // Since Proposal 12, sdot and lbtc's rewards are belong to the treasury.
        assert_eq!(XAssets::pcx_free_balance(&sdot_jackpot), 0);
        assert_eq!(XAssets::pcx_free_balance(&lbtc_jackpot), 0);
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
            480000000 + 32_000_000 + 160_000_000 * 2
        );
    });
}

#[test]
fn test12_global_distribution_ratio_with_a_lot_of_xbtc() {
    with_externalities(&mut new_test_ext(), || {
        XAssets::issue(&b"BTC".to_vec(), &999, 1_250_000_000).unwrap();
        System::set_block_number(1);
        XSession::check_rotate_session(System::block_number());
        let (sdot_jackpot, lbtc_jackpot, xbtc_jackpot) = token_jackpot_accountids();
        assert_eq!(XAssets::pcx_free_balance(&sdot_jackpot), 0);
        assert_eq!(XAssets::pcx_free_balance(&lbtc_jackpot), 0);
        assert_eq!(XAssets::pcx_free_balance(&xbtc_jackpot), 320_000_000);
        assert_eq!(
            XAssets::pcx_free_balance(&COUNCIL_ACCOUNT),
            480000000 + 160_000_000 * 2
        );
    });
}
