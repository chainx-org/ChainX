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

impl timestamp::Trait for Test {
    type Moment = u64;
    type OnTimestampSet = ();
}

// assets
impl xassets::Trait for Test {
    type Event = ();
    type OnAssetChanged = ();
    type OnAssetRegisterOrRevoke = ();
}

impl Trait for Test {
    type Event = ();
}

pub type XAssets = xassets::Module<Test>;
pub type XRecords = Module<Test>;

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

    let _btc_asset = xassets::Asset::new(
        b"BTC".to_vec(),     // token
        b"Bitcoin".to_vec(), // token
        Chain::Bitcoin,
        8, // bitcoin precision
        b"BTC chainx".to_vec(),
    )
    .unwrap();

    let _eth_asset = xassets::Asset::new(
        b"ETH".to_vec(),      // token
        b"Ethereum".to_vec(), // token
        Chain::Ethereum,
        8, // bitcoin precision
        b"ETH chainx".to_vec(),
    )
    .unwrap();

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
