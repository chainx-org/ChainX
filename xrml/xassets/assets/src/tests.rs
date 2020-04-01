// Copyright 2018-2019 Chainpool.

#![cfg(test)]

use super::mock::*;
use super::*;

use runtime_io::with_externalities;
use support::{assert_err, assert_noop, assert_ok};

#[test]
fn test_genesis() {
    with_externalities(&mut new_test_ext(), || {
        // Check that GenesisBuilder works properly.
        assert_eq!(Indices::lookup_index(0), Some(1));
        assert_eq!(Indices::lookup_index(1), Some(2));
        assert_eq!(Indices::lookup_index(2), Some(3));

        assert_eq!(XAssets::pcx_free_balance(&1), 1000);
        assert_eq!(XAssets::pcx_free_balance(&2), 510);
        assert_eq!(XAssets::pcx_free_balance(&3), 1000);

        // check token_list
        let btc_token = b"BTC".to_vec();

        assert_eq!(
            XAssets::assets(),
            vec![XAssets::TOKEN.to_vec(), btc_token.clone(),]
        );
        assert_eq!(XAssets::asset_info(&btc_token).unwrap().0.precision(), 8);
    });
}

#[test]
fn test_genesis_token_issue() {
    with_externalities(&mut new_test_ext(), || {
        let btc_token = b"BTC".to_vec();
        let chainx_token = XAssets::TOKEN.to_vec();
        assert_eq!(
            XAssets::asset_balance_of(&1, &chainx_token, AssetType::Free),
            1000
        );
        assert_eq!(Indices::lookup_index(0), Some(1));
        assert_eq!(
            XAssets::asset_balance_of(&2, &chainx_token, AssetType::Free),
            510
        );
        assert_eq!(Indices::lookup_index(1), Some(2));
        assert_eq!(
            XAssets::asset_balance_of(&3, &chainx_token, AssetType::Free),
            1000
        );
        assert_eq!(Indices::lookup_index(2), Some(3));
        assert_eq!(
            XAssets::asset_balance_of(&3, &btc_token, AssetType::Free),
            100
        );
    })
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
        assert_eq!(XAssets::register_asset(asset.clone(), true, false), Ok(()));

        let btc_token = b"BTC".to_vec(); //b"BTC".to_vec();

        assert_eq!(
            XAssets::assets(),
            vec![XAssets::TOKEN.to_vec(), btc_token, token.clone()]
        );

        assert_eq!(XAssets::all_type_total_asset_balance(&token), 0);
        assert_eq!(XAssets::asset_info(&token).unwrap().0.precision(), 4);
        assert_noop!(
            XAssets::register_asset(asset, true, false),
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
        assert_eq!(XAssets::register_asset(asset.clone(), true, false), Ok(()));

        // remove it
        assert_eq!(XAssets::revoke_asset(token.clone()), Ok(()));
        assert_noop!(XAssets::is_valid_asset(&token), "not a valid token");

        // re-register, but must be failed
        assert_noop!(
            XAssets::register_asset(asset, true, false),
            "already has this token"
        );
    })
}

#[test]
fn test_total_balance() {
    with_externalities(&mut new_test_ext(), || {
        let btc_token = b"BTC".to_vec();
        assert_eq!(XAssets::all_type_total_asset_balance(&btc_token), 100);

        XAssets::issue(&btc_token, &0, 100).unwrap();
        assert_eq!(XAssets::all_type_total_asset_balance(&btc_token), 200);

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
        assert_eq!(XAssets::all_type_total_asset_balance(&btc_token), 250);
        assert_eq!(
            XAssets::asset_balance_of(&0, &btc_token, AssetType::ReservedWithdrawal),
            50
        );

        XAssets::destroy(&btc_token, &0, 25).unwrap();
        assert_eq!(XAssets::all_type_total_asset_balance(&btc_token), 225);
        // chainx total
        let token = XAssets::TOKEN.to_vec();
        assert_eq!(
            XAssets::all_type_total_asset_balance(&token),
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
            XAssets::all_type_total_asset_balance(&token),
            1000 + 510 + 1000
        );
        assert_eq!(
            XAssets::asset_balance_of(&1, &token, AssetType::ReservedWithdrawal),
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
            XAssets::all_type_total_asset_balance(&token),
            1000 + 510 + 1000
        );
        assert_eq!(
            XAssets::asset_balance_of(&1, &token, AssetType::ReservedWithdrawal),
            25
        );
    })
}

