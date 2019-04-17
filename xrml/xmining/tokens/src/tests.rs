// Copyright 2018-2019 Chainpool.

#![cfg(test)]

use super::mock::*;
use super::*;

use runtime_io::with_externalities;
use support::assert_ok;

#[test]
fn issue_sdot_should_work() {
    with_externalities(&mut new_test_ext(), || {
        System::set_block_number(1);
        XSession::check_rotate_session(System::block_number());

        let sdot = <XSdot as ChainT>::TOKEN.to_vec();
        assert_ok!(XAssets::issue(&sdot, &1, 100));
        // amount: 0
        assert_eq!(
            XTokens::psedu_intention_profiles(&sdot),
            PseduIntentionVoteWeight {
                last_total_deposit_weight: 0,
                last_total_deposit_weight_update: 1
            }
        );
        // amount: 0
        assert_eq!(
            XTokens::deposit_records((1, sdot)),
            DepositVoteWeight {
                last_deposit_weight: 0,
                last_deposit_weight_update: 1
            }
        );

        System::set_block_number(2);
        XSession::check_rotate_session(System::block_number());

        let sdot = <XSdot as ChainT>::TOKEN.to_vec();
        // amount: 100
        assert_ok!(XAssets::issue(&sdot, &1, 100));
        assert_eq!(
            XTokens::psedu_intention_profiles(&sdot),
            PseduIntentionVoteWeight {
                last_total_deposit_weight: 100,
                last_total_deposit_weight_update: 2
            }
        );
        // amount: 100
        assert_eq!(
            XTokens::deposit_records((1, sdot)),
            DepositVoteWeight {
                last_deposit_weight: 100,
                last_deposit_weight_update: 2
            }
        );

        System::set_block_number(3);
        XSession::check_rotate_session(System::block_number());

        let sdot = <XSdot as ChainT>::TOKEN.to_vec();
        assert_ok!(XAssets::issue(&sdot, &2, 100));
        // amount: 200
        assert_eq!(
            XTokens::psedu_intention_profiles(&sdot),
            PseduIntentionVoteWeight {
                last_total_deposit_weight: 100 + 200 * 1,
                last_total_deposit_weight_update: 3
            }
        );
        // amount: 0
        assert_eq!(
            XTokens::deposit_records((2, sdot)),
            DepositVoteWeight {
                last_deposit_weight: 0,
                last_deposit_weight_update: 3
            }
        );

        System::set_block_number(4);
        XSession::check_rotate_session(System::block_number());

        let sdot = <XSdot as ChainT>::TOKEN.to_vec();
        assert_ok!(XAssets::issue(&sdot, &2, 100));
        // amount: 300
        assert_eq!(
            XTokens::psedu_intention_profiles(&sdot),
            PseduIntentionVoteWeight {
                last_total_deposit_weight: 300 + 300 * 1,
                last_total_deposit_weight_update: 4
            }
        );
        // amount: 100
        assert_eq!(
            XTokens::deposit_records((2, sdot)),
            DepositVoteWeight {
                last_deposit_weight: 100,
                last_deposit_weight_update: 4
            }
        );

        System::set_block_number(5);
        XSession::check_rotate_session(System::block_number());

        let sdot = <XSdot as ChainT>::TOKEN.to_vec();
        assert_ok!(XAssets::issue(&sdot, &1, 100));
        // amount: 400
        assert_eq!(
            XTokens::psedu_intention_profiles(&sdot),
            PseduIntentionVoteWeight {
                last_total_deposit_weight: 600 + 400 * 1,
                last_total_deposit_weight_update: 5
            }
        );
        // amount: 200
        assert_eq!(
            XTokens::deposit_records((1, sdot)),
            DepositVoteWeight {
                last_deposit_weight: 100 + 200 * 3,
                last_deposit_weight_update: 5
            }
        );

        System::set_block_number(6);
        XSession::check_rotate_session(System::block_number());

        let sdot = <XSdot as ChainT>::TOKEN.to_vec();
        assert_ok!(XAssets::issue(&sdot, &1, 100));
        // amount: 500
        assert_eq!(
            XTokens::psedu_intention_profiles(&sdot),
            PseduIntentionVoteWeight {
                last_total_deposit_weight: 1000 + 500 * 1,
                last_total_deposit_weight_update: 6
            }
        );
        // amount: 300
        assert_eq!(
            XTokens::deposit_records((1, sdot)),
            DepositVoteWeight {
                last_deposit_weight: 700 + 300 * 1,
                last_deposit_weight_update: 6
            }
        );
    });
}

