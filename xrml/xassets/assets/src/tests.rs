// Copyright 2018 Chainpool.

use super::*;
use mock::*;
use rstd::collections::btree_map::BTreeMap;
use rstd::iter::FromIterator;
use runtime_io::with_externalities;

#[test]
fn test_genesis() {
    with_externalities(&mut new_test_ext(), || {
        // Check that GenesisBuilder works properly.
        // check token_list
        let btc_token = b"BTC".to_vec();

        assert_eq!(
            XAssets::assets(),
            vec![XAssets::TOKEN.to_vec(), btc_token.clone(),]
        );

        assert_eq!(XAssets::asset_info(&btc_token).unwrap().0.precision(), 8);

        // chainx tokenbol for every user
        assert_eq!(XAssets::assets_of(&0), vec![XAssets::TOKEN.to_vec()]);
    });
}

#[test]
fn test_genesis_token_issue() {
    with_externalities(&mut new_test_ext(), || {
        let btc_token = b"BTC".to_vec();
        let chainx_token = XAssets::TOKEN.to_vec();
        assert_eq!(
            XAssets::asset_balance(&3, &chainx_token, AssetType::Free),
            1000
        );
        assert_eq!(XAssets::asset_balance(&3, &btc_token, AssetType::Free), 100);

        assert_eq!(XAssets::assets_of(&3), [chainx_token, btc_token]);
    })
}

#[test]
#[should_panic]
fn test_err_genesis() {
    with_externalities(&mut err_test_ext(), || {})
}

#[test]
fn test_register() {
    with_externalities(&mut new_test_ext(), || {
        let token: Token = b"ETH".to_vec(); //slice_to_u8_8(b"x-eos");
        let token_name: Token = b"Ethereum".to_vec(); //slice_to_u8_8(b"x-eos");
        let desc: Desc = b"eth token".to_vec(); //slice_to_u8_32(b"eos token");
        let precision = 4;
        let asset = Asset::new(
            token.clone(),
            token_name.clone(),
            Chain::Ethereum,
            precision,
            desc,
        )
        .unwrap();
        assert_eq!(XAssets::register_asset(asset.clone(), false, 0), Ok(()));

        let btc_token = b"BTC".to_vec(); //b"BTC".to_vec();

        assert_eq!(
            XAssets::assets(),
            vec![XAssets::TOKEN.to_vec(), btc_token, token.clone()]
        );

        assert_eq!(XAssets::total_asset_balance(&token, AssetType::Free), 0);
        assert_eq!(XAssets::asset_info(&token).unwrap().0.precision(), 4);
        assert_noop!(
            XAssets::register_asset(asset, false, 0),
            "already has this token"
        );
    })
}

#[test]
fn test_remove() {
    with_externalities(&mut new_test_ext(), || {
        // register a new token
        let token: Token = b"ETH".to_vec(); //slice_to_u8_8(b"x-eos");
        let token_name: Token = b"Ethereum".to_vec(); //slice_to_u8_8(b"x-eos");
        let desc: Desc = b"eth token".to_vec(); //slice_to_u8_32(b"eos token");
        let precision = 4;
        let asset = Asset::new(
            token.clone(),
            token_name.clone(),
            Chain::Ethereum,
            precision,
            desc,
        )
        .unwrap();
        assert_eq!(XAssets::register_asset(asset.clone(), false, 0), Ok(()));

        // remove it
        assert_eq!(XAssets::revoke_asset(token.clone()), Ok(()));
        assert_noop!(XAssets::is_valid_asset(&token), "not a valid token");

        // re-register, but must be failed
        assert_noop!(
            XAssets::register_asset(asset, false, 0),
            "already has this token"
        );
    })
}