#[test]
fn test_account_balance() {
    with_externalities(&mut new_test_ext(), || {
        let a: u64 = 1; // accountid
        let btc_token = b"BTC".to_vec();
        assert_eq!(XAssets::free_balance_of(&a, &btc_token), 0);
        assert_eq!(
            XAssets::asset_balance_of(&a, &btc_token, AssetType::ReservedWithdrawal),
            0
        );
        assert_eq!(XAssets::all_type_asset_balance(&a, &btc_token), 0);

        XAssets::issue(&btc_token, &a, 100).unwrap();
        assert_eq!(XAssets::free_balance_of(&a, &btc_token), 100);
        assert_eq!(
            XAssets::asset_balance_of(&a, &btc_token, AssetType::ReservedWithdrawal),
            0
        );
        assert_eq!(XAssets::all_type_asset_balance(&a, &btc_token), 100);

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
        assert_eq!(XAssets::all_type_asset_balance(&a, &btc_token), 50);
    })
}

#[test]
fn test_normal_issue_and_destroy() {
    with_externalities(&mut new_test_ext(), || {
        let a: u64 = 1; // accountid
        let btc_token = b"BTC".to_vec();

        // issue
        XAssets::issue(&btc_token, &a, 50).unwrap();
        assert_eq!(XAssets::all_type_asset_balance(&a, &btc_token), 50);
        assert_eq!(XAssets::all_type_total_asset_balance(&btc_token), 150);

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
            XAssets::asset_balance_of(&a, &btc_token, AssetType::ReservedWithdrawal),
            25
        );
        assert_eq!(XAssets::free_balance_of(&a, &btc_token), 25);
        assert_eq!(XAssets::all_type_asset_balance(&a, &btc_token), 50);

        // destroy
        XAssets::destroy(&btc_token, &a, 25).unwrap();
        assert_eq!(
            XAssets::asset_balance_of(&a, &btc_token, AssetType::ReservedWithdrawal),
            0
        );
        assert_eq!(XAssets::free_balance_of(&a, &btc_token), 25);
        assert_eq!(XAssets::all_type_asset_balance(&a, &btc_token), 25);
        assert_eq!(XAssets::all_type_total_asset_balance(&btc_token), 125);
    })
}

#[test]
fn test_unlock_issue_and_destroy2() {
    with_externalities(&mut new_test_ext(), || {
        let a: u64 = 1; // accountid
        let btc_token = b"BTC".to_vec();

        // issue
        XAssets::issue(&btc_token, &a, 50).unwrap();
        assert_eq!(XAssets::all_type_asset_balance(&a, &btc_token), 50);
        assert_eq!(XAssets::all_type_total_asset_balance(&btc_token), 150);

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
            XAssets::asset_balance_of(&a, &btc_token, AssetType::ReservedWithdrawal),
            25
        );
        assert_eq!(XAssets::free_balance_of(&a, &btc_token), 25);
        assert_eq!(XAssets::all_type_asset_balance(&a, &btc_token), 50);

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
            XAssets::asset_balance_of(&a, &btc_token, AssetType::ReservedWithdrawal),
            15
        );
        assert_eq!(XAssets::free_balance_of(&a, &btc_token), 35);
        assert_eq!(XAssets::all_type_asset_balance(&a, &btc_token), 50);
    })
}

