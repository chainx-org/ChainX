// Copyright 2018-2019 Chainpool.

#![cfg(test)]

use super::*;

use primitives::testing::{Digest, DigestItem, Header};
use primitives::traits::BlakeTwo256;
use primitives::{BuildStorage, StorageOverlay};
use substrate_primitives::{Blake2Hasher, H256};
use support::impl_outer_origin;

impl_outer_origin! {
    pub enum Origin for Test {}
}

#[derive(Clone, Eq, PartialEq, Debug)]
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

pub type Balance = u64;
impl Trait for Test {
    /// Event
    type Balance = Balance;
    type OnNewAccount = Indices;
    type Event = ();
    type OnAssetChanged = ();
    type OnAssetRegisterOrRevoke = ();
}

pub type Indices = indices::Module<Test>;
pub type XAssets = Module<Test>;

// This function basically just builds a genesis storage key/value store according to
// our desired mockup.
pub fn new_test_ext() -> runtime_io::TestExternalities<Blake2Hasher> {
    let mut r = system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap()
        .0;

    r.extend(
        GenesisConfig::<Test> {
            memo_len: 128,
            _genesis_phantom_data: Default::default(),
        }
        .build_storage()
        .unwrap()
        .0,
    );
    let mut init: runtime_io::TestExternalities<Blake2Hasher> = r.into();
    let pcx_token_name = b"PolkadotChainX".to_vec();
    let pcx_desc = b"PCX onchain token".to_vec();
    let pcx_precision = 8;
    runtime_io::with_externalities(&mut init, || {
        // xassets
        let chainx: Token = <XAssets as ChainT>::TOKEN.to_vec();

        let pcx = Asset::new(
            chainx.clone(),
            pcx_token_name,
            Chain::ChainX,
            pcx_precision,
            pcx_desc,
        )
        .unwrap();

        let btc = Asset::new(
            b"BTC".to_vec(), // token
            b"X-BTC".to_vec(),
            Chain::Bitcoin,
            8, // bitcoin precision
            b"ChainX's Cross-chain Bitcoin".to_vec(),
        )
        .unwrap();

        XAssets::bootstrap_register_asset(pcx, true, false).unwrap();
        XAssets::bootstrap_register_asset(btc.clone(), true, true).unwrap();
        XAssets::pcx_issue(&1, 1000).unwrap();
        XAssets::pcx_issue(&2, 510).unwrap();
        XAssets::pcx_issue(&3, 1000).unwrap();
        XAssets::issue(&btc.token(), &3, 100).unwrap();
    });
    let init: StorageOverlay = init.into();
    runtime_io::TestExternalities::new(init)
}