#[test]
fn test_total_balance() {
    with_externalities(&mut new_test_ext(), || {
        let btc_token = b"BTC".to_vec();
        assert_eq!(
            XAssets::total_asset_balance(&btc_token, AssetType::Free),
            100
        );

        XAssets::issue(&btc_token, &0, 100).unwrap();
        assert_eq!(
            XAssets::total_asset_balance(&btc_token, AssetType::Free),
            200
        );

        XAssets::issue(&btc_token, &0, 50).unwrap();
        XAssets::move_balance(
            &btc_token,
            &0,
            AssetType::Free,
            &0,
            AssetType::ReservedWithdrawal,
            50,
        )
        .unwrap();
        assert_eq!(XAssets::all_type_balance(&btc_token), 250);
        assert_eq!(
            XAssets::total_asset_balance(&btc_token, AssetType::ReservedWithdrawal),
            50
        );

        XAssets::destroy(&btc_token, &0, 25).unwrap();
        assert_eq!(XAssets::all_type_balance(&btc_token), 225);
        // chainx total
        let token = XAssets::TOKEN.to_vec();
        assert_eq!(
            XAssets::total_asset_balance(&token, AssetType::Free),
            1000 + 510 + 1000
        );
        XAssets::move_balance(
            &token,
            &1,
            AssetType::Free,
            &1,
            AssetType::ReservedWithdrawal,
            50,
        )
        .unwrap();

        assert_eq!(
            XAssets::total_asset_balance(&token, AssetType::Free),
            1000 + 510 + 1000 - 50
        );
        assert_eq!(
            XAssets::total_asset_balance(&token, AssetType::ReservedWithdrawal),
            50
        );
        XAssets::move_balance(
            &token,
            &1,
            AssetType::ReservedWithdrawal,
            &1,
            AssetType::Free,
            25,
        )
        .unwrap();

        assert_eq!(
            XAssets::total_asset_balance(&token, AssetType::Free),
            1000 + 510 + 1000 - 50 + 25
        );
        assert_eq!(
            XAssets::total_asset_balance(&token, AssetType::ReservedWithdrawal),
            25
        );
    })
}

#[test]
fn test_account_balance() {
    with_externalities(&mut new_test_ext(), || {
        let a: u64 = 1; // accountid
        let btc_token = b"BTC".to_vec();
        assert_eq!(XAssets::free_balance(&a, &btc_token), 0);
        assert_eq!(
            XAssets::asset_balance(&a, &btc_token, AssetType::ReservedWithdrawal),
            0
        );
        assert_eq!(XAssets::all_type_balance_of(&a, &btc_token), 0);

        XAssets::issue(&btc_token, &a, 100).unwrap();
        assert_eq!(XAssets::free_balance(&a, &btc_token), 100);
        assert_eq!(
            XAssets::asset_balance(&a, &btc_token, AssetType::ReservedWithdrawal),
            0
        );
        assert_eq!(XAssets::all_type_balance_of(&a, &btc_token), 100);

        XAssets::move_balance(
            &btc_token,
            &a,
            AssetType::Free,
            &a,
            AssetType::ReservedWithdrawal,
            50,
        )
        .unwrap();

        XAssets::destroy(&btc_token, &a, 50).unwrap();
        assert_eq!(XAssets::all_type_balance_of(&a, &btc_token), 50);
    })
}

#[test]
fn test_normal_issue_and_destroy() {
    with_externalities(&mut new_test_ext(), || {
        let a: u64 = 1; // accountid
        let btc_token = b"BTC".to_vec();

        // issue
        XAssets::issue(&btc_token, &a, 50).unwrap();
        assert_eq!(XAssets::all_type_balance_of(&a, &btc_token), 50);
        assert_eq!(XAssets::all_type_balance(&btc_token), 150);

        // reserve
        XAssets::move_balance(
            &btc_token,
            &a,
            AssetType::Free,
            &a,
            AssetType::ReservedWithdrawal,
            25,
        )
        .unwrap();

        assert_eq!(
            XAssets::asset_balance(&a, &btc_token, AssetType::ReservedWithdrawal),
            25
        );
        assert_eq!(XAssets::free_balance(&a, &btc_token), 25);
        assert_eq!(XAssets::all_type_balance_of(&a, &btc_token), 50);
        assert_eq!(
            XAssets::total_asset_balance(&btc_token, AssetType::ReservedWithdrawal),
            25
        );

        // destroy
        XAssets::destroy(&btc_token, &a, 25).unwrap();
        assert_eq!(
            XAssets::asset_balance(&a, &btc_token, AssetType::ReservedWithdrawal),
            0
        );
        assert_eq!(XAssets::free_balance(&a, &btc_token), 25);
        assert_eq!(XAssets::all_type_balance_of(&a, &btc_token), 25);
        assert_eq!(
            XAssets::total_asset_balance(&btc_token, AssetType::ReservedWithdrawal),
            0
        );
        assert_eq!(XAssets::all_type_balance(&btc_token), 125);
    })
}

