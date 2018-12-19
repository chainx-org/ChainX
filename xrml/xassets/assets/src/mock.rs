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

impl cxsystem::Trait for Test {}

impl associations::Trait for Test {
    type OnCalcFee = cxsupport::Module<Test>;
    type Event = ();
}

impl cxsupport::Trait for Test {}

// define tokenbalances module type
pub type TokenBalance = u128;

impl Trait for Test {
    const CHAINX_SYMBOL: SymbolString = b"pcx";
    const CHAINX_TOKEN_DESC: DescString = b"this is pcx for mock";
    type TokenBalance = TokenBalance;
    type Event = ();
    type OnMoveToken = ();
}

pub type TokenBalances = Module<Test>;
pub type Balances = balances::Module<Test>;

// This function basically just builds a genesis storage key/value store according to
// our desired mockup.
pub fn new_test_ext() -> runtime_io::TestExternalities<Blake2Hasher> {
    let mut r = system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap();
    // balance
<<<<<<< HEAD
    r.extend(balances::GenesisConfig::<Test> {
        balances: vec![(1, 1000), (2, 510), (3, 1000)],
        transaction_base_fee: 0,
        transaction_byte_fee: 0,
        existential_deposit: 500,
        transfer_fee: 0,
        creation_fee: 0,
        reclaim_rebate: 0,
    }.build_storage().unwrap());
=======
    r.extend(
        balances::GenesisConfig::<Test> {
            balances: vec![(1, 1000), (2, 510), (3, 1000)],
            transaction_base_fee: 0,
            transaction_byte_fee: 0,
            existential_deposit: 500,
            transfer_fee: 0,
            creation_fee: 0,
            reclaim_rebate: 0,
        }
        .build_storage()
        .unwrap(),
    );
>>>>>>> develop
    // token
    let t: Token = Token::new(b"x-btc".to_vec(), b"btc token".to_vec(), 8);
    let t2: Token = Token::new(b"x-eth".to_vec(), b"eth token".to_vec(), 4);

<<<<<<< HEAD
    r.extend(GenesisConfig::<Test> {
        token_list: vec![
            (t, [(3, 100)].to_vec()),
            (t2, [(3, 100)].to_vec()),
        ],
        transfer_token_fee: 10,
    }.build_storage().unwrap());
=======
    r.extend(
        GenesisConfig::<Test> {
            chainx_precision: 8,
            token_list: vec![(t, [(3, 100)].to_vec()), (t2, [(3, 100)].to_vec())],
            transfer_token_fee: 10,
        }
        .build_storage()
        .unwrap(),
    );
>>>>>>> develop
    r.into()
}

pub fn err_test_ext() -> runtime_io::TestExternalities<Blake2Hasher> {
    let mut r = system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap();
    // balance
<<<<<<< HEAD
    r.extend(balances::GenesisConfig::<Test> {
        balances: vec![(1, 1000), (2, 510), (3, 1000)],
        transaction_base_fee: 0,
        transaction_byte_fee: 0,
        existential_deposit: 500,
        transfer_fee: 0,
        creation_fee: 0,
        reclaim_rebate: 0,
    }.build_storage().unwrap());
=======
    r.extend(
        balances::GenesisConfig::<Test> {
            balances: vec![(1, 1000), (2, 510), (3, 1000)],
            transaction_base_fee: 0,
            transaction_byte_fee: 0,
            existential_deposit: 500,
            transfer_fee: 0,
            creation_fee: 0,
            reclaim_rebate: 0,
        }
        .build_storage()
        .unwrap(),
    );
>>>>>>> develop
    // token
    let t: Token = Token::new(b"x-btc?...".to_vec(), b"btc token".to_vec(), 8);
    let t2: Token = Token::new(b"x-eth".to_vec(), b"eth token".to_vec(), 4);

<<<<<<< HEAD
    r.extend(GenesisConfig::<Test> {
        token_list: vec![
            (t, [(3, 100)].to_vec()),
            (t2, [(3, 100)].to_vec()),
        ],
        transfer_token_fee: 10,
    }.build_storage().unwrap());
=======
    r.extend(
        GenesisConfig::<Test> {
            chainx_precision: 8,
            token_list: vec![(t, [(3, 100)].to_vec()), (t2, [(3, 100)].to_vec())],
            transfer_token_fee: 10,
        }
        .build_storage()
        .unwrap(),
    );
>>>>>>> develop
    r.into()
}

pub fn new_test_ext2() -> runtime_io::TestExternalities<Blake2Hasher> {
    let mut r = system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap();
    // balance
<<<<<<< HEAD
    r.extend(balances::GenesisConfig::<Test> {
        balances: vec![(1, 1000), (2, 510), (3, 1000)],
        transaction_base_fee: 0,
        transaction_byte_fee: 0,
        existential_deposit: 0,
        transfer_fee: 0,
        creation_fee: 0,
        reclaim_rebate: 0,
    }.build_storage().unwrap());
=======
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
        .unwrap(),
    );
>>>>>>> develop
    // token
    let t: Token = Token::new(b"x-btc".to_vec(), b"btc token".to_vec(), 8);
    let t2: Token = Token::new(b"x-eth".to_vec(), b"eth token".to_vec(), 4);

<<<<<<< HEAD
    r.extend(GenesisConfig::<Test> {
        token_list: vec![
            (t, [(3, 100)].to_vec()),
            (t2, [(3, 100)].to_vec()),
        ],
        transfer_token_fee: 10,
    }.build_storage().unwrap());
=======
    r.extend(
        GenesisConfig::<Test> {
            chainx_precision: 8,
            token_list: vec![(t, [(3, 100)].to_vec()), (t2, [(3, 100)].to_vec())],
            transfer_token_fee: 10,
        }
        .build_storage()
        .unwrap(),
    );
>>>>>>> develop
    r.into()
}