#[test]
fn test_error_issue_and_destroy1() {
    with_externalities(&mut new_test_ext(), || {
        let a: u64 = 1; // accountid
        let btc_token = b"BTC".to_vec();
        // issue
        XAssets::issue(&btc_token, &a, 50).unwrap();
        assert_eq!(XAssets::all_type_asset_balance(&a, &btc_token), 50);
        assert_eq!(XAssets::all_type_total_asset_balance(&btc_token), 150);
        // destroy first
        // destroy
        assert_err!(
            XAssets::destroy(&btc_token, &a, 25),
            "current balance too low to destroy"
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
        XAssets::move_balance(
            &btc_token,
            &a,
            AssetType::Free,
            &a,
            AssetType::ReservedWithdrawal,
            25,
        )
        .unwrap();
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
        assert_eq!(XAssets::all_type_asset_balance(&a, &btc_token), 50);
        assert_eq!(XAssets::all_type_total_asset_balance(&btc_token), 150);
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
            "current balance too high to issue"
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
            "current balance too low to destroy"
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

        XAssets::issue(&btc_token, &a, 0).unwrap();
        assert_err!(
            XAssets::destroy(&btc_token, &a, 25),
            "current balance too low to destroy"
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

        XAssets::move_balance(
            &btc_token,
            &a,
            AssetType::Free,
            &a,
            AssetType::ReservedWithdrawal,
            25,
        )
        .unwrap();

        assert_ok!(XAssets::destroy(&btc_token, &a, 25));
    })
}

#[test]
fn test_balance_btree_map() {
    with_externalities(&mut new_test_ext(), || {
        let a: u64 = 100; // accountid
        let b: u64 = 200;
        let token = XAssets::TOKEN.to_vec();
        let key = (a, token.clone());
        assert_eq!(XAssets::pcx_total_balance(), 2510);

        let _ = XAssets::pcx_issue(&a, 100);
        let _ = XAssets::pcx_move_balance(&a, AssetType::Free, &a, AssetType::GasPayment, 30);
        assert_eq!(AssetBalance::<Test>::get(&key).len(), 2);
        assert_eq!(TotalAssetBalance::<Test>::get(&token).len(), 2);

        let _ = XAssets::pcx_move_balance(&a, AssetType::GasPayment, &a, AssetType::Free, 10);
        let _ = XAssets::pcx_move_balance(&a, AssetType::GasPayment, &b, AssetType::Free, 20);
        assert_eq!(AssetBalance::<Test>::get(&key).len(), 1);
        assert_eq!(TotalAssetBalance::<Test>::get(&token).len(), 1);
        assert_eq!(XAssets::pcx_free_balance(&a), 80);
        assert_eq!(XAssets::pcx_free_balance(&b), 20);
        assert_eq!(XAssets::pcx_total_balance(), 2610); // 2510 + 100
    })
}

#[test]
fn test_compatible_balance_btree_map() {
    with_externalities(&mut new_test_ext(), || {
        let a: u64 = 100; // accountid
        let token = XAssets::TOKEN.to_vec();
        let key = (a, token.clone());
        // old version
        let _ = XAssets::pcx_issue(&a, 100);
        AssetBalance::<Test>::mutate(&key, |b| {
            b.insert(AssetType::ReservedDexSpot, Zero::zero());
        });

        assert_eq!(AssetBalance::<Test>::get(&key).len(), 2);

        // what ever operation to reset it
        let _ = XAssets::pcx_move_balance(&a, AssetType::Free, &a, AssetType::ReservedDexSpot, 10);
        // would remove `ReservedDexSpot` Zero item
        let _ = XAssets::pcx_move_balance(&a, AssetType::ReservedDexSpot, &a, AssetType::Free, 10);

        assert_eq!(AssetBalance::<Test>::get(&key).len(), 1);
        assert_eq!(TotalAssetBalance::<Test>::get(&token).len(), 1);
    })
}

#[test]
fn test_account_init() {
    with_externalities(&mut new_test_ext(), || {
        let a: u64 = 1; // accountid
        let id1: u64 = 1000;
        let id2: u64 = 1001;
        let id3: u64 = 1002;
        let btc_token = b"BTC".to_vec();
        let chainx_token = XAssets::TOKEN.to_vec();
        XAssets::issue(&btc_token, &a, 100).unwrap();

        // issue init
        XAssets::issue(&btc_token, &id1, 100).unwrap();
        assert_eq!(Indices::lookup_index(3), Some(id1));
        // transfer pcx init
        assert_ok!(XAssets::transfer(
            Some(a).into(),
            id2.into(),
            chainx_token.clone(),
            25,
            b"".to_vec()
        ));
        assert_eq!(Indices::lookup_index(4), Some(id2));
        // transfer token init
        assert_ok!(XAssets::transfer(
            Some(a).into(),
            id3.into(),
            btc_token.clone(),
            25,
            b"".to_vec()
        ));
        assert_eq!(Indices::lookup_index(5), Some(id3));
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
        assert_eq!(Indices::lookup_index(3), Some(new_id));
        assert_ok!(XAssets::transfer(
            Some(a).into(),
            new_id.into(),
            btc_token.clone(),
            25,
            b"".to_vec()
        ));
        assert_eq!(Indices::lookup_index(4), None);
        assert_ok!(XAssets::transfer(
            Some(a).into(),
            new_id.into(),
            chainx_token.clone(),
            25,
            b"".to_vec()
        ));
        assert_eq!(Indices::lookup_index(4), None);
        assert_eq!(XAssets::free_balance_of(&a, &chainx_token), 1000 - 25);
        assert_eq!(XAssets::free_balance_of(&new_id, &chainx_token), 25);
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

        assert_eq!(XAssets::free_balance_of(&a, &chainx_token), 1000 - 25);
        assert_eq!(XAssets::free_balance_of(&b, &chainx_token), 510 + 25);

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
        assert_eq!(XAssets::all_type_asset_balance(&a, &btc_token), 25);
        assert_eq!(XAssets::free_balance_of(&b, &btc_token), 25);

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

        assert_eq!(XAssets::all_type_asset_balance(&a, &btc_token), 50);
    })
}

/*
#[test]
fn test_set_token() {
    with_externalities(&mut new_test_ext(), || {
        let a: u64 = 1; // accountid
        let btc_token = b"BTC".to_vec();
        XAssets::issue(&btc_token, &a, 50).unwrap();
        assert_ok!(XAssets::set_balance(a.into(), XAssets::TOKEN.to_vec(), b));
        assert_eq!(XAssets::free_balance_of(&a, &btc_token), 500);

        assert_ok!(XAssets::set_balance(a.into(), btc_token.clone(), b));
        assert_eq!(XAssets::free_balance_of(&a, &btc_token), 500);
        assert_eq!(XAssets::all_type_total_asset_balance(&btc_token), 500 + 100);

        assert_ok!(XAssets::set_balance(a.into(), btc_token.clone(), b));
        assert_eq!(XAssets::free_balance_of(&a, &btc_token), 600);
        assert_eq!(XAssets::all_type_total_asset_balance(&btc_token), 600 + 100);
    })
}*/

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
            "Token can only use ASCII alphanumeric character or \'-\', \'.\', \'|\', \'~\'."
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
            "Token can only use ASCII alphanumeric character or \'-\', \'.\', \'|\', \'~\'."
        )
    })
}