#[test]
fn test_unlock_issue_and_destroy2() {
    with_externalities(&mut new_test_ext(), || {
        let a: u64 = 1; // accountid
        let btc_token = b"BTC".to_vec();

        // issue
        XAssets::issue(&btc_token, &a, 50).unwrap();
        assert_eq!(XAssets::all_type_balance_of(&a, &btc_token), 50);
        assert_eq!(XAssets::all_type_balance(&btc_token), 150);

        // reserve
        XAssets::move_balance(
            &btc_token,
            &a,
            AssetType::Free,
            &a,
            AssetType::ReservedWithdrawal,
            25,
        )
        .unwrap();

        assert_eq!(
            XAssets::asset_balance(&a, &btc_token, AssetType::ReservedWithdrawal),
            25
        );
        assert_eq!(XAssets::free_balance(&a, &btc_token), 25);
        assert_eq!(XAssets::all_type_balance_of(&a, &btc_token), 50);
        assert_eq!(
            XAssets::total_asset_balance(&btc_token, AssetType::ReservedWithdrawal),
            25
        );

        // unreserve
        XAssets::move_balance(
            &btc_token,
            &a,
            AssetType::ReservedWithdrawal,
            &a,
            AssetType::Free,
            10,
        )
        .unwrap();

        assert_eq!(
            XAssets::asset_balance(&a, &btc_token, AssetType::ReservedWithdrawal),
            15
        );
        assert_eq!(XAssets::free_balance(&a, &btc_token), 35);
        assert_eq!(XAssets::all_type_balance_of(&a, &btc_token), 50);
        assert_eq!(
            XAssets::total_asset_balance(&btc_token, AssetType::ReservedWithdrawal),
            15
        );
    })
}

#[test]
fn test_error_issue_and_destroy1() {
    with_externalities(&mut new_test_ext(), || {
        let a: u64 = 1; // accountid
        let btc_token = b"BTC".to_vec();
        // issue
        XAssets::issue(&btc_token, &a, 50).unwrap();
        assert_eq!(XAssets::all_type_balance_of(&a, &btc_token), 50);
        assert_eq!(XAssets::all_type_balance(&btc_token), 150);
        // destroy first
        // destroy
        assert_err!(
            XAssets::destroy(&btc_token, &a, 25),
            "reserved balance too low to destroy"
        );
        // reserve
        assert_eq!(
            XAssets::total_asset_balance(&btc_token, AssetType::Free),
            150
        );

        assert_err!(
            XAssets::move_balance(
                &btc_token,
                &a,
                AssetType::Free,
                &a,
                AssetType::ReservedWithdrawal,
                100
            ),
            AssetErr::NotEnough
        );

        // lock first
        assert_ok!(XAssets::move_balance(
            &btc_token,
            &a,
            AssetType::Free,
            &a,
            AssetType::ReservedWithdrawal,
            25
        ));
        // destroy
        assert_ok!(XAssets::destroy(&btc_token, &a, 25));
    })
}

#[test]
fn test_error_issue_and_destroy2() {
    with_externalities(&mut new_test_ext(), || {
        let a: u64 = 1; // accountid
        let btc_token = b"BTC".to_vec();
        // issue
        XAssets::issue(&btc_token, &a, 50).unwrap();
        assert_eq!(XAssets::all_type_balance_of(&a, &btc_token), 50);
        assert_eq!(XAssets::all_type_balance(&btc_token), 150);
        // overflow
        let i: i32 = -1;

        assert_err!(
            XAssets::move_balance(
                &btc_token,
                &a,
                AssetType::Free,
                &a,
                AssetType::ReservedWithdrawal,
                i as Balance,
            ),
            AssetErr::NotEnough
        );

        assert_err!(
            XAssets::issue(&btc_token, &a, i as Balance),
            "free balance too high to issue"
        );
    })
}

