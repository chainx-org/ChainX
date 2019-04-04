// Copyright 2018-2019 Chainpool.

#![cfg(test)]

use super::*;

use primitives::testing::{Digest, DigestItem, Header, UintAuthorityId};
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

impl balances::Trait for Test {
    type Balance = u64;
    type OnFreeBalanceZero = ();
    type OnNewAccount = ();
    type TransactionPayment = ();
    type TransferPayment = ();
    type DustRemoval = ();
    type Event = ();
}

impl consensus::Trait for Test {
    type Log = DigestItem;
    type SessionKey = UintAuthorityId;
    type InherentOfflineReport = ();
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
        Some(0)
    }
    fn get_validator_name(accountid: &u64) -> Option<Vec<u8>> {
        None
    }
}

impl xaccounts::Trait for Test {
    type Event = ();
    type DetermineIntentionJackpotAccountId = MockDeterminator;
}

pub struct MockDeterminator;

impl xaccounts::IntentionJackpotAccountIdFor<u64> for MockDeterminator {
    fn accountid_for(_: &u64) -> u64 {
        1000
    }
}

impl xassets::Trait for Test {
    type Event = ();
    type OnAssetChanged = ();
    type OnAssetRegisterOrRevoke = ();
}

impl Trait for Test {}

pub type XAssets = xassets::Module<Test>;
pub type XFeeManager = Module<Test>;

pub fn new_test_ext() -> runtime_io::TestExternalities<Blake2Hasher> {
    let mut r = system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap()
        .0;
    // balance
    r.extend(
        balances::GenesisConfig::<Test> {
            balances: vec![(1, 1000), (2, 510), (3, 1000)],
            transaction_base_fee: 10,
            transaction_byte_fee: 1,
            existential_deposit: 0,
            transfer_fee: 0,
            creation_fee: 0,
            vesting: vec![],
        }
        .build_storage()
        .unwrap()
        .0,
    );
    // xsystem
    r.extend(
        xsystem::GenesisConfig::<Test> {
            death_account: 0,
            burn_account: 100,
        }
        .build_storage()
        .unwrap()
        .0,
    );
    // xassets
    r.extend(
        xassets::GenesisConfig::<Test> {
            memo_len: 128,
            _genesis_phantom_data: Default::default(),
        }
        .build_storage()
        .unwrap()
        .0,
    );

    r.extend(
        GenesisConfig::<Test> {
            producer_fee_proportion: (1, 10),
            transaction_base_fee: 10,
            transaction_byte_fee: 1,
        }
        .build_storage()
        .unwrap()
        .0,
    );
    r.into()
}
