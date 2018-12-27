// Copyright 2018 Chainpool.

use substrate_primitives::{Blake2Hasher, H256};

use runtime_io;
use runtime_io::with_externalities;
use runtime_primitives::testing::{Digest, DigestItem, Header};
use runtime_primitives::traits::BlakeTwo256;
use runtime_primitives::BuildStorage;

use {balances, system, GenesisConfig, Module, Trait};

use super::*;
use assets::assetdef::{Asset, Chain, ChainT, Token};
use std::str;

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

impl xsystem::Trait for Test {
    const XSYSTEM_SET_POSITION: u32 = 3;
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
    let balance_factor = if ext_deposit > 0 { 256 } else { 1000000 };

    t.extend(
        balances::GenesisConfig::<Test> {
            balances: if monied {
                if reward > 0 {
                    vec![
                        (1, 10 * balance_factor),
                        (2, 20 * balance_factor),
                        (3, 30 * balance_factor),
                        (4, 40 * balance_factor),
                        (10, balance_factor),
                        (20, balance_factor),
                    ]
                } else {
                    vec![
                        (1, 10 * balance_factor),
                        (2, 20 * balance_factor),
                        (3, 30 * balance_factor),
                        (4, 40 * balance_factor),
                    ]
                }
            } else {
                vec![(10, balance_factor), (20, balance_factor)]
            },
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
        GenesisConfig::<Test> {
            order_fee: 10,
            pair_list: vec![],
            max_command_id: 0,
            average_price_len: 10000,
        }
        .build_storage()
        .unwrap()
        .0,
    );

    runtime_io::TestExternalities::new(t)
}

impl assets::Trait for Test {
    type Event = ();
    type OnAssetChanged = ();
}
impl xaccounts::Trait for Test {}
impl Trait for Test {
    type Event = ();
    type Amount = u128;
    type Price = u128;
}

pub type Pendingorders = Module<Test>;
pub type Assets = assets::Module<Test>;
pub type Balances = balances::Module<Test>;
