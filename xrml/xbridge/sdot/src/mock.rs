// Copyright 2018-2019 Chainpool.

#![cfg(test)]

use primitives::testing::{Digest, DigestItem, Header};
use primitives::traits::{BlakeTwo256, IdentityLookup};
use primitives::BuildStorage;
use substrate_primitives::{Blake2Hasher, H256};
use support::impl_outer_origin;

use super::*;

impl_outer_origin! {
    pub enum Origin for Test {}
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Test;

impl system::Trait for Test {
    type Origin = Origin;
    type Index = u64;
    type BlockNumber = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type Digest = Digest;
    type AccountId = u64;
    type Lookup = Indices;
    type Header = Header;
    type Event = ();
    type Log = DigestItem;
}

impl indices::Trait for Test {
    type AccountIndex = u32;
    type IsDeadAccount = XAssets;
    type ResolveHint = indices::SimpleResolveHint<Self::AccountId, Self::AccountIndex>;
    type Event = ();
}

impl timestamp::Trait for Test {
    type Moment = u64;
    type OnTimestampSet = ();
}

impl xsystem::Trait for Test {
    type ValidatorList = MockValidatorList;
    type Validator = MockValidator;
}
pub struct MockValidatorList;
impl xsystem::ValidatorList<u64> for MockValidatorList {
    fn validator_list() -> Vec<u64> {
        vec![]
    }
}
pub struct MockValidator;
impl xsystem::Validator<u64> for MockValidator {
    fn get_validator_by_name(_name: &[u8]) -> Option<u64> {
        None
    }
    fn get_validator_name(_account_id: &u64) -> Option<Vec<u8>> {
        None
    }
}

impl xassets::Trait for Test {
    type Balance = u64;
    type OnNewAccount = Indices;
    type Event = ();
    type OnAssetChanged = ();
    type OnAssetRegisterOrRevoke = ();
}

impl xrecords::Trait for Test {
    type Event = ();
}

//impl Trait for Test {
//    type AccountExtractor = ();
//    type CrossChainProvider = ();
//    type Event = ();
//}

pub type System = system::Module<Test>;
pub type Indices = indices::Module<Test>;
pub type Timestamp = timestamp::Module<Test>;
pub type XSystem = xsystem::Module<Test>;
pub type XAssets = xassets::Module<Test>;
pub type XRecords = xrecords::Module<Test>;
pub type XSdot = Module<Test>;
