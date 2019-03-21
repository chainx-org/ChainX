// Copyright 2018 Chainpool.
//! Test utilities

#![cfg(test)]

use super::*;
use primitives::testing::{ConvertUintAuthorityId, Digest, DigestItem, Header, UintAuthorityId};
use primitives::traits::{BlakeTwo256, IdentityLookup};
use primitives::BuildStorage;
use runtime_io;
use runtime_io::with_externalities;
use substrate_primitives::{Blake2Hasher, H256};
use support::impl_outer_origin;
use support::{assert_noop, assert_ok};
use {balances, consensus, indices, session, system, timestamp, Module, Trait};

impl_outer_origin! {
    pub enum Origin for Test {}
}

// Workaround for https://github.com/rust-lang/rust/issues/26925 . Remove when sorted.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Test;

impl consensus::Trait for Test {
    type Log = DigestItem;
    type SessionKey = UintAuthorityId;
    type InherentOfflineReport = ();
}
impl system::Trait for Test {
    type Origin = Origin;
    type Index = u64;
    type BlockNumber = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type Digest = Digest;
    type AccountId = u64;
    type Lookup = IdentityLookup<u64>;
    type Header = Header;
    type Event = ();
    type Log = DigestItem;
}
impl indices::Trait for Test {
    type AccountIndex = u32;
    type IsDeadAccount = Balances;
    type ResolveHint = indices::SimpleResolveHint<Self::AccountId, Self::AccountIndex>;
    type Event = ();
}
impl balances::Trait for Test {
    type Balance = u64;
    type OnNewAccount = Indices;
    type OnFreeBalanceZero = ();
    type Event = ();
}
impl timestamp::Trait for Test {
    type Moment = u64;
    type OnTimestampSet = ();
}
impl session::Trait for Test {
    type ConvertAccountIdToSessionKey = ConvertUintAuthorityId;
    type OnSessionChange = ();
    type Event = ();
}
impl Trait for Test {
    type Event = ();
    type DetermineIntentionJackpotAccountId = MockDeterminator;
}

pub struct MockDeterminator;

impl IntentionJackpotAccountIdFor<u64> for MockDeterminator {
    fn accountid_for(_: &u64) -> u64 {
        1000
    }
}

pub fn new_test_ext() -> runtime_io::TestExternalities<Blake2Hasher> {
    let mut t = system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap()
        .0;
    t.extend(
        consensus::GenesisConfig::<Test> {
            code: vec![],
            authorities: vec![],
        }
        .build_storage()
        .unwrap()
        .0,
    );
    t.extend(
        session::GenesisConfig::<Test> {
            session_length: 1,
            validators: vec![10, 20],
            keys: vec![],
        }
        .build_storage()
        .unwrap()
        .0,
    );

    t.extend(
        balances::GenesisConfig::<Test> {
            balances: vec![(1, 10), (2, 20), (3, 30), (4, 40), (10, 100), (20, 100)],
            existential_deposit: 0,
            transfer_fee: 0,
            creation_fee: 0,
            vesting: vec![],
        }
        .build_storage()
        .unwrap()
        .0,
    );
    runtime_io::TestExternalities::new(t)
}

pub type System = system::Module<Test>;
pub type XAccounts = Module<Test>;

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

pub type Indices = indices::Module<Test>;
pub type Balances = balances::Module<Test>;
