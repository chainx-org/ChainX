// Copyright 2018 Chainpool.

use runtime_io::with_externalities;
use mock::*;
use super::*;

#[test]
fn test_genesis() {
    with_externalities(&mut new_test_ext(), || {
        // Check that GenesisBuilder works properly.
        // check token_list
        let btc_symbol = b"x-btc".to_vec();
        let eth_symbol = b"x-eth".to_vec();

        assert_eq!(TokenBalances::token_list(), vec![
            Test::CHAINX_SYMBOL.to_vec(),
            btc_symbol.clone(),
            eth_symbol.clone(),
        ]);

        assert_eq!(TokenBalances::token_info(btc_symbol.clone()).unwrap().0.precision(), 8);
        assert_eq!(TokenBalances::token_info(eth_symbol.clone()).unwrap().0.precision(), 4);

        assert_eq!(TokenBalances::total_free_token(btc_symbol.clone()), 100);
        assert_eq!(TokenBalances::total_reserved_token(btc_symbol.clone()), 0);

        // chainx symbol for every user
        assert_eq!(TokenBalances::token_list_of(&0), [Test::CHAINX_SYMBOL.to_vec()].to_vec());
    });
}

#[test]
fn test_register() {
    with_externalities(&mut new_test_ext(), || {
        let t_sym: Symbol = b"x-eos".to_vec(); //slice_to_u8_8(b"x-eos");
        let t_desc: TokenDesc = b"eos token".to_vec(); //slice_to_u8_32(b"eos token");
        let precision = 4;
        let t: Token = Token::new(t_sym.clone(), t_desc, precision);
        assert_eq!(TokenBalances::register_token(t, 0, 0), Ok(()));

        assert_eq!(TokenBalances::token_list_len(), 4);
        assert_eq!(TokenBalances::token_list_map(3), t_sym.clone());

        let btc_symbol = b"x-btc".to_vec(); //b"x-btc".to_vec();
        let eth_symbol = b"x-eth".to_vec(); //slice_to_u8_8(b"x-eth");
        assert_eq!(TokenBalances::token_list(), vec![
            Test::CHAINX_SYMBOL.to_vec(),
            btc_symbol.clone(),
            eth_symbol.clone(),
            t_sym.clone(),
        ]);

        assert_eq!(TokenBalances::total_free_token(t_sym.clone()), 0);
        assert_eq!(TokenBalances::token_info(t_sym.clone()).unwrap().0.precision(), 4);

        // test err branch
        let btc_t = Token::new(btc_symbol.clone(), b"btc token".to_vec(), 4);
        assert_noop!(TokenBalances::register_token(btc_t, 0, 0), "already has this token symbol");
        assert_eq!(TokenBalances::token_list_len(), 4);
        assert_eq!(TokenBalances::token_list_map(4), b"".to_vec());
    })
}

#[test]
fn test_remove() {
    with_externalities(&mut new_test_ext(), || {
        // register a new token
        let t_sym: Symbol = b"x-eos".to_vec();
        let t_desc: TokenDesc = b"eos token".to_vec();
        let precision: Precision = 4;
        let t: Token = Token::new(t_sym.clone(), t_desc, precision);
        assert_eq!(TokenBalances::register_token(t.clone(), 0, 0), Ok(()));
        assert_eq!(TokenBalances::token_list_map(3), t_sym.clone());

        // remove it
        assert_eq!(TokenBalances::cancel_token(&t_sym.clone()), Ok(()));
        assert_eq!(TokenBalances::token_list_map(3), t_sym.clone());
        assert_eq!(TokenBalances::token_list_len(), 4); // length not modify

        // re-register, but must be failed
        assert_noop!(TokenBalances::register_token(t.clone(), 0, 0), "already has this token symbol");

        // create new token symbol
        let t_new: Token = Token { symbol: b"x-eos2".to_vec(), ..t };
        assert_noop!(TokenBalances::cancel_token(&t_new.symbol), "this token symbol dose not register yet or is invalid");
        assert_eq!(TokenBalances::register_token(t_new.clone(), 0, 0), Ok(()));
        assert_eq!(TokenBalances::token_list_map(3), t_sym.clone());
        assert_eq!(TokenBalances::token_list_map(4), t_new.symbol);
        assert_eq!(TokenBalances::token_list_len(), 5);
    })
}

#[test]
fn test_total_balance() {
    with_externalities(&mut new_test_ext(), || {
        let btc_symbol = b"x-btc".to_vec();
        assert_eq!(TokenBalances::total_token(&btc_symbol.clone()), 100);

        TokenBalances::issue(&0, &btc_symbol, 100).unwrap();
        assert_eq!(TokenBalances::total_token(&btc_symbol.clone()), 200);

        TokenBalances::issue(&0, &btc_symbol, 50).unwrap();
        TokenBalances::reserve(&0, &btc_symbol, 50).unwrap();
        assert_eq!(TokenBalances::total_token(&btc_symbol.clone()), 250);

        TokenBalances::destroy(&0, &btc_symbol, 25).unwrap();
        assert_eq!(TokenBalances::total_token(&btc_symbol.clone()), 225);
    })
}

