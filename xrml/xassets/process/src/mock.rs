// Copyright 2018-2019 Chainpool.

#![cfg(test)]

use super::*;

// Substrate
use primitives::testing::{Digest, DigestItem, Header, UintAuthorityId};
use primitives::traits::{BlakeTwo256, IdentityLookup};
use primitives::BuildStorage;
use substrate_primitives::{Blake2Hasher, H256};
use support::impl_outer_origin;

// ChainX
use xassets::{Asset, Chain};

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
        Some(0)
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

impl xrecords::Trait for Test {
    type Event = ();
}

impl xfee_manager::Trait for Test {}

impl xbitcoin::Trait for Test {
    type Event = ();
}

impl Trait for Test {}

pub type XAssets = xassets::Module<Test>;
pub type XRecords = xrecords::Module<Test>;
pub type XBitCoin = xbitcoin::Module<Test>;
pub type XProcess = Module<Test>;

// This function basically just builds a genesis storage key/value store according to
// our desired mockup.
pub fn new_test_ext() -> runtime_io::TestExternalities<Blake2Hasher> {
    let mut r = system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap()
        .0;
    // balance
    r.extend(
        balances::GenesisConfig::<Test> {
            balances: vec![(1, 1000), (2, 510)],
            existential_deposit: 500,
            transfer_fee: 0,
            creation_fee: 0,
            transaction_base_fee: 0,
            transaction_byte_fee: 0,
            vesting: vec![],
        }
        .build_storage()
        .unwrap()
        .0,
    );
    // token balance
    let _btc_asset = Asset::new(
        b"BTC".to_vec(),     // token
        b"Bitcoin".to_vec(), // token
        Chain::Bitcoin,
        8, // bitcoin precision
        b"BTC chainx".to_vec(),
    )
    .unwrap();

    // token balance
    let xdot_asset = Asset::new(
        b"XDOT".to_vec(), // token
        b"XDOT".to_vec(), // token
        Chain::Ethereum,
        3,
        b"XDOT chainx".to_vec(),
    )
    .unwrap();

    // bridge btc
    r.extend(
        xbitcoin::GenesisConfig::<Test> {
            // start genesis block: (genesis, blocknumber)
            genesis: Default::default(),
            params_info: Default::default(),
            confirmation_number: 6,
            reserved_block: 2100,
            btc_withdrawal_fee: 40000,
            max_withdrawal_count: 10,
            _genesis_phantom_data: Default::default(),
        }
        .build_storage()
        .unwrap()
        .0,
    );

    r.extend(
        GenesisConfig::<Test> {
            token_black_list: vec![xdot_asset.token()],
            _genesis_phantom_data: Default::default(),
        }
        .build_storage()
        .unwrap()
        .0,
    );

    r.extend(
        xassets::GenesisConfig::<Test> {
            memo_len: 128,
            _genesis_phantom_data: Default::default(),
        }
        .build_storage()
        .unwrap()
        .0,
    );

    r.into()
}
