// Copyright 2018 Chainpool.

use runtime_io::with_externalities;
use mock::*;
use super::*;

#[test]
fn test_genesis() {
    with_externalities(&mut new_test_ext(), || {
        // Check that GenesisBuilder works properly.
        // check token_list
        let btc_symbol = u8_to_symbol(b"x-btc");
        let eth_symbol = u8_to_symbol(b"x-eth");

        assert_eq!(TokenBalances::token_list(), vec![
            btc_symbol,
            eth_symbol,
        ]);

        assert_eq!(TokenBalances::token_info(btc_symbol).precision(), 8);
        assert_eq!(TokenBalances::token_info(eth_symbol).precision(), 4);

        assert_eq!(TokenBalances::total_free_token(btc_symbol), 100);
        assert_eq!(TokenBalances::total_locked_token(btc_symbol), 0);
    });
}

#[test]
fn test_register() {
    with_externalities(&mut new_test_ext(), || {
        let t_sym: Symbol = u8_to_symbol(b"x-eos");
        let t_desc: TokenDesc = u8_to_token_desc(b"eos token");
        let precision = 4;
        let t: TokenT<Test> = Token::new(t_sym, t_desc, precision);
        assert_eq!(TokenBalances::register_token(t, 0, 0), Ok(()));

        assert_eq!(TokenBalances::token_list_len(), 3);
        assert_eq!(TokenBalances::token_list_map(2), (true, t_sym));

        let btc_symbol = u8_to_symbol(b"x-btc");
        let eth_symbol = u8_to_symbol(b"x-eth");
        assert_eq!(TokenBalances::token_list(), vec![
            btc_symbol,
            eth_symbol,
            t_sym,
        ]);

        assert_eq!(TokenBalances::total_free_token(t_sym), 0);
        assert_eq!(TokenBalances::token_info(t_sym).precision(), 4);

        // test err branch
        let btc_t = Token::new(btc_symbol, u8_to_token_desc(b"btc token"), 4);
        assert_noop!(TokenBalances::register_token(btc_t, 0, 0), "already has this token symbol");
        assert_eq!(TokenBalances::token_list_len(), 3);
        assert_eq!(TokenBalances::token_list_map(3), (false, u8_to_symbol(b"")));
    })
}


#[test]
fn test_remove() {
    with_externalities(&mut new_test_ext(), || {
        // register a new token
        let t_sym: Symbol = u8_to_symbol(b"x-eos");
        let t_desc: TokenDesc = u8_to_token_desc(b"eos token");
        let precision = 4;
        let t: TokenT<Test> = Token::new(t_sym, t_desc, precision);
        assert_eq!(TokenBalances::register_token(t.clone(), 0, 0), Ok(()));
        assert_eq!(TokenBalances::token_list_map(2), (true, t_sym));

        // remove it
        assert_eq!(TokenBalances::cancel_token(&t_sym), Ok(()));
        assert_eq!(TokenBalances::token_list_map(2), (false, t_sym));
        assert_eq!(TokenBalances::token_list_len(), 3); // length not modify

        assert_noop!(TokenBalances::cancel_token(&t_sym), "this token symbol dose not register yet or is invalid");
        // re-register, but must be failed
        assert_noop!(TokenBalances::register_token(t.clone(), 0, 0), "already has this token symbol");

        // create new token symbol
        let t_new: TokenT<Test> = Token { symbol: u8_to_symbol(b"x-eos2"), ..t };
        assert_eq!(TokenBalances::register_token(t_new.clone(), 0, 0), Ok(()));
        assert_eq!(TokenBalances::token_list_map(2), (false, t_sym));
        assert_eq!(TokenBalances::token_list_map(3), (true, t_new.symbol));
        assert_eq!(TokenBalances::token_list_len(), 4);
    })
}

#[test]
fn test_total_balance() {
    with_externalities(&mut new_test_ext(), || {
        let btc_symbol = u8_to_symbol(b"x-btc");
        assert_eq!(TokenBalances::total_token(&btc_symbol), 100);

        TokenBalances::increase_total_free_token_by(&btc_symbol, 100).unwrap();
        assert_eq!(TokenBalances::total_token(&btc_symbol), 200);

        TokenBalances::increase_total_locked_token_by(&btc_symbol, 50).unwrap();
        assert_eq!(TokenBalances::total_token(&btc_symbol), 250);

        TokenBalances::decrease_total_locked_token_by(&btc_symbol, 25).unwrap();
        TokenBalances::decrease_total_free_token_by(&btc_symbol, 15).unwrap();
        assert_eq!(TokenBalances::total_token(&btc_symbol), 210);
    })
}