#[test]
fn test_error_issue_and_destroy3() {
    with_externalities(&mut new_test_ext(), || {
        let a: u64 = 1; // accountid
        let btc_token = b"BTC".to_vec();
        // lock or destroy without init
        assert_err!(
            XAssets::destroy(&btc_token, &a, 25),
            "not a existed token in this account token list"
        );

        assert_err!(
            XAssets::move_balance(
                &btc_token,
                &a,
                AssetType::Free,
                &a,
                AssetType::ReservedWithdrawal,
                25
            ),
            AssetErr::InvalidToken
        );

        XAssets::issue(&btc_token, &a, 0).unwrap();
        assert_err!(
            XAssets::destroy(&btc_token, &a, 25),
            "reserved balance too low to destroy"
        );

        assert_err!(
            XAssets::move_balance(
                &btc_token,
                &a,
                AssetType::Free,
                &a,
                AssetType::ReservedWithdrawal,
                25
            ),
            AssetErr::NotEnough
        );

        XAssets::issue(&btc_token, &a, 100).unwrap();

        assert_ok!(XAssets::move_balance(
            &btc_token,
            &a,
            AssetType::Free,
            &a,
            AssetType::ReservedWithdrawal,
            25
        ));

        assert_ok!(XAssets::destroy(&btc_token, &a, 25));
    })
}

#[test]
fn test_transfer_not_init() {
    with_externalities(&mut new_test_ext(), || {
        let a: u64 = 1; // accountid
        let new_id: u64 = 1000;
        let btc_token = b"BTC".to_vec();
        let chainx_token = XAssets::TOKEN.to_vec();
        XAssets::issue(&btc_token, &a, 50).unwrap();
        assert_ok!(XAssets::transfer(
            Some(a).into(),
            new_id.into(),
            btc_token.clone(),
            25,
            b"".to_vec()
        ));
        //assert_eq!(Indices::lookup_index(1), Some(new_id));
        assert_ok!(XAssets::transfer(
            Some(a).into(),
            new_id.into(),
            btc_token.clone(),
            25,
            b"".to_vec()
        ));
        assert_ok!(XAssets::transfer(
            Some(a).into(),
            new_id.into(),
            chainx_token.clone(),
            25,
            b"".to_vec()
        ));

        assert_eq!(XAssets::free_balance(&a, &chainx_token), 1000 - 25);
        assert_eq!(XAssets::free_balance(&new_id, &chainx_token), 25);
    })
}

#[test]
fn test_transfer_chainx() {
    with_externalities(&mut new_test_ext(), || {
        let a: u64 = 1; // accountid
        let b: u64 = 2; // accountid

        let chainx_token = XAssets::TOKEN.to_vec();

        assert_ok!(XAssets::transfer(
            Some(a).into(),
            b.into(),
            chainx_token.clone(),
            25,
            b"".to_vec()
        ));

        assert_eq!(XAssets::free_balance(&a, &chainx_token), 1000 - 25);
        assert_eq!(XAssets::free_balance(&b, &chainx_token), 510 + 25);

        assert_err!(
            XAssets::transfer(
                Some(a).into(),
                b.into(),
                chainx_token.clone(),
                1000,
                b"".to_vec()
            ),
            "balance too low for this account"
        );
    })
}

