// Copyright 2018-2020 Chainpool.

use super::*;

#[test]
fn test09_global_distribution_ratio_cant_be_all_zero() {
    with_externalities(&mut new_test_ext(), || {
        assert_noop!(
            XStaking::set_global_distribution_ratio((0, 0, 0)),
            "CrossMiningAndPCXStaking shares can not be zero"
        );
    });
}
