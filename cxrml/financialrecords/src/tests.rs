// Copyright 2018 Chainpool.

use substrate_primitives::{H256, Blake2Hasher, RlpCodec};

use runtime_primitives::BuildStorage;
use runtime_primitives::traits::BlakeTwo256;
use runtime_primitives::testing::{Digest, DigestItem, Header};
use runtime_io;
use runtime_io::with_externalities;

use super::*;
use tokenbalances::Token;

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

impl tokenbalances::Trait for Test {
    type TokenBalance = TokenBalance;
    type Precision = Precision;
    type Event = ();
}

pub type TestPrecision = <Test as tokenbalances::Trait>::Precision;

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
    // token balance
    let t: Token<TestPrecision> = Token::new(b"x-btc".to_vec(), b"btc token".to_vec(), 8);
    let t2: Token<TestPrecision> = Token::new(b"x-eth".to_vec(), b"eth token".to_vec(), 4);

    r.extend(tokenbalances::GenesisConfig::<Test> {
        token_list: vec![
            (t, 100, 0),
            (t2, 100, 0),
        ],
        transfer_token_fee: 10,
    }.build_storage().unwrap());
    // financialrecords
    r.extend(GenesisConfig::<Test> {
        deposit_fee: 10,
        withdrawal_fee: 10,
    }.build_storage().unwrap());
    r.into()
}

impl Trait for Test {
    type Event = ();
}

type FinancialRecords = Module<Test>;
type TokenBalances = tokenbalances::Module<Test>;
type Balances = balances::Module<Test>;

#[test]
fn test_normal() {
    with_externalities(&mut new_test_ext(), || {
        let a: u64 = 1; // accountid
        let btc_symbol = b"x-btc".to_vec();

        // deposit
        let index = FinancialRecords::deposit_with_index(&a, &btc_symbol, 100).unwrap();
        assert_eq!(index, 0);
        FinancialRecords::deposit_finish_with_index(&a, index, true).unwrap();
        // withdraw
        let index = FinancialRecords::withdrawal_with_index(&a, &btc_symbol, 50).unwrap();
        assert_eq!(index, 1);
        FinancialRecords::withdrawal_locking_with_index(&a, index).unwrap();
        assert_eq!(FinancialRecords::withdrawal_finish_with_index(&a, index, true), Ok(1));
    })
}

#[test]
fn test_normal2() {
    with_externalities(&mut new_test_ext(), || {
        let a: u64 = 1; // accountid
        let btc_symbol = b"x-btc".to_vec();
        let eth_symbol = b"x-eth".to_vec();

        // deposit
        FinancialRecords::deposit_init(&a, &btc_symbol, 100).unwrap();
        assert_ok!(FinancialRecords::deposit_finish(&a, &btc_symbol, true));
        assert_ok!(FinancialRecords::deposit(&a, &eth_symbol, 100));

        assert_eq!(TokenBalances::total_token_of(&a, &btc_symbol), 100);
        assert_eq!(TokenBalances::total_token_of(&a, &eth_symbol), 100);

        // withdraw
        FinancialRecords::withdrawal(&a, &btc_symbol, 50).unwrap();
        FinancialRecords::withdrawal(&a, &eth_symbol, 50).unwrap();

        assert_ok!(FinancialRecords::withdrawal_finish(&a, &btc_symbol, true));
        assert_ok!(FinancialRecords::withdrawal_finish(&a, &eth_symbol, false));

        assert_eq!(TokenBalances::total_token_of(&a, &btc_symbol), 50);
        assert_eq!(TokenBalances::total_token_of(&a, &eth_symbol), 100);

        assert_eq!(FinancialRecords::records_len_of(&a), 4);

        assert_eq!(FinancialRecords::last_deposit_index_of(&a, &btc_symbol).unwrap(), 0);
        assert_eq!(FinancialRecords::last_withdrawal_index_of(&a, &btc_symbol).unwrap(), 2);
        assert_eq!(FinancialRecords::last_deposit_index_of(&a, &eth_symbol).unwrap(), 1);
        assert_eq!(FinancialRecords::last_withdrawal_index_of(&a, &eth_symbol).unwrap(), 3);

        assert_eq!(Balances::free_balance(&a), 960);
    })
}

#[test]
fn test_last_not_finish() {
    with_externalities(&mut new_test_ext(), || {
        let a: u64 = 1; // accountid
        let btc_symbol = b"x-btc".to_vec();
        FinancialRecords::deposit_init(&a, &btc_symbol, 100).unwrap();
        assert_err!(FinancialRecords::withdrawal(&a, &btc_symbol, 50),
            "the account has no deposit record for this token yet");
        // let deposit fail
        assert_ok!(FinancialRecords::deposit_finish(&a, &btc_symbol, false)); // 1. deposit failed
        assert_eq!(TokenBalances::total_token_of(&a, &btc_symbol), 0);

        assert_err!(FinancialRecords::withdrawal(&a, &btc_symbol, 50), "not a existed token in this account token list");
        assert_eq!(FinancialRecords::records_len_of(&a), 1);

        FinancialRecords::deposit_init(&a, &btc_symbol, 100).unwrap();
        assert_ok!(FinancialRecords::deposit_finish(&a, &btc_symbol, true));  // 2. deposit success

        assert_ok!(FinancialRecords::withdrawal(&a, &btc_symbol, 50));

        assert_ok!(FinancialRecords::deposit(&a, &btc_symbol, 50));  // 3. deposit success

        assert_eq!(TokenBalances::total_token_of(&a, &btc_symbol), 150);
        assert_eq!(TokenBalances::free_token_of(&a, &btc_symbol), 100);

        assert_ok!(FinancialRecords::withdrawal_finish(&a, &btc_symbol, false)); // 4. withdrawal failed
        assert_eq!(TokenBalances::free_token_of(&a, &btc_symbol), 150);

        assert_ok!(FinancialRecords::withdrawal(&a, &btc_symbol, 25));
        assert_ok!(FinancialRecords::withdrawal_finish(&a, &btc_symbol, true));  // destroy token here 5. withdrawal success
        assert_eq!(FinancialRecords::records_len_of(&a), 5);
        assert_eq!(TokenBalances::free_token_of(&a, &btc_symbol), 125);

        assert_eq!(TokenBalances::total_token(&btc_symbol), 225);

        // 1. deposit failed 2. deposit success 3. deposit success 4. withdrawal failed 5. withdrawal success
        // 10 + 10 + 10 + 10 = 50
        assert_eq!(Balances::free_balance(&a), 950);
    })
}

