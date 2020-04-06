// Copyright 2018-2019 Chainpool.

#![cfg(test)]

use super::*;

use primitives::testing::{Digest, DigestItem, Header, UintAuthorityId};
use primitives::traits::{BlakeTwo256, IdentityLookup};
use primitives::{BuildStorage, StorageOverlay};
use runtime_io::with_externalities;
use substrate_primitives::{Blake2Hasher, H256};
use support::impl_outer_origin;

use xassets::Asset;

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

// assets
impl xassets::Trait for Test {
    type Balance = u64;
    type OnNewAccount = ();
    type Event = ();
    type OnAssetChanged = ();
    type OnAssetRegisterOrRevoke = ();
    type DetermineTokenJackpotAccountId = ();
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

    r.extend(
        xassets::GenesisConfig::<Test> {
            memo_len: 128,
            _genesis_phantom_data: Default::default(),
        }
        .build_storage()
        .unwrap()
        .0,
    );

    let mut init: runtime_io::TestExternalities<Blake2Hasher> = r.into();
    with_externalities(&mut init, || {
        let chainx: Token = <XAssets as ChainT>::TOKEN.to_vec();

        let pcx = Asset::new(
            chainx.clone(),
            b"PolkadotChainX".to_vec(),
            Chain::ChainX,
            8,
            b"PCX onchain token".to_vec(),
        )
        .unwrap();
        let btc = Asset::new(
            b"BTC".to_vec(),     // token
            b"Bitcoin".to_vec(), // token
            Chain::Bitcoin,
            8, // bitcoin precision
            b"BTC chainx".to_vec(),
        )
        .unwrap();

        let eth = Asset::new(
            b"ETH".to_vec(),      // token
            b"Ethereum".to_vec(), // token
            Chain::Ethereum,
            8, // bitcoin precision
            b"ETH chainx".to_vec(),
        )
        .unwrap();
        XAssets::bootstrap_register_asset(pcx, true, false).unwrap();
        XAssets::bootstrap_register_asset(btc, true, true).unwrap();
        XAssets::bootstrap_register_asset(eth, true, true).unwrap();
    });
    let init: StorageOverlay = init.into();
    runtime_io::TestExternalities::new(init)
}
