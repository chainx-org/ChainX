use super::*;
use crate::mock::*;
use frame_support::{assert_err, assert_noop, assert_ok};

#[test]
fn bond_should_work() {
    ExtBuilder::default().build_and_execute(|| {
        assert_eq!(
            <ValidatorLedgers<Test>>::get(2),
            ValidatorLedger {
                total: 20,
                last_total_vote_weight: 0,
                last_total_vote_weight_update: 1,
            }
        );
        System::set_block_number((System::block_number() + 1).into());
        assert_ok!(XStaking::bond(
            Origin::signed(1),
            2,
            5,
            b"memo".as_ref().into()
        ));
        assert_eq!(
            <ValidatorLedgers<Test>>::get(2),
            ValidatorLedger {
                total: 25,
                last_total_vote_weight: 20,
                last_total_vote_weight_update: 2,
            }
        );
        assert_eq!(
            <Nominations<Test>>::get(1, 2),
            NominatorLedger {
                value: 5,
                last_vote_weight: 0,
                last_vote_weight_update: 2,
            }
        );
    });
}

#[test]
fn unbond_should_work() {
    ExtBuilder::default().build_and_execute(|| {
        assert_eq!(
            <ValidatorLedgers<Test>>::get(2),
            ValidatorLedger {
                total: 20,
                last_total_vote_weight: 0,
                last_total_vote_weight_update: 1,
            }
        );

        System::set_block_number((System::block_number() + 1).into());

        assert_ok!(XStaking::bond(
            Origin::signed(1),
            2,
            5,
            b"memo".as_ref().into()
        ));

        assert_eq!(
            <ValidatorLedgers<Test>>::get(2),
            ValidatorLedger {
                total: 25,
                last_total_vote_weight: 20,
                last_total_vote_weight_update: 2,
            }
        );
    });
}