#[test]
fn test_withdrawal_larger() {
    with_externalities(&mut new_test_ext(), || {
        let a: u64 = 1; // accountid
        let btc_symbol = b"x-btc".to_vec();
        assert_ok!(FinancialRecords::deposit(&a, &btc_symbol, 10));

        assert_err!(FinancialRecords::withdrawal(&a, &btc_symbol, 50), "not enough free token to withdraw");
        assert_eq!(FinancialRecords::records_len_of(&a), 1);
    })
}

#[test]
fn test_fee() {
    with_externalities(&mut new_test_ext(), || {
        let b: u64 = 2; // accountid
        let btc_symbol = b"x-btc".to_vec();
        assert_ok!(FinancialRecords::deposit(&b, &btc_symbol, 100));

        assert_err!(FinancialRecords::withdrawal(&b, &btc_symbol, 50), "chainx balance is not enough after this tx, not allow to be killed at here");
        assert_eq!(FinancialRecords::records_len_of(&b), 1);
    })
}

#[test]
fn test_withdrawal_first() {
    with_externalities(&mut new_test_ext(), || {
        let a: u64 = 1; // accountid
        let btc_symbol = b"x-btc".to_vec();
        assert_err!(FinancialRecords::withdrawal(&a, &btc_symbol, 50), "the account has no deposit record for this token yet");
    })
}

#[test]
fn test_multi_sym() {
    with_externalities(&mut new_test_ext(), || {
        let a: u64 = 1; // accountid
        let btc_symbol = b"x-btc".to_vec();
        let eth_symbol = b"x-eth".to_vec();


        assert_err!(FinancialRecords::withdrawal_finish(&a, &btc_symbol, true), "have not executed withdrawal() or withdrawal_init() yet for this record");
        assert_err!(FinancialRecords::withdrawal(&a, &btc_symbol, 50), "the account has no deposit record for this token yet");

        assert_ok!(FinancialRecords::deposit(&a, &btc_symbol, 100));  // index = 0
        assert_ok!(FinancialRecords::deposit(&a, &eth_symbol, 100));  // eth 100 index = 1
        assert_ok!(FinancialRecords::deposit(&a, &btc_symbol, 100));  // btc 200 index = 2

        assert_eq!(FinancialRecords::last_deposit_index_of(&a, &btc_symbol), Some(2));
        assert_eq!(FinancialRecords::last_deposit_index_of(&a, &eth_symbol), Some(1));
        assert_eq!(FinancialRecords::last_withdrawal_index_of(&a, &eth_symbol), None);

        assert_eq!(TokenBalances::total_token_of(&a, &btc_symbol), 200);
        assert_eq!(TokenBalances::total_token_of(&a, &eth_symbol), 100);

        // withdraw
        assert_ok!(FinancialRecords::withdrawal(&a, &btc_symbol, 50));  // index = 3
        assert_err!(FinancialRecords::withdrawal(&a, &btc_symbol, 25), "the last action have not finished yet! only if the last deposit/withdrawal have finished you can do a new action.");

        assert_ok!(FinancialRecords::withdrawal(&a, &eth_symbol, 50));  // parallel withdraw  index = 4

        assert_eq!(TokenBalances::free_token_of(&a, &btc_symbol), 150);
        assert_eq!(TokenBalances::free_token_of(&a, &eth_symbol), 50);

        assert_ok!(FinancialRecords::deposit(&a, &eth_symbol, 50));  // deposit while withdraw  index = 5

        assert_eq!(TokenBalances::free_token_of(&a, &eth_symbol), 100);

        assert_ok!(FinancialRecords::withdrawal_finish(&a, &btc_symbol, true));
        assert_ok!(FinancialRecords::withdrawal_finish(&a, &eth_symbol, false));

        assert_eq!(TokenBalances::total_token_of(&a, &btc_symbol), 150);
        assert_eq!(TokenBalances::total_token_of(&a, &eth_symbol), 150);

        assert_eq!(FinancialRecords::last_deposit_index_of(&a, &btc_symbol), Some(2));
        assert_eq!(FinancialRecords::last_deposit_index_of(&a, &eth_symbol), Some(5));

        assert_eq!(FinancialRecords::last_withdrawal_index_of(&a, &btc_symbol), Some(3));
        assert_eq!(FinancialRecords::last_withdrawal_index_of(&a, &eth_symbol), Some(4));


        assert_eq!(FinancialRecords::records_len_of(&a), 6);
    })
}