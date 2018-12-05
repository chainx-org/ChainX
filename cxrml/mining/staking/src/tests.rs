// Copyright 2017 Parity Technologies (UK) Ltd.
// This file is part of Substrate.

// Substrate is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Substrate is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Substrate.  If not, see <http://www.gnu.org/licenses/>.

//! Tests for the module.

#![cfg(test)]
use super::*;
#[allow(unused_imports)]
use mock::{
    new_test_ext, Associations, Balances, Origin, Session, Staking, System, Test, Timestamp,
};
use runtime_io::with_externalities;

#[test]
fn initialize_should_work() {
    with_externalities(&mut new_test_ext(0, 1, 1, 0, true, 10), || {
        assert_eq!(Staking::era_length(), 1);
        assert_eq!(Staking::sessions_per_era(), 1);
        assert_eq!(Staking::last_era_length_change(), 0);
        assert_eq!(Staking::current_era(), 0);
        assert_eq!(Session::current_index(), 0);

        assert_eq!(Staking::cert_profiles(&0).remaining_shares, 44);
        assert_eq!(Staking::cert_profiles(&0).owner, 10);

        assert_eq!(Staking::intention_profiles(&10).is_active, true);
        assert_eq!(Staking::intention_profiles(&10).activator_index, 0);
        assert_eq!(
            Staking::intention_profiles(&10).total_nomination,
            100_000_000
        );

        assert_eq!(Staking::intentions(), [10]);
        assert_eq!(Balances::reserved_balance(&10), 100_000_000);
        assert_eq!(Balances::free_balance(&10), 99900000000);

        assert_eq!(
            Staking::nomination_record_of(&10, &10).nomination,
            100_000_000
        );

        assert_eq!(Staking::nominator_profiles(&10).nominees, [10]);

        assert_eq!(
            Associations::channel_relationship(&b"ChainX".to_vec()),
            Some(10)
        );
        assert_eq!(
            Associations::channel_relationship_rev(&10),
            Some(b"ChainX".to_vec())
        );
    });
}

#[test]
fn session_rewards_should_work() {
    with_externalities(&mut new_test_ext(0, 1, 1, 0, true, 10), || {
        Balances::set_free_balance(&10, 0);

        System::set_block_number(1);
        Session::check_rotate_session(System::block_number());
        assert_eq!(Balances::free_balance(&10), 10);
        assert_eq!(Staking::intention_profiles(&10).jackpot, 90);

        System::set_block_number(2);
        Session::check_rotate_session(System::block_number());
        assert_eq!(Balances::free_balance(&10), 20);
        assert_eq!(Staking::intention_profiles(&10).jackpot, 180);

        System::set_block_number(3);
        Session::check_rotate_session(System::block_number());
        assert_eq!(Balances::free_balance(&10), 30);
        assert_eq!(Staking::intention_profiles(&10).jackpot, 270);
    });
}

#[test]
fn register_should_work() {
    with_externalities(&mut new_test_ext(0, 1, 1, 0, true, 10), || {
        Balances::set_free_balance(&10, 0);

        System::set_block_number(1);
        Session::check_rotate_session(System::block_number());
        assert_eq!(Balances::free_balance(&10), 10);
        assert_eq!(Staking::intention_profiles(&10).jackpot, 90);

        System::set_block_number(2);
        Session::check_rotate_session(System::block_number());
        assert_eq!(Balances::free_balance(&10), 20);
        assert_eq!(Staking::intention_profiles(&10).jackpot, 180);

        System::set_block_number(3);
        Session::check_rotate_session(System::block_number());
        assert_eq!(Balances::free_balance(&10), 30);
        assert_eq!(Staking::intention_profiles(&10).jackpot, 270);
    });
}

#[test]
fn activate_and_deactivate_should_work() {
    with_externalities(&mut new_test_ext(0, 1, 1, 0, true, 10), || {
        Balances::set_free_balance(&10, 0);

        assert_eq!(Staking::intentions(), [10]);

        System::set_block_number(1);
        assert_ok!(Staking::register(
            Origin::signed(10),
            0,
            1,
            String::from("1").into_bytes(),
            String::from("url").into_bytes(),
            1
        ));
        assert_eq!(Staking::intentions(), [10, 1]);
        Session::check_rotate_session(System::block_number());

        assert_ok!(Staking::activate(Origin::signed(1)));

        System::set_block_number(2);
        Session::check_rotate_session(System::block_number());
        assert_eq!(Session::validators(), [10, 1]);

        assert_ok!(Staking::deactivate(Origin::signed(1)));

        System::set_block_number(3);
        Session::check_rotate_session(System::block_number());
        assert_eq!(Session::validators(), [10]);

        System::set_block_number(4);
        Session::check_rotate_session(System::block_number());

        System::set_block_number(5);
        Session::check_rotate_session(System::block_number());

        System::set_block_number(6);
        Session::check_rotate_session(System::block_number());
    });
}

