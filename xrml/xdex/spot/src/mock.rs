// Copyright 2018 Chainpool.
use substrate_primitives::{Blake2Hasher, H256};

use primitives::testing::{Digest, DigestItem, Header, UintAuthorityId};
use primitives::traits::BlakeTwo256;
use primitives::BuildStorage;
use runtime_io;

use super::*;
use std::str;
use xassets;
use xassets::assetdef::{Asset, Chain, ChainT, Token};

impl_outer_origin! {
    pub enum Origin for Test {}
}

#[derive(Clone, Eq, PartialEq)]
pub struct Test;

impl consensus::Trait for Test {
    const NOTE_OFFLINE_POSITION: u32 = 1;
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

impl timestamp::Trait for Test {
    const TIMESTAMP_SET_POSITION: u32 = 0;
    type Moment = u64;
    type OnTimestampSet = ();
}

impl xbitcoin::Trait for Test {
    type Event = ();
}

impl xaccounts::Trait for Test {
    type Event = ();
}

impl xassets::Trait for Test {
    type Event = ();
    type OnAssetChanged = ();
    type OnAssetRegisterOrRevoke = ();
}

impl xrecords::Trait for Test {
    type Event = ();
}

impl Trait for Test {
    type Event = ();
    type Price = u128;
}
// This function basically just builds a genesis storage key/value store according to
// our desired mockup.
pub fn new_test_ext(
    ext_deposit: u64,
    session_length: u64,
    sessions_per_era: u64,
    current_era: u64,
    monied: bool,
    reward: u64,
) -> runtime_io::TestExternalities<Blake2Hasher> {
    let mut t = system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap()
        .0;
    let balance_factor = if ext_deposit > 0 { 256 } else { 100000000 };

    let btc_asset = Asset::new(
        <xbitcoin::Module<Test> as ChainT>::TOKEN.to_vec(), // token
        b"Bitcoin".to_vec(),
        Chain::Bitcoin,
        8, // bitcoin precision
        b"BTC chainx".to_vec(),
    )
    .unwrap();
    let pcx_token_name = b"PolkadotChainX".to_vec();
    let pcx_precision = 3_u16;
    let pcx_desc = b"PCX onchain token".to_vec();

    t.extend(
        balances::GenesisConfig::<Test> {
            balances: vec![
                (1, 1_000_000_000),
                (2, 1_000_000_000),
                (3, 1_000_000_000),
                (4, 1_000_000_000),
            ],
            transaction_base_fee: 0,
            transaction_byte_fee: 0,
            existential_deposit: ext_deposit,
            transfer_fee: 0,
            creation_fee: 0,
            reclaim_rebate: 0,
        }
        .build_storage()
        .unwrap()
        .0,
    );
    t.extend(
        xassets::GenesisConfig::<Test> {
            pcx: (pcx_token_name, pcx_precision, pcx_desc),
            memo_len: 128,
            asset_list: vec![(
                btc_asset,
                true,
                vec![
                    (1, 1_000_000_000),
                    (2, 1_000_000_000),
                    (3, 1_000_000_000),
                    (4, 1_000_000_000),
                ],
            )],
        }
        .build_storage()
        .unwrap()
        .0,
    );
    t.extend(
        GenesisConfig::<Test> {
            pair_list: vec![(
                <xassets::Module<Test> as ChainT>::TOKEN.to_vec(),
                <xbitcoin::Module<Test> as ChainT>::TOKEN.to_vec(),
                5,
                true,
            )],
            // (OrderPair { first: Runtime::CHAINX_SYMBOL.to_vec(), second: BridgeOfBTC::SYMBOL.to_vec() }, 8)
            price_volatility: 10,
            _genesis_phantom_data: Default::default(),
        }
        .build_storage()
        .unwrap()
        .0,
    );

    runtime_io::TestExternalities::new(t)
}

pub type Spot = Module<Test>;
pub type Assets = xassets::Module<Test>;
pub type Balances = balances::Module<Test>;
