// Copyright 2018-2019 Chainpool.

#![cfg(test)]

use super::*;

use primitives::testing::{Digest, DigestItem, Header};
use primitives::traits::BlakeTwo256;
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
    type Lookup = Indices;
    type Header = Header;
    type Event = ();
    type Log = DigestItem;
}

impl balances::Trait for Test {
    type Balance = u64;
    type OnFreeBalanceZero = ();
    type OnNewAccount = Indices;
    type TransactionPayment = ();
    type TransferPayment = ();
    type DustRemoval = ();
    type Event = ();
}

impl indices::Trait for Test {
    type AccountIndex = u32;
    type IsDeadAccount = Balances;
    type ResolveHint = indices::SimpleResolveHint<Self::AccountId, Self::AccountIndex>;
    type Event = ();
}

impl Trait for Test {
    /// Event
    type Event = ();
    type OnAssetChanged = ();
    type OnAssetRegisterOrRevoke = ();
}

pub type Balance = u64;
pub type Balances = balances::Module<Test>;
pub type Indices = indices::Module<Test>;
pub type XAssets = Module<Test>;

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
            balances: vec![(1, 1000), (2, 510), (3, 1000)],
            existential_deposit: 0,
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
    // indices
    r.extend(
        indices::GenesisConfig::<Test> { ids: vec![1, 2, 3] }
            .build_storage()
            .unwrap()
            .0,
    );

    let _btc_asset = Asset::new(
        b"BTC".to_vec(), // token
        b"Bitcoin".to_vec(),
        Chain::Bitcoin,
        8, // bitcoin precision
        b"BTC chainx".to_vec(),
    )
    .unwrap();

    r.extend(
        GenesisConfig::<Test> {
            memo_len: 128,
            _genesis_phantom_data: Default::default(),
        }
        .build_storage()
        .unwrap()
        .0,
    );

    r.into()
}

pub fn err_test_ext() -> runtime_io::TestExternalities<Blake2Hasher> {
    let mut r = system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap()
        .0;
    // balance
    r.extend(
        balances::GenesisConfig::<Test> {
            balances: vec![(1, 1000), (2, 510), (3, 1000)],
            existential_deposit: 0,
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

    let _btc_asset = Asset::new(
        b"BTC******".to_vec(), // token
        b"Bitcoin".to_vec(),   // token
        Chain::Bitcoin,
        8, // bitcoin precision
        b"BTC chainx".to_vec(),
    )
    .unwrap();

    r.extend(
        GenesisConfig::<Test> {
            memo_len: 128,
            _genesis_phantom_data: Default::default(),
        }
        .build_storage()
        .unwrap()
        .0,
    );

    r.into()
}