#[test]
fn test_account_balance() {
    with_externalities(&mut new_test_ext(), || {
        let a: u64 = 1; // accountid
        let btc_symbol = b"x-btc".to_vec();
        let key = (a, btc_symbol.clone());
        assert_eq!(TokenBalances::free_token(&key), 0);
        assert_eq!(TokenBalances::reserved_token(&key), 0);
        assert_eq!(TokenBalances::total_token_of(&a, &btc_symbol), 0);

        TokenBalances::issue(&a, &btc_symbol, 100).unwrap();
        assert_eq!(TokenBalances::free_token(&key), 100);
        assert_eq!(TokenBalances::reserved_token(&key), 0);
        assert_eq!(TokenBalances::total_token_of(&a, &btc_symbol.clone()), 100);

        TokenBalances::reserve(&a, &btc_symbol, 50).unwrap();
        TokenBalances::destroy(&a, &btc_symbol, 50).unwrap();
        assert_eq!(TokenBalances::total_token_of(&a, &btc_symbol.clone()), 50);
    })
}

#[test]
fn test_normal_issue_and_destroy() {
    with_externalities(&mut new_test_ext(), || {
        let a: u64 = 1; // accountid
        let btc_symbol = b"x-btc".to_vec();
        let key = (a, btc_symbol.clone());

        // issue
        TokenBalances::issue(&a, &btc_symbol.clone(), 50).unwrap();
        assert_eq!(TokenBalances::total_token_of(&a, &btc_symbol.clone()), 50);
        assert_eq!(TokenBalances::total_token(&btc_symbol.clone()), 150);

        // reserve
        TokenBalances::reserve(&a, &btc_symbol.clone(), 25).unwrap();
        assert_eq!(TokenBalances::reserved_token(&key), 25);
        assert_eq!(TokenBalances::free_token(&key), 25);
        assert_eq!(TokenBalances::total_token_of(&a, &btc_symbol.clone()), 50);
        assert_eq!(TokenBalances::total_reserved_token(&btc_symbol.clone()), 25);

        // destroy
        TokenBalances::destroy(&a, &btc_symbol.clone(), 25).unwrap();
        assert_eq!(TokenBalances::reserved_token(&key), 0);
        assert_eq!(TokenBalances::free_token(&key), 25);
        assert_eq!(TokenBalances::total_token_of(&a, &btc_symbol.clone()), 25);
        assert_eq!(TokenBalances::total_reserved_token(&btc_symbol.clone()), 0);
        assert_eq!(TokenBalances::total_token(&btc_symbol.clone()), 125);
    })
}

#[test]
fn test_unlock_issue_and_destroy2() {
    with_externalities(&mut new_test_ext(), || {
        let a: u64 = 1; // accountid
        let btc_symbol = b"x-btc".to_vec();
        let key = (a, btc_symbol.clone());

        // issue
        TokenBalances::issue(&a, &btc_symbol.clone(), 50).unwrap();
        assert_eq!(TokenBalances::total_token_of(&a, &btc_symbol.clone()), 50);
        assert_eq!(TokenBalances::total_token(&btc_symbol.clone()), 150);


        // reserve
        TokenBalances::reserve(&a, &btc_symbol.clone(), 25).unwrap();
        assert_eq!(TokenBalances::reserved_token(&key), 25);
        assert_eq!(TokenBalances::free_token(&key), 25);
        assert_eq!(TokenBalances::total_token_of(&a, &btc_symbol.clone()), 50);
        assert_eq!(TokenBalances::total_reserved_token(&btc_symbol.clone()), 25);

        // unreserve
        TokenBalances::unreserve(&a, &btc_symbol.clone(), 10).unwrap();
        assert_eq!(TokenBalances::reserved_token(&key), 15);
        assert_eq!(TokenBalances::free_token(&key), 35);
        assert_eq!(TokenBalances::total_token_of(&a, &btc_symbol.clone()), 50);
        assert_eq!(TokenBalances::total_reserved_token(&btc_symbol.clone()), 15);
    })
}