#[test]
fn move_sdot_later_should_work() {
    with_externalities(&mut new_test_ext(), || {
        System::set_block_number(3);
        XSession::check_rotate_session(System::block_number());

        let sdot = <XSdot as ChainT>::TOKEN.to_vec();
        assert_ok!(XAssets::issue(&sdot, &1, 100));
        assert_eq!(
            XTokens::psedu_intention_profiles(&sdot),
            PseduIntentionVoteWeight {
                last_total_deposit_weight: 0,
                last_total_deposit_weight_update: 3
            }
        );

        System::set_block_number(4);
        XSession::check_rotate_session(System::block_number());

        let sdot = <XSdot as ChainT>::TOKEN.to_vec();
        assert_ok!(XAssets::issue(&sdot, &2, 100));
        assert_eq!(
            XTokens::psedu_intention_profiles(&sdot),
            PseduIntentionVoteWeight {
                last_total_deposit_weight: 100,
                last_total_deposit_weight_update: 4
            }
        );

        System::set_block_number(5);
        XSession::check_rotate_session(System::block_number());

        let sdot = <XSdot as ChainT>::TOKEN.to_vec();
        XAssets::move_balance(&sdot, &1, AssetType::Free, &2, AssetType::Free, 10).unwrap();

        assert_eq!(
            XTokens::psedu_intention_profiles(&sdot),
            PseduIntentionVoteWeight {
                last_total_deposit_weight: 100,
                last_total_deposit_weight_update: 4
            }
        );

        assert_eq!(
            XTokens::deposit_records((1, sdot.clone())),
            DepositVoteWeight {
                last_deposit_weight: 100 * 2,
                last_deposit_weight_update: 5
            }
        );

        assert_eq!(
            XTokens::deposit_records((2, sdot)),
            DepositVoteWeight {
                last_deposit_weight: 100 * 1,
                last_deposit_weight_update: 5
            }
        );
    });
}

#[test]
fn claim_sdot_should_work() {
    with_externalities(&mut new_test_ext(), || {
        System::set_block_number(3);
        XSession::check_rotate_session(System::block_number());

        let sdot = <XSdot as ChainT>::TOKEN.to_vec();
        assert_ok!(XAssets::issue(&sdot, &100, 100));

        assert_eq!(
            XTokens::psedu_intention_profiles(&sdot),
            PseduIntentionVoteWeight {
                last_total_deposit_weight: 0,
                last_total_deposit_weight_update: 3
            }
        );

        System::set_block_number(4);
        XSession::check_rotate_session(System::block_number());
        let sdot = <XSdot as ChainT>::TOKEN.to_vec();
        assert_ok!(XAssets::issue(&sdot, &200, 100));

        assert_eq!(
            XTokens::psedu_intention_profiles(&sdot),
            PseduIntentionVoteWeight {
                last_total_deposit_weight: 0 + 100 * 1,
                last_total_deposit_weight_update: 4
            }
        );

        assert_eq!(XAssets::pcx_free_balance(&10), 39603961);
        assert_eq!(XAssets::pcx_free_balance(&100), 0);
        XTokens::apply_claim(&100, &sdot).unwrap();
        // 10% goes to channel/council
        assert_eq!(XAssets::pcx_free_balance(&10), 0);
        assert_eq!(XAssets::pcx_free_balance(&100), 39603961 - 39603961 / 10);

        assert_eq!(
            XTokens::psedu_intention_profiles(&sdot),
            PseduIntentionVoteWeight {
                last_total_deposit_weight: 0,
                last_total_deposit_weight_update: 4
            }
        );

        assert_eq!(
            XTokens::deposit_records((100, sdot.clone())),
            DepositVoteWeight {
                last_deposit_weight: 0,
                last_deposit_weight_update: 4
            }
        );

        System::set_block_number(5);
        XSession::check_rotate_session(System::block_number());

        let sdot = <XSdot as ChainT>::TOKEN.to_vec();
        XAssets::move_balance(&sdot, &100, AssetType::Free, &200, AssetType::Free, 10).unwrap();

        assert_eq!(
            XTokens::deposit_records((100, sdot.clone())),
            DepositVoteWeight {
                last_deposit_weight: 0 + 100 * 1,
                last_deposit_weight_update: 5
            }
        );

        assert_eq!(
            XTokens::deposit_records((200, sdot.clone())),
            DepositVoteWeight {
                last_deposit_weight: 0 + 100 * 1,
                last_deposit_weight_update: 5
            }
        );

        assert_eq!(XAssets::pcx_free_balance(&10), 78431373);
        assert_eq!(XAssets::pcx_free_balance(&100), 35643565);
        XTokens::apply_claim(&100, &sdot).unwrap();
        assert_eq!(XAssets::pcx_free_balance(&10), 39215687);
        assert_eq!(XAssets::pcx_free_balance(&100), 70937683);

        assert_eq!(
            XTokens::psedu_intention_profiles(&sdot),
            PseduIntentionVoteWeight {
                last_total_deposit_weight: 0 + 200 * 1 - 100 * 1,
                last_total_deposit_weight_update: 5
            }
        );
    });
}