#[test]
fn test_account_balance() {
    with_externalities(&mut new_test_ext(), || {
        let a: u64 = 1; // accountid
        let btc_symbol = u8_to_symbol(b"x-btc");
        assert_eq!(TokenBalances::free_token_of(&a, &btc_symbol), 0);
        assert_eq!(TokenBalances::locked_token_of(&a, &btc_symbol), 0);
        assert_eq!(TokenBalances::total_token_of(&a, &btc_symbol), 0);

        TokenBalances::increase_account_free_token_by(&a, &btc_symbol, 100).unwrap();
        assert_eq!(TokenBalances::total_token_of(&a, &btc_symbol), 100);

        TokenBalances::decrease_account_free_token_by(&a, &btc_symbol, 50).unwrap();
        assert_eq!(TokenBalances::total_token_of(&a, &btc_symbol), 50);
    })
}

#[test]
fn test_normal_deposit_and_withdraw() {
    with_externalities(&mut new_test_ext(), || {
        let a: u64 = 1; // accountid
        let btc_symbol = u8_to_symbol(b"x-btc");

        // deposit
        TokenBalances::deposit(&a, &btc_symbol, 50).unwrap();
        assert_eq!(TokenBalances::total_token_of(&a, &btc_symbol), 50);
        assert_eq!(TokenBalances::total_token(&btc_symbol), 150);

        // lock
        TokenBalances::lock_withdraw_token(&a, &btc_symbol, 25).unwrap();
        assert_eq!(TokenBalances::locked_token_of(&a, &btc_symbol), 25);
        assert_eq!(TokenBalances::free_token_of(&a, &btc_symbol), 25);
        assert_eq!(TokenBalances::total_token_of(&a, &btc_symbol), 50);
        assert_eq!(TokenBalances::total_locked_token(&btc_symbol), 25);

        // withdraw
        TokenBalances::withdraw(&a, &btc_symbol, 25).unwrap();
        assert_eq!(TokenBalances::locked_token_of(&a, &btc_symbol), 0);
        assert_eq!(TokenBalances::free_token_of(&a, &btc_symbol), 25);
        assert_eq!(TokenBalances::total_token_of(&a, &btc_symbol), 25);
        assert_eq!(TokenBalances::total_locked_token(&btc_symbol), 0);
        assert_eq!(TokenBalances::total_token(&btc_symbol), 125);
    })
}

#[test]
fn test_unlock_deposit_and_withdraw2() {
    with_externalities(&mut new_test_ext(), || {
        let a: u64 = 1; // accountid
        let btc_symbol = u8_to_symbol(b"x-btc");

        // deposit
        TokenBalances::deposit(&a, &btc_symbol, 50).unwrap();
        assert_eq!(TokenBalances::total_token_of(&a, &btc_symbol), 50);
        assert_eq!(TokenBalances::total_token(&btc_symbol), 150);

        // lock
        TokenBalances::lock_withdraw_token(&a, &btc_symbol, 25).unwrap();
        assert_eq!(TokenBalances::locked_token_of(&a, &btc_symbol), 25);
        assert_eq!(TokenBalances::free_token_of(&a, &btc_symbol), 25);
        assert_eq!(TokenBalances::total_token_of(&a, &btc_symbol), 50);
        assert_eq!(TokenBalances::total_locked_token(&btc_symbol), 25);

        // unlock
        TokenBalances::unlock_withdraw_token(&a, &btc_symbol, 10).unwrap();
        assert_eq!(TokenBalances::locked_token_of(&a, &btc_symbol), 15);
        assert_eq!(TokenBalances::free_token_of(&a, &btc_symbol), 35);
        assert_eq!(TokenBalances::total_token_of(&a, &btc_symbol), 50);
        assert_eq!(TokenBalances::total_locked_token(&btc_symbol), 15);
    })
}

#[test]
fn test_error_deposit_and_withdraw1() {
    with_externalities(&mut new_test_ext(), || {
        let a: u64 = 1; // accountid
        let btc_symbol = u8_to_symbol(b"x-btc");
        // deposit
        TokenBalances::deposit(&a, &btc_symbol, 50).unwrap();
        assert_eq!(TokenBalances::total_token_of(&a, &btc_symbol), 50);
        assert_eq!(TokenBalances::total_token(&btc_symbol), 150);
        // withdraw first
        // withdraw
        assert_err!(TokenBalances::withdraw(&a, &btc_symbol, 25), "not enough locked token to withdraw");
        // lock
        assert_eq!(TokenBalances::total_free_token(&btc_symbol), 150);
        assert_err!(TokenBalances::lock_withdraw_token(&a, &btc_symbol, 100), "not enough free token to lock for this account");
        // lock first
        assert_ok!(TokenBalances::lock_withdraw_token(&a, &btc_symbol, 25));
        // withdraw
        assert_ok!(TokenBalances::withdraw(&a, &btc_symbol, 25));
    })
}


