// Copyright 2018 Chainpool.

use substrate_primitives::{H256, Blake2Hasher, RlpCodec};

use primitives::BuildStorage;
use primitives::traits::BlakeTwo256;
use primitives::testing::{Digest, DigestItem, Header};
use runtime_io;

use {GenesisConfig, Module, Trait, system, balances, cxsupport, Token};

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

impl cxsupport::Trait for Test {}

// define tokenbalances module type
pub type TokenBalance = u128;
pub type Precision = u32;

impl Trait for Test {
    type TokenBalance = TokenBalance;
    type Precision = Precision;
    type Event = ();
}

pub type TestPrecision = <Test as Trait>::Precision;

pub type TokenBalances = Module<Test>;
pub type Balances = balances::Module<Test>;

// This function basically just builds a genesis storage key/value store according to
// our desired mockup.
pub fn new_test_ext() -> runtime_io::TestExternalities<Blake2Hasher, RlpCodec> {
    let mut r = system::GenesisConfig::<Test>::default().build_storage().unwrap();
    // balance
    r.extend(balances::GenesisConfig::<Test> {
        balances: vec![(1, 1000), (2, 510)],
        transaction_base_fee: 0,
        transaction_byte_fee: 0,
        existential_deposit: 500,
        transfer_fee: 0,
        creation_fee: 0,
        reclaim_rebate: 0,
    }.build_storage().unwrap());
    // token
    let t: Token<TestPrecision> = Token::new(b"x-btc".to_vec(), b"btc token".to_vec(), 8);
    let t2: Token<TestPrecision> = Token::new(b"x-eth".to_vec(), b"eth token".to_vec(), 4);


    r.extend(GenesisConfig::<Test> {
        token_list: vec![
            (t, 100, 0),
            (t2, 100, 0),
        ],
        transfer_token_fee: 10,
    }.build_storage().unwrap());
    r.into()
}