#[test]
fn move_sdot_to_an_account_never_deposited_should_work() {
    with_externalities(&mut new_test_ext(), || {
        System::set_block_number(3);
        XSession::check_rotate_session(System::block_number());
        let sdot = <XSdot as ChainT>::TOKEN.to_vec();
        assert_ok!(XAssets::issue(&sdot, &100, 100));

        System::set_block_number(4);
        XSession::check_rotate_session(System::block_number());

        let sdot = <XSdot as ChainT>::TOKEN.to_vec();
        XAssets::move_balance(&sdot, &100, AssetType::Free, &200, AssetType::Free, 10).unwrap();

        assert_eq!(
            XTokens::deposit_records((100, sdot.clone())),
            DepositVoteWeight {
                last_deposit_weight: 0 + 100 * 1,
                last_deposit_weight_update: 4
            }
        );

        assert_eq!(
            XTokens::deposit_records((200, sdot.clone())),
            DepositVoteWeight {
                last_deposit_weight: 0,
                last_deposit_weight_update: 4
            }
        );

        System::set_block_number(5);
        XSession::check_rotate_session(System::block_number());
        XAssets::move_balance(&sdot, &100, AssetType::Free, &200, AssetType::Free, 10).unwrap();

        assert_eq!(
            XTokens::deposit_records((100, sdot.clone())),
            DepositVoteWeight {
                last_deposit_weight: 100 + 90 * 1,
                last_deposit_weight_update: 5
            }
        );

        assert_eq!(
            XTokens::deposit_records((200, sdot.clone())),
            DepositVoteWeight {
                last_deposit_weight: 0 + 10 * 1,
                last_deposit_weight_update: 5
            }
        );
    });
}

#[test]
fn total_token_reward_should_be_right() {
    with_externalities(&mut new_test_ext(), || {
        // validators: 1, 2, 3, 4
        // jackpot: 101, 102, 103, 104
        // team: 666
        // council: 888
        // depositors: 100, 200, 300
        //
        // Initial state: all accounts' balance is 0.
        let all = vec![1, 2, 3, 4, 101, 102, 103, 104, 100, 200, 300, 10, 666, 888];
        assert_eq!(
            all.iter()
                .map(|x| XAssets::pcx_free_balance(x))
                .sum::<u64>(),
            0
        );

        let sdot = <XSdot as ChainT>::TOKEN.to_vec();

        // 5_000_000_000 per session
        System::set_block_number(3);
        XSession::check_rotate_session(System::block_number());
        assert_eq!(XAssets::pcx_free_balance(&666), 1_000_000_000);
        assert_ok!(XAssets::issue(&sdot, &100, 100));

        assert_eq!(XAssets::pcx_free_balance(&1), 100_000_000);
        assert_eq!(XAssets::pcx_free_balance(&101), 900_000_000);

        assert_eq!(
            all.iter()
                .map(|x| XAssets::pcx_free_balance(x))
                .sum::<u64>(),
            5_000_000_000
        );

        System::set_block_number(4);
        XSession::check_rotate_session(System::block_number());
        assert_ok!(XAssets::issue(&sdot, &200, 200));

        assert_eq!(
            all.iter()
                .map(|x| XAssets::pcx_free_balance(x))
                .sum::<u64>(),
            5_000_000_000 * 2
        );

        System::set_block_number(5);
        XSession::check_rotate_session(System::block_number());
        assert_ok!(XAssets::issue(&sdot, &300, 300));

        assert_eq!(
            all.iter()
                .map(|x| XAssets::pcx_free_balance(x))
                .sum::<u64>(),
            5_000_000_000 * 3
        );

        System::set_block_number(6);
        XSession::check_rotate_session(System::block_number());
        XAssets::move_balance(&sdot, &100, AssetType::Free, &200, AssetType::Free, 10).unwrap();

        assert_eq!(
            all.iter()
                .map(|x| XAssets::pcx_free_balance(x))
                .sum::<u64>(),
            5_000_000_000 * 4
        );

        System::set_block_number(7);
        XSession::check_rotate_session(System::block_number());
        XAssets::move_balance(&sdot, &300, AssetType::Free, &100, AssetType::Free, 100).unwrap();

        XTokens::apply_claim(&100, &sdot).unwrap();
        XTokens::apply_claim(&200, &sdot).unwrap();
        XTokens::apply_claim(&300, &sdot).unwrap();

        assert_eq!(
            all.iter()
                .map(|x| XAssets::pcx_free_balance(x))
                .sum::<u64>(),
            5_000_000_000 * 5
        );
    });
}