#[test]
fn test_error_issue_and_destroy1() {
    with_externalities(&mut new_test_ext(), || {
        let a: u64 = 1; // accountid
        let btc_symbol = b"x-btc".to_vec();
        // issue
        TokenBalances::issue(&a, &btc_symbol.clone(), 50).unwrap();
        assert_eq!(TokenBalances::total_token_of(&a, &btc_symbol.clone()), 50);
        assert_eq!(TokenBalances::total_token(&btc_symbol.clone()), 150);
        // destroy first
        // destroy
        assert_err!(TokenBalances::destroy(&a, &btc_symbol.clone(), 25), "reserved token too low to destroy");
        // reserve
        assert_eq!(TokenBalances::total_free_token(&btc_symbol.clone()), 150);
        assert_err!(TokenBalances::reserve(&a, &btc_symbol.clone(), 100), "free token too low to reserve");
        // lock first
        assert_ok!(TokenBalances::reserve(&a, &btc_symbol.clone(), 25));
        // destroy
        assert_ok!(TokenBalances::destroy(&a, &btc_symbol.clone(), 25));
    })
}

#[test]
fn test_error_issue_and_destroy2() {
    with_externalities(&mut new_test_ext(), || {
        let a: u64 = 1; // accountid
        let btc_symbol = b"x-btc".to_vec();
        // issue
        TokenBalances::issue(&a, &btc_symbol.clone(), 50).unwrap();
        assert_eq!(TokenBalances::total_token_of(&a, &btc_symbol), 50);
        assert_eq!(TokenBalances::total_token(&btc_symbol.clone()), 150);
        // overflow
        let i: i32 = -1;
        assert_err!(TokenBalances::reserve(&a, &btc_symbol.clone(), i as TokenBalance), "free token too low to reserve");
        assert_err!(TokenBalances::issue(&a, &btc_symbol.clone(), i as TokenBalance), "free token too high to issue");
    })
}

#[test]
fn test_error_issue_and_destroy3() {
    with_externalities(&mut new_test_ext(), || {
        let a: u64 = 1; // accountid
        let btc_symbol = b"x-btc".to_vec();
        // lock or destroy without init
        assert_err!(TokenBalances::destroy(&a, &btc_symbol.clone(), 25), "not a existed token in this account token list");
        assert_err!(TokenBalances::reserve(&a, &btc_symbol.clone(), 25), "not a existed token in this account token list");
        TokenBalances::issue(&a, &btc_symbol.clone(), 0).unwrap();
        assert_err!(TokenBalances::destroy(&a, &btc_symbol.clone(), 25), "reserved token too low to destroy");
        assert_err!(TokenBalances::reserve(&a, &btc_symbol.clone(), 25), "free token too low to reserve");

        TokenBalances::issue(&a, &btc_symbol.clone(), 100).unwrap();
        assert_ok!(TokenBalances::reserve(&a, &btc_symbol.clone(), 25));
        assert_ok!(TokenBalances::destroy(&a, &btc_symbol.clone(), 25));
    })
}

#[test]
fn test_transfer() {
    with_externalities(&mut new_test_ext(), || {
        let a: u64 = 1; // accountid
        let b: u64 = 2; // accountid
        let btc_symbol = b"x-btc".to_vec();
        // issue 50 to account 1
        TokenBalances::issue(&a, &btc_symbol.clone(), 50).unwrap();
        // transfer
        TokenBalances::transfer_token(Some(a).into(), b.into(), btc_symbol.clone().clone(), 25).unwrap();
        // sum not change
        assert_eq!(TokenBalances::total_free_token(&btc_symbol.clone()), 150);
        assert_eq!(TokenBalances::total_token_of(&a, &btc_symbol.clone()), 25);
        assert_eq!(TokenBalances::free_token(&(b, btc_symbol.clone())), 25);
        assert_eq!(Balances::free_balance(&a), 990);

        assert_err!(TokenBalances::transfer_token(Some(a).into(), b.into(), btc_symbol.clone().clone(), 50), "free token too low to send value")
    })
}

#[test]
fn test_transfer_to_self() {
    with_externalities(&mut new_test_ext(), || {
        let a: u64 = 1; // accountid
        let btc_symbol = b"x-btc".to_vec();
        // issue 50 to account 1
        TokenBalances::issue(&a, &btc_symbol.clone(), 50).unwrap();
        // transfer
        TokenBalances::transfer_token(Some(a).into(), a.into(), btc_symbol.clone().clone(), 25).unwrap();
        // sum not change
        assert_eq!(TokenBalances::total_free_token(&btc_symbol.clone()), 150);
        assert_eq!(TokenBalances::total_token_of(&a, &btc_symbol.clone()), 50);
        assert_eq!(Balances::free_balance(&a), 990);
    })
}

