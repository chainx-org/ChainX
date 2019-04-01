// Copyright 2018-2019 Chainpool.

#![cfg(test)]

use super::mock::*;

use primitives::testing::UintAuthorityId;
use runtime_io::with_externalities;
use support::assert_ok;

#[test]
fn simple_setup_should_work() {
    with_externalities(&mut new_test_ext(), || {
        assert_eq!(
            Consensus::authorities(),
            vec![
                UintAuthorityId(1).into(),
                UintAuthorityId(2).into(),
                UintAuthorityId(3).into()
            ]
        );
        assert_eq!(XSession::length(), 2);
        assert_eq!(XSession::validators(), vec![(1, 0), (2, 0), (3, 0)]);
    });
}

#[test]
fn should_work_with_early_exit() {
    with_externalities(&mut new_test_ext(), || {
        System::set_block_number(1);
        assert_ok!(XSession::set_length(10));
        assert_eq!(XSession::blocks_remaining(), 1);
        XSession::check_rotate_session(1);

        System::set_block_number(2);
        assert_eq!(XSession::blocks_remaining(), 0);
        XSession::check_rotate_session(2);
        assert_eq!(XSession::length(), 10);

        System::set_block_number(7);
        assert_eq!(XSession::current_index(), 1);
        assert_eq!(XSession::blocks_remaining(), 5);
        assert_ok!(XSession::force_new_session(false));
        XSession::check_rotate_session(7);

        System::set_block_number(8);
        assert_eq!(XSession::current_index(), 2);
        assert_eq!(XSession::blocks_remaining(), 9);
        XSession::check_rotate_session(8);

        System::set_block_number(17);
        assert_eq!(XSession::current_index(), 2);
        assert_eq!(XSession::blocks_remaining(), 0);
        XSession::check_rotate_session(17);

        System::set_block_number(18);
        assert_eq!(XSession::current_index(), 3);
    });
}

#[test]
fn session_length_change_should_work() {
    with_externalities(&mut new_test_ext(), || {
        // Block 1: Change to length 3; no visible change.
        System::set_block_number(1);
        assert_ok!(XSession::set_length(3));
        XSession::check_rotate_session(1);
        assert_eq!(XSession::length(), 2);
        assert_eq!(XSession::current_index(), 0);

        // Block 2: Length now changed to 3. Index incremented.
        System::set_block_number(2);
        assert_ok!(XSession::set_length(3));
        XSession::check_rotate_session(2);
        assert_eq!(XSession::length(), 3);
        assert_eq!(XSession::current_index(), 1);

        // Block 3: Length now changed to 3. Index incremented.
        System::set_block_number(3);
        XSession::check_rotate_session(3);
        assert_eq!(XSession::length(), 3);
        assert_eq!(XSession::current_index(), 1);

        // Block 4: Change to length 2; no visible change.
        System::set_block_number(4);
        assert_ok!(XSession::set_length(2));
        XSession::check_rotate_session(4);
        assert_eq!(XSession::length(), 3);
        assert_eq!(XSession::current_index(), 1);

        // Block 5: Length now changed to 2. Index incremented.
        System::set_block_number(5);
        XSession::check_rotate_session(5);
        assert_eq!(XSession::length(), 2);
        assert_eq!(XSession::current_index(), 2);

        // Block 6: No change.
        System::set_block_number(6);
        XSession::check_rotate_session(6);
        assert_eq!(XSession::length(), 2);
        assert_eq!(XSession::current_index(), 2);

        // Block 7: Next index.
        System::set_block_number(7);
        XSession::check_rotate_session(7);
        assert_eq!(XSession::length(), 2);
        assert_eq!(XSession::current_index(), 3);
    });
}

#[test]
fn session_change_should_work() {
    with_externalities(&mut new_test_ext(), || {
        // Block 1: No change
        System::set_block_number(1);
        XSession::check_rotate_session(1);
        assert_eq!(
            Consensus::authorities(),
            vec![UintAuthorityId(1), UintAuthorityId(2), UintAuthorityId(3)]
        );

        // Block 2: Session rollover, but no change.
        System::set_block_number(2);
        XSession::check_rotate_session(2);
        assert_eq!(
            Consensus::authorities(),
            vec![UintAuthorityId(1), UintAuthorityId(2), UintAuthorityId(3)]
        );

        // Block 3: Set new key for validator 2; no visible change.
        System::set_block_number(3);
        assert_ok!(XSession::set_key(Origin::signed(2), UintAuthorityId(5)));
        assert_eq!(
            Consensus::authorities(),
            vec![UintAuthorityId(1), UintAuthorityId(2), UintAuthorityId(3)]
        );

        XSession::check_rotate_session(3);
        assert_eq!(
            Consensus::authorities(),
            vec![UintAuthorityId(1), UintAuthorityId(2), UintAuthorityId(3)]
        );

        // Block 4: XSession rollover, authority 2 changes.
        System::set_block_number(4);
        XSession::check_rotate_session(4);
        assert_eq!(
            Consensus::authorities(),
            vec![UintAuthorityId(1), UintAuthorityId(5), UintAuthorityId(3)]
        );
    });
}
