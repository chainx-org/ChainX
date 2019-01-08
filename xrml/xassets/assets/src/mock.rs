// Copyright 2018 Chainpool.

use substrate_primitives::{Blake2Hasher, H256};

use primitives::testing::{Digest, DigestItem, Header};
use primitives::traits::BlakeTwo256;
use primitives::BuildStorage;
use runtime_io;

use super::*;

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
    type Header = Header;
    type Event = ();
    type Log = DigestItem;
}

impl balances::Trait for Test {
    type Balance = u64;
    type AccountIndex = u64;
    type OnFreeBalanceZero = ();
    type EnsureAccountLiquid = ();
    type Event = ();
}

impl Trait for Test {
    /// Event
    type Event = ();
    type OnAssetChanged = ();
    type OnAssetRegistration = ();
}

pub type XAssets = Module<Test>;
pub type Balances = balances::Module<Test>;
pub type Balance = u64;

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
            transaction_base_fee: 0,
            transaction_byte_fee: 0,
            existential_deposit: 0,
            transfer_fee: 0,
            creation_fee: 0,
            reclaim_rebate: 0,
        }
        .build_storage()
        .unwrap()
        .0,
    );

    let btc_asset = Asset::new(
        b"BTC".to_vec(), // token
        Chain::Bitcoin,
        8, // bitcoin precision
        b"BTC chainx".to_vec(),
    )
    .unwrap();

    r.extend(
        GenesisConfig::<Test> {
            pcx: (3, b"PCX onchain token".to_vec()),
            memo_len: 128,
            // asset, is_psedu_intention, init for account
            // Vec<(Asset, bool, Vec<(T::AccountId, u64)>)>;
            asset_list: vec![(btc_asset, true, vec![(3, 100)])],
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
            transaction_base_fee: 0,
            transaction_byte_fee: 0,
            existential_deposit: 0,
            transfer_fee: 0,
            creation_fee: 0,
            reclaim_rebate: 0,
        }
        .build_storage()
        .unwrap()
        .0,
    );

    let btc_asset = Asset::new(
        b"BTC******".to_vec(), // token
        Chain::Bitcoin,
        8, // bitcoin precision
        b"BTC chainx".to_vec(),
    )
    .unwrap();

    r.extend(
        GenesisConfig::<Test> {
            pcx: (3, b"PCX onchain token".to_vec()),
            memo_len: 128,
            // asset, is_psedu_intention, init for account
            // Vec<(Asset, bool, Vec<(T::AccountId, u64)>)>;
            asset_list: vec![(btc_asset, true, vec![(3, 100)])],
        }
        .build_storage()
        .unwrap()
        .0,
    );

    r.into()
}