#[test]
fn stake_should_work() {
    with_externalities(&mut new_test_ext(0, 1, 1, 0, true, 10), || {
        Balances::set_free_balance(&10, 100_000_000);

        assert_eq!(Staking::intentions(), [10]);

        System::set_block_number(1);
        Session::check_rotate_session(System::block_number());
        assert_eq!(Balances::free_balance(&10), 100_000_010);
        assert_eq!(Staking::intention_profiles(&10).jackpot, 90);

        assert_ok!(Staking::stake(Origin::signed(10), 100_000_000));
        assert_eq!(Balances::free_balance(&10), 10);

        System::set_block_number(2);
        Session::check_rotate_session(System::block_number());
        assert_eq!(Balances::free_balance(&10), 30);
        assert_eq!(Staking::intention_profiles(&10).jackpot, 270);

        assert_eq!(
            Staking::intention_profiles(&10).total_nomination,
            200_000_000
        );
    });
}

#[test]
fn claim_should_work() {
    with_externalities(&mut new_test_ext(0, 1, 1, 0, true, 10), || {
        Balances::set_free_balance(&10, 0);

        System::set_block_number(1);
        Session::check_rotate_session(System::block_number());
        assert_eq!(Balances::free_balance(&10), 10);
        assert_eq!(Staking::intention_profiles(&10).jackpot, 90);

        assert_ok!(Staking::claim(Origin::signed(10), 10.into()));
        assert_eq!(Staking::intention_profiles(&10).jackpot, 0);
        assert_eq!(Balances::free_balance(&10), 100);

        System::set_block_number(2);
        Session::check_rotate_session(System::block_number());
        assert_eq!(Balances::free_balance(&10), 110);
        assert_eq!(Staking::intention_profiles(&10).jackpot, 90);

        System::set_block_number(3);
        Session::check_rotate_session(System::block_number());
        assert_eq!(Balances::free_balance(&10), 120);
        assert_eq!(Staking::intention_profiles(&10).jackpot, 180);

        assert_ok!(Staking::claim(Origin::signed(10), 10.into()));
        assert_eq!(Balances::free_balance(&10), 300);
    });
}

#[test]
fn nominate_and_claim_should_work() {
    with_externalities(&mut new_test_ext(0, 1, 1, 0, true, 10), || {
        Balances::set_free_balance(&10, 0);
        Balances::set_free_balance(&20, 100_000_000);

        System::set_block_number(1);
        assert_ok!(Staking::nominate(
            Origin::signed(20),
            10.into(),
            100_000_000
        ));
        Session::check_rotate_session(System::block_number());
        assert_eq!(Balances::free_balance(&10), 20);
        assert_eq!(Staking::intention_profiles(&10).jackpot, 180);

        assert_ok!(Staking::claim(Origin::signed(10), 10.into()));
        assert_eq!(Staking::intention_profiles(&10).jackpot, 0);
        assert_eq!(Balances::free_balance(&10), 200);

        System::set_block_number(2);
        Session::check_rotate_session(System::block_number());
        assert_eq!(Balances::free_balance(&10), 220);
        assert_eq!(Staking::intention_profiles(&10).jackpot, 180);

        assert_ok!(Staking::claim(Origin::signed(20), 10.into()));
        assert_eq!(Staking::intention_profiles(&10).jackpot, 90);
        assert_eq!(Balances::free_balance(&20), 90);

        System::set_block_number(3);
        Session::check_rotate_session(System::block_number());
        assert_eq!(Balances::free_balance(&10), 240);
        assert_eq!(Staking::intention_profiles(&10).jackpot, 270);

        assert_ok!(Staking::claim(Origin::signed(10), 10.into()));
        assert_eq!(Balances::free_balance(&10), 420);
    });
}