#[test]
fn test_error_deposit_and_withdraw2() {
    with_externalities(&mut new_test_ext(), || {
        let a: u64 = 1; // accountid
        let btc_symbol = u8_to_symbol(b"x-btc");
        // deposit
        TokenBalances::deposit(&a, &btc_symbol, 50).unwrap();
        assert_eq!(TokenBalances::total_token_of(&a, &btc_symbol), 50);
        assert_eq!(TokenBalances::total_token(&btc_symbol), 150);
        // overflow
        let i: i32 = -1;
        assert_err!(TokenBalances::lock_withdraw_token(&a, &btc_symbol, i as TokenBalance), "not enough free token to lock");
        assert_err!(TokenBalances::deposit(&a, &btc_symbol, i as TokenBalance), "Overflow in increase_total_free_token_by");
    })
}

#[test]
fn test_error_deposit_and_withdraw3() {
    with_externalities(&mut new_test_ext(), || {
        let a: u64 = 1; // accountid
        let btc_symbol = u8_to_symbol(b"x-btc");
        // lock or withdraw without init
        assert_err!(TokenBalances::withdraw(&a, &btc_symbol, 25), "not a existed token in this account token list");
        assert_err!(TokenBalances::lock_withdraw_token(&a, &btc_symbol, 25), "not a existed token in this account token list");
        TokenBalances::deposit(&a, &btc_symbol, 0).unwrap();
        assert_err!(TokenBalances::withdraw(&a, &btc_symbol, 25), "not enough locked token to withdraw");
        assert_err!(TokenBalances::lock_withdraw_token(&a, &btc_symbol, 25), "not enough free token to lock for this account");

        TokenBalances::deposit(&a, &btc_symbol, 100).unwrap();
        assert_ok!(TokenBalances::lock_withdraw_token(&a, &btc_symbol, 25));
        assert_ok!(TokenBalances::withdraw(&a, &btc_symbol, 25));
    })
}

#[test]
fn test_transfer() {
    with_externalities(&mut new_test_ext(), || {
        let a: u64 = 1; // accountid
        let b: u64 = 2; // accountid
        let btc_symbol = u8_to_symbol(b"x-btc");
        // deposit 50 to account 1
        TokenBalances::deposit(&a, &btc_symbol, 50).unwrap();
        // transfer
        TokenBalances::transfer_token(Some(a).into(), b.into(), btc_symbol.clone(), 25).unwrap();
        // sum not change
        assert_eq!(TokenBalances::total_free_token(&btc_symbol), 150);
        assert_eq!(TokenBalances::total_token_of(&a, &btc_symbol), 25);
        assert_eq!(TokenBalances::free_token_of(&b, &btc_symbol), 25);
        assert_eq!(Balances::free_balance(&a), 990);

        assert_err!(TokenBalances::transfer_token(Some(a).into(), b.into(), btc_symbol.clone(), 50), "transactor's free token balance too low, can't transfer this token")
    })
}

#[test]
fn test_transfer_to_self() {
    with_externalities(&mut new_test_ext(), || {
        let a: u64 = 1; // accountid
        let btc_symbol = u8_to_symbol(b"x-btc");
        // deposit 50 to account 1
        TokenBalances::deposit(&a, &btc_symbol, 50).unwrap();
        // transfer
        TokenBalances::transfer_token(Some(a).into(), a.into(), btc_symbol.clone(), 25).unwrap();
        // sum not change
        assert_eq!(TokenBalances::total_free_token(&btc_symbol), 150);
        assert_eq!(TokenBalances::total_token_of(&a, &btc_symbol), 50);
        assert_eq!(Balances::free_balance(&a), 990);
    })
}

#[test]
fn test_transfer_err() {
    with_externalities(&mut new_test_ext(), || {
        let a: u64 = 1; // accountid
        let b: u64 = 2; // accountid
        let btc_symbol = u8_to_symbol(b"x-btc");
        // deposit 50 to account 2
        TokenBalances::deposit(&b, &btc_symbol, 50).unwrap();
        // transfer
        TokenBalances::transfer_token(Some(b).into(), a.into(), btc_symbol.clone(), 25).unwrap();
        // sum not change
        assert_eq!(TokenBalances::total_free_token(&btc_symbol), 150);
        assert_eq!(TokenBalances::free_token_of(&b, &btc_symbol), 25);
        assert_eq!(TokenBalances::total_token_of(&a, &btc_symbol), 25);
        assert_eq!(Balances::free_balance(&b), 500);

        assert_err!(TokenBalances::transfer_token(Some(b).into(), a.into(), btc_symbol.clone(), 1),
            "chainx balance is not enough after this tx, not allow to be killed at here");
        assert_eq!(Balances::free_balance(&b), 500);
    })
}