#[test]
fn test_transfer_token() {
    with_externalities(&mut new_test_ext(), || {
        let a: u64 = 1; // accountid
        let b: u64 = 2; // accountid
        let btc_token = b"BTC".to_vec();
        // issue 50 to account 1
        XAssets::issue(&btc_token, &a, 50).unwrap();
        // transfer
        XAssets::transfer(
            Some(a).into(),
            b.into(),
            btc_token.clone(),
            25,
            b"".to_vec(),
        )
        .unwrap();
        // sum not change
        assert_eq!(
            XAssets::total_asset_balance(&btc_token, AssetType::Free),
            150
        );
        assert_eq!(XAssets::all_type_balance_of(&a, &btc_token), 25);
        assert_eq!(XAssets::free_balance(&b, &btc_token), 25);

        assert_err!(
            XAssets::transfer(Some(a).into(), b.into(), btc_token, 50, b"".to_vec()),
            "balance too low for this account"
        )
    })
}

#[test]
fn test_transfer_to_self() {
    with_externalities(&mut new_test_ext(), || {
        let a: u64 = 1; // accountid
        let btc_token = b"BTC".to_vec();
        // issue 50 to account 1
        XAssets::issue(&btc_token, &a, 50).unwrap();
        // transfer
        assert_ok!(XAssets::transfer(
            Some(a).into(),
            a.into(),
            btc_token.clone(),
            25,
            b"".to_vec()
        ));

        // sum not change
        assert_eq!(
            XAssets::total_asset_balance(&btc_token, AssetType::Free),
            150
        );
        assert_eq!(XAssets::all_type_balance_of(&a, &btc_token), 50);
    })
}

#[test]
fn test_set_token() {
    with_externalities(&mut new_test_ext(), || {
        let a: u64 = 1; // accountid
        let btc_token = b"BTC".to_vec();
        XAssets::issue(&btc_token, &a, 50).unwrap();
        let b = CodecBTreeMap::<AssetType, Balance>(BTreeMap::from_iter(
            vec![(AssetType::Free, 500)].into_iter(),
        ));
        assert_ok!(XAssets::set_balance(a.into(), XAssets::TOKEN.to_vec(), b));
        assert_eq!(Balances::free_balance(&a), 500);

        let b = CodecBTreeMap::<AssetType, Balance>(BTreeMap::from_iter(
            vec![(AssetType::Free, 500)].into_iter(),
        ));
        assert_ok!(XAssets::set_balance(a.into(), btc_token.clone(), b));
        assert_eq!(XAssets::free_balance(&a, &btc_token), 500);
        assert_eq!(
            XAssets::total_asset_balance(&btc_token, AssetType::Free),
            500 + 100
        );
        assert_eq!(XAssets::all_type_balance(&btc_token), 500 + 100);

        let b = CodecBTreeMap::<AssetType, Balance>(BTreeMap::from_iter(
            vec![(AssetType::Free, 600)].into_iter(),
        ));
        assert_ok!(XAssets::set_balance(a.into(), btc_token.clone(), b));
        assert_eq!(XAssets::free_balance(&a, &btc_token), 600);
        assert_eq!(XAssets::all_type_balance(&btc_token), 600 + 100);
    })
}

#[test]
fn test_char_valid() {
    with_externalities(&mut new_test_ext(), || {
        let token = b"".to_vec();
        let asset = Asset::new(
            token.clone(),
            token.clone(),
            Chain::Ethereum,
            1,
            b"123".to_vec(),
        );
        assert_err!(asset, "Token length is zero or too long.");

        let token = b"dfasdlfjkalsdjfklasjdflkasjdfklasjklfasjfkdlsajf".to_vec();
        let asset = Asset::new(
            token.clone(),
            token.clone(),
            Chain::Ethereum,
            1,
            b"123".to_vec(),
        );
        assert_err!(asset, "Token length is zero or too long.");

        let token = b"23jfkldae(".to_vec();
        let asset = Asset::new(
            token.clone(),
            token.clone(),
            Chain::Ethereum,
            1,
            b"123".to_vec(),
        );
        assert_err!(
            asset,
            "Token can only use numbers, capital/lowercase letters or \'-\', \'.\', \'|\', \'~\'."
        );

        let asset = Asset::new(b"BTC2".to_vec(), b"Bitcoin".to_vec(), Chain::Ethereum, 1, b"btc token fdsfsdfasfasdfasdfasdfasdfasdfasdfjaskldfjalskdjflk;asjdfklasjkldfjalksdjfklasjflkdasjflkjkladsjfkrtewtewrtwertrjhjwretywertwertwerrtwerrtwerrtwertwelasjdfklsajdflkaj".to_vec());
        assert_err!(asset, "Token desc too long");
        let asset = Asset::new(
            b"BTC?".to_vec(),
            b"Bitcoin".to_vec(),
            Chain::Ethereum,
            1,
            b"123".to_vec(),
        );
        assert_err!(
            asset,
            "Token can only use numbers, capital/lowercase letters or \'-\', \'.\', \'|\', \'~\'."
        )
    })
}

