// Copyright 2018-2019 Chainpool.

#![cfg(test)]

use super::mock::*;

use runtime_io::with_externalities;

#[test]
fn issue_should_work() {
    with_externalities(&mut new_test_ext(), || {
        //        System::set_block_number(10);
        //        assert_ok!(XAccounts::issue(b"alice".to_vec(), 1, 1));
        //        assert_eq!(XAccounts::total_issued(), 3);
        //        assert_eq!(
        //            XAccounts::cert_immutable_props_of(b"alice".to_vec()),
        //            CertImmutableProps {
        //                issued_at: 10,
        //                frozen_duration: 1
        //            }
        //        );
        //        assert_eq!(XAccounts::remaining_shares_of(b"alice".to_vec()), 50);
        //        assert_noop!(
        //            XAccounts::issue(b"alice".to_vec(), 1, 1),
        //            "Cannot issue if this cert name already exists."
        //        );
    });
}
