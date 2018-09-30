// Copyright 2018 Chainpool.

use substrate_primitives::{H256, Blake2Hasher, RlpCodec};

use runtime_primitives::BuildStorage;
use runtime_primitives::traits::BlakeTwo256;
use runtime_primitives::testing::{Digest, DigestItem, Header};
use runtime_io;
use runtime_io::with_externalities;

use super::*;
use tokenbalances::{Token, TokenT};
use tokenbalances::utils::*;

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

// define tokenbalances module type
pub type Symbol = [u8; 8];
pub type TokenDesc = [u8; 32];
pub type TokenBalance = u128;
pub type Precision = u32;

impl tokenbalances::Trait for Test {
    type TokenBalance = TokenBalance;
    type Precision = Precision;
    type TokenDesc = TokenDesc;
    type Symbol = Symbol;
    type Event = ();
}

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
    let t: TokenT<Test> = Token::new(slice_to_u8_8(b"x-btc"), slice_to_u8_32(b"btc token"), 8);
    let t2: TokenT<Test> = Token::new(slice_to_u8_8(b"x-eth"), slice_to_u8_32(b"eth token"), 4);

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
        let btc_symbol = slice_to_u8_8(b"x-btc");

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
        let btc_symbol = slice_to_u8_8(b"x-btc");

        // deposit
        FinancialRecords::deposit_init(&a, &btc_symbol, 100).unwrap();
        assert_ok!(FinancialRecords::deposit_finish(&a, true));
        // withdraw
        FinancialRecords::withdrawal(&a, &btc_symbol, 50).unwrap();

        assert_ok!(FinancialRecords::withdrawal_finish(&a, true));

        assert_eq!(TokenBalances::total_token_of(&a, &btc_symbol), 50);
    })
}

#[test]
fn test_last_not_finish() {
    with_externalities(&mut new_test_ext(), || {
        let a: u64 = 1; // accountid
        let btc_symbol = slice_to_u8_8(b"x-btc");
        FinancialRecords::deposit_init(&a, &btc_symbol, 100).unwrap();
        assert_err!(FinancialRecords::withdrawal(&a, &btc_symbol, 50),
            "the last action have not finished yet! only if the last deposit/withdrawal have finished you can do a new action.");
        // let deposit fail
        assert_ok!(FinancialRecords::deposit_finish(&a, false));
        assert_eq!(TokenBalances::total_token_of(&a, &btc_symbol), 0);
        assert_err!(FinancialRecords::withdrawal(&a, &btc_symbol, 50), "not a existed token in this account token list");
        assert_eq!(FinancialRecords::records_len_of(&a), 1);

        FinancialRecords::deposit_init(&a, &btc_symbol, 100).unwrap();
        assert_ok!(FinancialRecords::deposit_finish(&a, true));
        assert_ok!(FinancialRecords::withdrawal(&a, &btc_symbol, 50));
        assert_err!(FinancialRecords::deposit(&a, &btc_symbol, 50),
            "the last action have not finished yet! only if the last deposit/withdrawal have finished you can do a new action.");
        assert_eq!(TokenBalances::total_token_of(&a, &btc_symbol), 100);
        assert_eq!(TokenBalances::free_token_of(&a, &btc_symbol), 50);

        assert_ok!(FinancialRecords::withdrawal_finish(&a, false));
        assert_eq!(TokenBalances::free_token_of(&a, &btc_symbol), 100);

        assert_ok!(FinancialRecords::withdrawal(&a, &btc_symbol, 25));
        assert_ok!(FinancialRecords::withdrawal_finish(&a, true));  // destroy token here
        assert_eq!(FinancialRecords::records_len_of(&a), 4);
        assert_eq!(TokenBalances::free_token_of(&a, &btc_symbol), 75);

        assert_eq!(TokenBalances::total_token(&btc_symbol), 175);

        // 1. deposit failed 2. deposit success 3. withdrawal failed 4. withdrawal success
        // 10 + 10 + 10 + 10 = 40
        assert_eq!(Balances::free_balance(&a), 960);
    })
}

#[test]
fn test_withdrawal_larger() {
    with_externalities(&mut new_test_ext(), || {
        let a: u64 = 1; // accountid
        let btc_symbol = slice_to_u8_8(b"x-btc");
        assert_ok!(FinancialRecords::deposit(&a, &btc_symbol, 10));

        assert_err!(FinancialRecords::withdrawal(&a, &btc_symbol, 50), "not enough free token to withdraw");
        assert_eq!(FinancialRecords::records_len_of(&a), 1);
    })
}

#[test]
fn test_fee() {
    with_externalities(&mut new_test_ext(), || {
        let b: u64 = 2; // accountid
        let btc_symbol = slice_to_u8_8(b"x-btc");
        assert_ok!(FinancialRecords::deposit(&b, &btc_symbol, 100));

        assert_err!(FinancialRecords::withdrawal(&b, &btc_symbol, 50), "chainx balance is not enough after this tx, not allow to be killed at here");
        assert_eq!(FinancialRecords::records_len_of(&b), 1);
    })
}

#[test]
fn test_withdrawal_first() {
    with_externalities(&mut new_test_ext(), || {
        let a: u64 = 1; // accountid
        let btc_symbol = slice_to_u8_8(b"x-btc");
        assert_err!(FinancialRecords::withdrawal(&a, &btc_symbol, 50), "the account has not deposit record yet");
    })
}