#[test]
fn test_chainx() {
    with_externalities(&mut new_test_ext(), || {
        let a: u64 = 1; // accountid
        let token = XAssets::TOKEN.to_vec();
        assert_ok!(XAssets::issue(&token, &a, 100));

        assert_eq!(Balances::free_balance(&a), 1100);

        assert_ok!(XAssets::move_balance(
            &token,
            &a,
            AssetType::Free,
            &a,
            AssetType::ReservedWithdrawal,
            100
        ));

        assert_eq!(Balances::free_balance(&a), 1000);
        assert_eq!(Balances::reserved_balance(&a), 0);
        assert_eq!(
            XAssets::asset_balance(&a, &token, AssetType::ReservedWithdrawal),
            100
        );

        assert_ok!(XAssets::move_balance(
            &token,
            &a,
            AssetType::ReservedWithdrawal,
            &a,
            AssetType::Free,
            50
        ));

        assert_eq!(Balances::free_balance(&a), 1050);
        assert_eq!(
            XAssets::asset_balance(&a, &token, AssetType::ReservedWithdrawal),
            50
        );
        assert_eq!(Balances::reserved_balance(&a), 0);
        assert_err!(
            XAssets::destroy(&token, &a, 50),
            "should not use chainx token here"
        );
    })
}

#[test]
fn test_chainx_err() {
    with_externalities(&mut new_test_ext(), || {
        let a: u64 = 1; // accountid
        let token = XAssets::TOKEN.to_vec();

        assert_err!(
            XAssets::move_balance(
                &token,
                &a,
                AssetType::Free,
                &a,
                AssetType::ReservedWithdrawal,
                2000
            ),
            AssetErr::NotEnough
        );

        assert_err!(
            XAssets::move_balance(
                &token,
                &a,
                AssetType::ReservedWithdrawal,
                &a,
                AssetType::Free,
                10
            ),
            AssetErr::NotEnough
        );

        let i: i32 = -1;
        let larger_balance: Balance = i as u64;

        assert_eq!(larger_balance, 18446744073709551615);

        assert_err!(
            XAssets::move_balance(
                &token,
                &a,
                AssetType::Free,
                &a,
                AssetType::ReservedWithdrawal,
                larger_balance
            ),
            AssetErr::NotEnough
        );
    })
}

#[test]
fn test_move() {
    with_externalities(&mut new_test_ext(), || {
        let a: u64 = 1; // accountid
        let b: u64 = 2; // accountid
        let token = XAssets::TOKEN.to_vec();
        assert_ok!(XAssets::move_free_balance(&token, &a, &b, 100));
        assert_err!(
            XAssets::move_free_balance(&token, &a, &b, 1000),
            AssetErr::NotEnough
        );
        assert_eq!(Balances::free_balance(&a), 900);
        assert_eq!(Balances::free_balance(&b), 510 + 100);

        let token = b"BTC".to_vec();
        assert_err!(
            XAssets::move_free_balance(&token, &a, &b, 100),
            AssetErr::InvalidToken
        );

        XAssets::issue(&token, &a, 100).unwrap();
        assert_ok!(XAssets::move_free_balance(&token, &a, &b, 100));
        assert_err!(
            XAssets::move_free_balance(&token, &a, &b, 1000),
            AssetErr::NotEnough
        );

        assert_eq!(XAssets::free_balance(&a, &token), 0);
        assert_eq!(XAssets::free_balance(&b, &token), 100);
    })
}