#[test]
fn test_transfer_err() {
    with_externalities(&mut new_test_ext(), || {
        let a: u64 = 1; // accountid
        let b: u64 = 2; // accountid
        let btc_symbol = b"x-btc".to_vec();
        // issue 50 to account 2
        TokenBalances::issue(&b, &btc_symbol.clone(), 50).unwrap();
        // transfer
        TokenBalances::transfer_token(Some(b).into(), a.into(), btc_symbol.clone().clone(), 25).unwrap();
        // sum not change
        assert_eq!(TokenBalances::total_free_token(&btc_symbol.clone()), 150);
        assert_eq!(TokenBalances::free_token(&(b, btc_symbol.clone())), 25);
        assert_eq!(TokenBalances::total_token_of(&a, &btc_symbol.clone()), 25);
        assert_eq!(Balances::free_balance(&b), 500);

        assert_err!(TokenBalances::transfer_token(Some(b).into(), a.into(), btc_symbol.clone(), 1),
            "chainx balance is not enough after this tx, not allow to be killed at here");
        assert_eq!(Balances::free_balance(&b), 500);
    })
}

#[test]
fn test_char_valid() {
    with_externalities(&mut new_test_ext(), || {
        let to: balances::Address<Test> = balances::address::Address::Id(2);
        let origin = system::RawOrigin::Signed(1).into();
        let sym = b"".to_vec();
        assert_err!(TokenBalances::transfer_token(origin, to.clone(), sym, 10), "symbol length too long or zero");

        let origin = system::RawOrigin::Signed(1).into();
        let sym = b"dfasdlfjkalsdjfklasjdflkasjdfklasjklfasjfkdlsajf".to_vec();
        assert_err!(TokenBalances::transfer_token(origin, to.clone(), sym, 10), "symbol length too long or zero");

        let origin = system::RawOrigin::Signed(1).into();
        let sym = b"23jfkldae(".to_vec();
        assert_err!(TokenBalances::transfer_token(origin, to.clone(), sym, 10), "not a valid symbol char for number, capital/small letter or '-', '.', '|', '~'");

        let t: Token = Token::new(b"x-btc2".to_vec(), b"btc token fdsfsdfasfasdfasdfasdfasdfasdfasdfjaskldfjalskdjflk;asjdfklasjkldfjalksdjfklasjflkdasjflkjkladsjfkrtewtewrtwertrjhjwretywertwertwerrtwerrtwerrtwertwelasjdfklsajdflkaj".to_vec(), 8);
        assert_err!(TokenBalances::register_token(t, 0, 0), "token desc length too long");
        let t: Token = Token::new(b"x-btc?".to_vec(), b"btc token".to_vec(), 8);
        assert_err!(TokenBalances::register_token(t, 0, 0), "not a valid symbol char for number, capital/small letter or '-', '.', '|', '~'")
    })
}

#[test]
fn test_chainx() {
    with_externalities(&mut new_test_ext2(), || {
        let a: u64 = 1; // accountid
        let b: u64 = 2; // accountid
        let sym = Test::CHAINX_SYMBOL.to_vec();
        assert_err!(TokenBalances::issue(&a, &sym, 100), "can't issue chainx token");

        assert_ok!(TokenBalances::reserve(&a, &sym, 100));
        assert_eq!(Balances::free_balance(&a), 900);
        assert_eq!(Balances::reserved_balance(&a), 100);
        assert_eq!(TokenBalances::reserved_token(&(a, sym.clone())), 100);

        assert_ok!(TokenBalances::unreserve(&a, &sym, 50));
        assert_eq!(Balances::free_balance(&a), 950);
        assert_eq!(TokenBalances::reserved_token(&(a, sym.clone())), 50);
        assert_eq!(Balances::reserved_balance(&a), 50);
        assert_err!(TokenBalances::destroy(&a, &sym, 50), "can't destroy chainx token");

        assert_err!(TokenBalances::transfer_token(Some(b).into(), a.into(), sym.clone(), 1), "not allow to transfer chainx use transfer_token");
    })
}

#[test]
fn test_chainx_err() {
    with_externalities(&mut new_test_ext2(), || {
        let a: u64 = 1; // accountid
        let sym = Test::CHAINX_SYMBOL.to_vec();

        assert_err!(TokenBalances::reserve(&a, &sym, 2000), "chainx free token too low to reserve");
        assert_err!(TokenBalances::unreserve(&a, &sym, 10), "chainx reserved token too low to unreserve");

        let i: i32 = -1;
        let larger_balance: TokenBalance = (i as u64) as u128 + 2;

        assert_eq!(larger_balance, 18446744073709551617);
        assert_eq!(larger_balance as u64, 1);

        assert_ok!(TokenBalances::reserve(&a, &sym, larger_balance));
        assert_eq!(Balances::free_balance(&a), 999);

        let i: i32 = -1;
        let max_balance: TokenBalance = i as u128;
        assert_eq!(max_balance as u64, 18446744073709551615);
        assert_err!(TokenBalances::reserve(&a, &sym, max_balance), "chainx free token too low to reserve");
    })
}