#[test]
fn test_chainx() {
    with_externalities(&mut new_test_ext(), || {
        let a: u64 = 1; // accountid
        let token = XAssets::TOKEN.to_vec();
        assert_ok!(XAssets::issue(&token, &a, 100));

        assert_eq!(XAssets::free_balance_of(&a, &token), 1100);

        XAssets::move_balance(
            &token,
            &a,
            AssetType::Free,
            &a,
            AssetType::ReservedWithdrawal,
            100,
        )
        .unwrap();

        assert_eq!(XAssets::free_balance_of(&a, &token), 1000);
        assert_eq!(
            XAssets::asset_balance_of(&a, &token, AssetType::ReservedWithdrawal),
            100
        );

        XAssets::move_balance(
            &token,
            &a,
            AssetType::ReservedWithdrawal,
            &a,
            AssetType::Free,
            50,
        )
        .unwrap();

        assert_eq!(XAssets::free_balance_of(&a, &token), 1050);
        assert_eq!(
            XAssets::asset_balance_of(&a, &token, AssetType::ReservedWithdrawal),
            50
        );
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
        XAssets::move_free_balance(&token, &a, &b, 100).unwrap();
        assert_err!(
            XAssets::move_free_balance(&token, &a, &b, 1000),
            AssetErr::NotEnough
        );
        assert_eq!(XAssets::free_balance_of(&a, &token), 900);
        assert_eq!(XAssets::free_balance_of(&b, &token), 510 + 100);

        let token = b"BTC".to_vec();
        assert_err!(
            XAssets::move_free_balance(&token, &a, &b, 100),
            AssetErr::NotEnough
        );

        XAssets::issue(&token, &a, 100).unwrap();
        XAssets::move_free_balance(&token, &a, &b, 100).unwrap();
        assert_err!(
            XAssets::move_free_balance(&token, &a, &b, 1000),
            AssetErr::NotEnough
        );

        assert_eq!(XAssets::free_balance_of(&a, &token), 0);
        assert_eq!(XAssets::free_balance_of(&b, &token), 100);
    })
}
