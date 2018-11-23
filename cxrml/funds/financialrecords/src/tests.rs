// Copyright 2018 Chainpool.

use substrate_primitives::{Blake2Hasher, H256};

use runtime_io;
use runtime_io::with_externalities;
use runtime_primitives::testing::{Digest, DigestItem, Header};
use runtime_primitives::traits::BlakeTwo256;
use runtime_primitives::BuildStorage;

use super::*;
use tokenbalances::{DescString, SymbolString, Token};

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

impl tokenbalances::Trait for Test {
    const CHAINX_SYMBOL: SymbolString = b"pcx";
    const CHAINX_TOKEN_DESC: DescString = b"this is pcx for mock";
    type TokenBalance = TokenBalance;
    type Event = ();
    type OnMoveToken = ();
}

impl Trait for Test {
    type Event = ();
    type OnDepositToken = ();
    type OnWithdrawToken = ();
}

// This function basically just builds a genesis storage key/value store according to
// our desired mockup.
pub fn new_test_ext() -> runtime_io::TestExternalities<Blake2Hasher> {
    let mut r = system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap();
    // balance
    r.extend(
        balances::GenesisConfig::<Test> {
            balances: vec![(1, 1000), (2, 510)],
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
    // token balance
    let t: Token = Token::new(b"x-btc".to_vec(), b"btc token".to_vec(), 8);
    let t2: Token = Token::new(b"x-eth".to_vec(), b"eth token".to_vec(), 4);

    r.extend(
        tokenbalances::GenesisConfig::<Test> {
            chainx_precision: 8,
            token_list: vec![(t, vec![]), (t2, vec![])],
            transfer_token_fee: 10,
        }
        .build_storage()
        .unwrap(),
    );
    // financialrecords
    r.extend(
        GenesisConfig::<Test> { withdrawal_fee: 10 }
            .build_storage()
            .unwrap(),
    );
    r.into()
}

pub fn new_test_ext2() -> runtime_io::TestExternalities<Blake2Hasher> {
    let mut r = system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap();
    // balance
    r.extend(
        balances::GenesisConfig::<Test> {
            balances: vec![(1, 1000), (2, 510)],
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
    // token balance
    let t: Token = Token::new(b"x-btc".to_vec(), b"btc token".to_vec(), 8);
    let t2: Token = Token::new(b"x-eth".to_vec(), b"eth token".to_vec(), 4);

    r.extend(
        tokenbalances::GenesisConfig::<Test> {
            chainx_precision: 8,
            token_list: vec![(t, vec![]), (t2, vec![])],
            transfer_token_fee: 10,
        }
        .build_storage()
        .unwrap(),
    );
    // financialrecords
    r.extend(
        GenesisConfig::<Test> { withdrawal_fee: 10 }
            .build_storage()
            .unwrap(),
    );
    r.into()
}

type FinancialRecords = Module<Test>;
type TokenBalances = tokenbalances::Module<Test>;
//type Balances = balances::Module<Test>;

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
        let index =
            FinancialRecords::withdrawal_with_index(&a, &btc_symbol, 50, vec![], vec![]).unwrap();
        assert_eq!(index, 1);
        FinancialRecords::withdrawal_locking_with_index(&a, index).unwrap();
        assert_eq!(
            FinancialRecords::withdrawal_finish_with_index(&a, index, true),
            Ok(1)
        );
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
        FinancialRecords::withdrawal(&a, &btc_symbol, 50, vec![], vec![]).unwrap();
        FinancialRecords::withdrawal(&a, &eth_symbol, 50, vec![], vec![]).unwrap();

        assert_ok!(FinancialRecords::withdrawal_finish(&a, &btc_symbol, true));
        assert_ok!(FinancialRecords::withdrawal_finish(&a, &eth_symbol, false));

        assert_eq!(TokenBalances::total_token_of(&a, &btc_symbol), 50);
        assert_eq!(TokenBalances::total_token_of(&a, &eth_symbol), 100);

        assert_eq!(FinancialRecords::records_len_of(&a), 4);

        let key1 = (a, btc_symbol.clone());
        let key2 = (a, eth_symbol.clone());

        assert_eq!(FinancialRecords::last_deposit_index_of(&key1).unwrap(), 0);
        assert_eq!(
            FinancialRecords::last_withdrawal_index_of(&key1).unwrap(),
            2
        );
        assert_eq!(FinancialRecords::last_deposit_index_of(&key2).unwrap(), 1);
        assert_eq!(
            FinancialRecords::last_withdrawal_index_of(&key2).unwrap(),
            3
        );
    })
}

#[test]
fn test_last_not_finish() {
    with_externalities(&mut new_test_ext(), || {
        let a: u64 = 1; // accountid
        let btc_symbol = b"x-btc".to_vec();
        FinancialRecords::deposit_init(&a, &btc_symbol, 100).unwrap();
        assert_err!(
            FinancialRecords::withdrawal(&a, &btc_symbol, 50, vec![], vec![]),
            "the account has no deposit record for this token yet"
        );
        // let deposit fail
        assert_ok!(FinancialRecords::deposit_finish(&a, &btc_symbol, false)); // 1. deposit failed
        assert_eq!(TokenBalances::total_token_of(&a, &btc_symbol), 0);

        assert_err!(
            FinancialRecords::withdrawal(&a, &btc_symbol, 50, vec![], vec![]),
            "not a existed token in this account token list"
        );
        assert_eq!(FinancialRecords::records_len_of(&a), 1);

        FinancialRecords::deposit_init(&a, &btc_symbol, 100).unwrap();
        assert_ok!(FinancialRecords::deposit_finish(&a, &btc_symbol, true)); // 2. deposit success

        assert_ok!(FinancialRecords::withdrawal(
            &a,
            &btc_symbol,
            50,
            vec![],
            vec![]
        ));

        assert_ok!(FinancialRecords::deposit(&a, &btc_symbol, 50)); // 3. deposit success

        assert_eq!(TokenBalances::total_token_of(&a, &btc_symbol), 150);
        assert_eq!(
            TokenBalances::free_token(&(a.clone(), btc_symbol.clone())),
            100
        );

        assert_ok!(FinancialRecords::withdrawal_finish(&a, &btc_symbol, false)); // 4. withdrawal failed
        assert_eq!(
            TokenBalances::free_token(&(a.clone(), btc_symbol.clone())),
            150
        );

        assert_ok!(FinancialRecords::withdrawal(
            &a,
            &btc_symbol,
            25,
            vec![],
            vec![]
        ));
        assert_ok!(FinancialRecords::withdrawal_finish(&a, &btc_symbol, true)); // destroy token here 5. withdrawal success
        assert_eq!(FinancialRecords::records_len_of(&a), 5);
        assert_eq!(
            TokenBalances::free_token(&(a.clone(), btc_symbol.clone())),
            125
        );

        assert_eq!(TokenBalances::total_token(&btc_symbol), 125);
    })
}

#[test]
fn test_withdrawal_larger() {
    with_externalities(&mut new_test_ext(), || {
        let a: u64 = 1; // accountid
        let btc_symbol = b"x-btc".to_vec();
        assert_ok!(FinancialRecords::deposit(&a, &btc_symbol, 10));

        assert_err!(
            FinancialRecords::withdrawal(&a, &btc_symbol, 50, vec![], vec![]),
            "not enough free token to withdraw"
        );
        assert_eq!(FinancialRecords::records_len_of(&a), 1);
    })
}

#[test]
fn test_withdrawal_first() {
    with_externalities(&mut new_test_ext(), || {
        let a: u64 = 1; // accountid
        let btc_symbol = b"x-btc".to_vec();
        assert_err!(
            FinancialRecords::withdrawal(&a, &btc_symbol, 50, vec![], vec![]),
            "the account has no deposit record for this token yet"
        );
    })
}

#[test]
fn test_multi_sym() {
    with_externalities(&mut new_test_ext(), || {
        let a: u64 = 1; // accountid
        let btc_symbol = b"x-btc".to_vec();
        let eth_symbol = b"x-eth".to_vec();
        let key1 = (a, btc_symbol.clone());
        let key2 = (a, eth_symbol.clone());

        assert_err!(
            FinancialRecords::withdrawal_finish(&a, &btc_symbol, true),
            "have not executed withdrawal() or withdrawal_init() yet for this record"
        );
        assert_err!(
            FinancialRecords::withdrawal(&a, &btc_symbol, 50, vec![], vec![]),
            "the account has no deposit record for this token yet"
        );

        assert_ok!(FinancialRecords::deposit(&a, &btc_symbol, 100)); // index = 0
        assert_ok!(FinancialRecords::deposit(&a, &eth_symbol, 100)); // eth 100 index = 1
        assert_ok!(FinancialRecords::deposit(&a, &btc_symbol, 100)); // btc 200 index = 2

        assert_eq!(FinancialRecords::last_deposit_index_of(&key1), Some(2));
        assert_eq!(FinancialRecords::last_deposit_index_of(&key2), Some(1));
        assert_eq!(FinancialRecords::last_withdrawal_index_of(&key2), None);

        assert_eq!(TokenBalances::total_token_of(&a, &btc_symbol), 200);
        assert_eq!(TokenBalances::total_token_of(&a, &eth_symbol), 100);

        // withdraw
        assert_ok!(FinancialRecords::withdrawal(
            &a,
            &btc_symbol,
            50,
            vec![],
            vec![]
        )); // index = 3
        assert_err!(FinancialRecords::withdrawal(&a, &btc_symbol, 25, vec![], vec![]), "the last action have not finished yet! only if the last deposit/withdrawal have finished you can do a new action.");

        assert_ok!(FinancialRecords::withdrawal(
            &a,
            &eth_symbol,
            50,
            vec![],
            vec![]
        )); // parallel withdraw  index = 4

        assert_eq!(
            TokenBalances::free_token(&(a.clone(), btc_symbol.clone())),
            150
        );
        assert_eq!(
            TokenBalances::free_token(&(a.clone(), eth_symbol.clone())),
            50
        );

        assert_ok!(FinancialRecords::deposit(&a, &eth_symbol, 50)); // deposit while withdraw  index = 5

        assert_eq!(
            TokenBalances::free_token(&(a.clone(), eth_symbol.clone())),
            100
        );

        assert_ok!(FinancialRecords::withdrawal_finish(&a, &btc_symbol, true));
        assert_ok!(FinancialRecords::withdrawal_finish(&a, &eth_symbol, false));

        assert_eq!(TokenBalances::total_token_of(&a, &btc_symbol), 150);
        assert_eq!(TokenBalances::total_token_of(&a, &eth_symbol), 150);

        assert_eq!(FinancialRecords::last_deposit_index_of(&key1), Some(2));
        assert_eq!(FinancialRecords::last_deposit_index_of(&key2), Some(5));

        assert_eq!(FinancialRecords::last_withdrawal_index_of(&key1), Some(3));
        assert_eq!(FinancialRecords::last_withdrawal_index_of(&key2), Some(4));

        assert_eq!(FinancialRecords::records_len_of(&a), 6);
    })
}

#[test]
fn test_withdraw_log_cache() {
    with_externalities(&mut new_test_ext2(), || {
        // issue
        let a: u64 = 1; // accountid
        let b: u64 = 2; // accountid
        let btc_symbol = b"x-btc".to_vec();
        let eth_symbol = b"x-eth".to_vec();
        // let key_a_btc = (a, btc_symbol.clone());
        // let key_a_eth = (a, eth_symbol.clone());
        // let key_b_btc = (b, btc_symbol.clone());
        // let key_b_eth = (b, eth_symbol.clone());

        FinancialRecords::deposit(&a, &btc_symbol, 1000).unwrap();
        FinancialRecords::deposit(&b, &btc_symbol, 1000).unwrap();
        FinancialRecords::deposit(&a, &eth_symbol, 1000).unwrap();
        FinancialRecords::deposit(&b, &eth_symbol, 1000).unwrap();

        assert_eq!(FinancialRecords::records_len_of(&a), 2);
        assert_eq!(FinancialRecords::records_len_of(&b), 2);

        // withdraw
        FinancialRecords::withdrawal(&a, &btc_symbol, 100, vec![], vec![]).unwrap();
        assert_eq!(FinancialRecords::records_len_of(&a), 3);
        FinancialRecords::withdrawal(&b, &btc_symbol, 100, vec![], vec![]).unwrap();
        assert_eq!(FinancialRecords::records_len_of(&a), 3);

        let log_a = FinancialRecords::withdraw_log_cache((a, 2)).unwrap();
        assert_eq!(log_a.prev(), None);
        assert_eq!(log_a.next(), Some((2, 2)));
        let log_b = FinancialRecords::withdraw_log_cache((b, 2)).unwrap();
        assert_eq!(log_b.prev(), Some((1, 2)));
        assert_eq!(log_b.next(), None);

        FinancialRecords::withdrawal(&a, &eth_symbol, 100, vec![], vec![]).unwrap();
        FinancialRecords::withdrawal(&b, &eth_symbol, 100, vec![], vec![]).unwrap();

        // btc cache
        if let Some(btc_header) = FinancialRecords::log_header_for(&btc_symbol) {
            let mut index = btc_header.index();
            let mut v = vec![];
            while let Some(node) = FinancialRecords::withdraw_log_cache(&index) {
                v.push((node.data.accountid(), node.data.index()));
                if let Some(next) = node.next() {
                    index = next;
                } else {
                    break;
                }
            }
            assert_eq!(v.as_slice(), [(a, 2), (b, 2)]);
        } else {
            panic!("unreachable!")
        }
        // eth cache
        if let Some(eth_header) = FinancialRecords::log_header_for(&eth_symbol) {
            let mut index = eth_header.index();
            let mut v = vec![];
            while let Some(node) = FinancialRecords::withdraw_log_cache(&index) {
                v.push((node.data.accountid(), node.data.index()));
                if let Some(next) = node.next() {
                    index = next;
                } else {
                    break;
                }
            }
            assert_eq!(v.as_slice(), [(a, 3), (b, 3)]);
        } else {
            panic!("unreachable!")
        }

        // withdraw finish
        // loop linked node collection and find out

        // for example
        // loop cache and withdraw for b finish
        // btc relay withdraw finish
        assert_ok!(FinancialRecords::withdrawal_finish(&b, &btc_symbol, true));

        if let Some(btc_header) = FinancialRecords::log_header_for(&btc_symbol) {
            let mut index = btc_header.index();
            let mut v = vec![];
            while let Some(node) = FinancialRecords::withdraw_log_cache(&index) {
                v.push((node.data.accountid(), node.data.index()));
                if let Some(next) = node.next() {
                    index = next;
                } else {
                    break;
                }
            }
            assert_eq!(v.as_slice(), [(a, 2)]);
        } else {
            panic!("unreachable!")
        }

        let log = FinancialRecords::withdraw_log_cache((a, 2)).unwrap();
        assert_eq!(log.prev(), None);
        assert_eq!(log.next(), None);

        // btc relay withdraw err
        assert_ok!(FinancialRecords::withdrawal_finish(&a, &btc_symbol, false));
        // all cache removed
        assert_eq!(FinancialRecords::log_header_for(&btc_symbol) == None, true);

        // eth relay withdraw
        assert_ok!(FinancialRecords::withdrawal_finish(&a, &eth_symbol, true));
        if let Some(eth_header) = FinancialRecords::log_header_for(&eth_symbol) {
            let mut index = eth_header.index();
            let mut v = vec![];
            while let Some(node) = FinancialRecords::withdraw_log_cache(&index) {
                v.push((node.data.accountid(), node.data.index()));
                if let Some(next) = node.next() {
                    index = next;
                } else {
                    break;
                }
            }
            assert_eq!(v.as_slice(), [(b, 3)]);
        } else {
            panic!("unreachable!")
        }

        assert_ok!(FinancialRecords::withdrawal_finish(&b, &eth_symbol, true));
        assert_eq!(FinancialRecords::log_header_for(&btc_symbol) == None, true);
    })
}
