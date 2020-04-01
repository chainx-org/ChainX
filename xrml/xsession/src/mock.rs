// Copyright 2018-2019 Chainpool.

#![cfg(test)]

use super::*;

use primitives::testing::{ConvertUintAuthorityId, Digest, DigestItem, Header, UintAuthorityId};
use primitives::traits::{BlakeTwo256, IdentityLookup};
use primitives::BuildStorage;
use substrate_primitives::{Blake2Hasher, H256};
use support::impl_outer_origin;

impl_outer_origin! {
    pub enum Origin for Test {}
}

#[derive(Clone, Eq, PartialEq)]
pub struct Test;

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

impl consensus::Trait for Test {
    type Log = DigestItem;
    type SessionKey = UintAuthorityId;
    type InherentOfflineReport = ();
}

impl timestamp::Trait for Test {
    type Moment = u64;
    type OnTimestampSet = ();
}

impl xaccounts::Trait for Test {
    type DetermineIntentionJackpotAccountId = MockDeterminator;
}

pub struct MockDeterminator;

impl xaccounts::IntentionJackpotAccountIdFor<u64> for MockDeterminator {
    fn accountid_for_unsafe(_: &u64) -> u64 {
        1000
    }
    fn accountid_for_safe(_: &u64) -> Option<u64> {
        Some(1000)
    }
}

impl Trait for Test {
    type ConvertAccountIdToSessionKey = ConvertUintAuthorityId;
    type OnSessionChange = ();
    type Event = ();
}

pub type System = system::Module<Test>;
pub type Consensus = consensus::Module<Test>;
pub type XSession = Module<Test>;

pub fn new_test_ext() -> runtime_io::TestExternalities<Blake2Hasher> {
    let mut t = system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap()
        .0;
    t.extend(
        consensus::GenesisConfig::<Test> {
            code: vec![],
            authorities: vec![UintAuthorityId(1), UintAuthorityId(2), UintAuthorityId(3)],
        }
        .build_storage()
        .unwrap()
        .0,
    );
    t.extend(
        timestamp::GenesisConfig::<Test> { minimum_period: 5 }
            .build_storage()
            .unwrap()
            .0,
    );
    t.extend(
        GenesisConfig::<Test> {
            keys: vec![],
            session_length: 2,
            validators: vec![(1, 0), (2, 0), (3, 0)],
        }
        .build_storage()
        .unwrap()
        .0,
    );
    runtime_io::TestExternalities::new(t)
}
