// Copyright 2019-2022 ChainX Project Authors. Licensed under GPL-3.0.

use crate::mock::*;
use crate::{to_ascii_hex, EcdsaSignature};
use frame_support::{assert_noop, assert_ok};
use sp_core::{H160, U256};

use ethabi::{Function, Param, ParamType, Token};
use hex_literal::hex;
use std::str::FromStr;

/*
{
  "address": "0xf24ff3a9cf04c71dbc94d0b566f7a27b94566cac",
  "msg": "evm:d43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d",
  "sig": "0x7def4e5806b7cf5dbfa44bc9d14422462dc9fe803c74e5d544db71bdcefc8ba04fc54cd079f2f8a2947f4d3b1c0d9e9f12fa279f6a40828ecc08766b4bab4bb21c",
  "version": "2"
}
*/
const SIGNATURE: [u8; 65] = hex!["7def4e5806b7cf5dbfa44bc9d14422462dc9fe803c74e5d544db71bdcefc8ba04fc54cd079f2f8a2947f4d3b1c0d9e9f12fa279f6a40828ecc08766b4bab4bb21c"];
const EVM_ADDR: [u8; 20] = hex!["f24ff3a9cf04c71dbc94d0b566f7a27b94566cac"];
const SUB_ACCOUNT: &str = "5USGSZK3raH3LD4uxvNTa23HN5VULnYrkXonRktyizTJUYg9";
const PUBKEY: &str = "d43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d";
const ERC20_1: [u8; 20] = [1u8; 20];
const ERC20_2: [u8; 20] = [2u8; 20];
const MAPPING_ACCOUNT: &str = "5Fghzk1AJt88PeFEzuRfXzbPchiBbsVGTTXcdx599VdZzkTA";

pub fn mint_into_abi() -> Function {
    #[allow(deprecated)]
    Function {
        name: "mint_into".to_owned(),
        inputs: vec![
            Param {
                name: "account".to_owned(),
                kind: ParamType::Address,
                internal_type: None,
            },
            Param {
                name: "amount".to_owned(),
                kind: ParamType::Uint(256),
                internal_type: None,
            },
        ],
        outputs: vec![],
        constant: Some(false),
        state_mutability: Default::default(),
    }
}

pub fn burn_from_abi() -> Function {
    #[allow(deprecated)]
    Function {
        name: "burn_from".to_owned(),
        inputs: vec![
            Param {
                name: "account".to_owned(),
                kind: ParamType::Address,
                internal_type: None,
            },
            Param {
                name: "amount".to_owned(),
                kind: ParamType::Uint(256),
                internal_type: None,
            },
        ],
        outputs: vec![],
        constant: Some(false),
        state_mutability: Default::default(),
    }
}

#[test]
fn evm_address_mapping_substrate_account() {
    use sp_core::Hasher;
    let address = H160::from_slice(&EVM_ADDR);

    let mut data = [0u8; 24];
    data[0..4].copy_from_slice(b"evm:");
    data[4..24].copy_from_slice(&address[..]);

    let mapping_account = AccountId32::new(BlakeTwo256::hash(&data).to_fixed_bytes());
    let sub_account: AccountId32 = AccountId32::from_str(MAPPING_ACCOUNT).unwrap();

    assert_eq!(mapping_account, sub_account)
}

#[test]
fn test_to_ascii_hex() {
    let sub_account: AccountId32 = AccountId32::from_str(SUB_ACCOUNT).unwrap();
    let pubkey = String::from_utf8(to_ascii_hex(sub_account.as_ref())).unwrap();

    assert_eq!(&pubkey, PUBKEY);
}

#[test]
fn recover_eth_address() {
    new_test_ext().execute_with(|| {
        let s = EcdsaSignature::from_slice(&SIGNATURE).unwrap();
        let p = PUBKEY.as_bytes();
        let address = crate::eth_recover(&s, p, &[][..]).unwrap();

        assert_eq!(address, H160::from_slice(&EVM_ADDR))
    })
}

#[test]
fn mint_into_abi_encode() {
    #[allow(deprecated)]
    let mint_into = mint_into_abi();

    let account = H160::from_slice(&EVM_ADDR);
    let amount = U256::from(100_000_000);
    let mut uint = [0u8; 32];
    amount.to_big_endian(&mut uint[..]);

    let encoded = mint_into
        .encode_input(&[Token::Address(account), Token::Uint(uint.into())])
        .unwrap();

    let expected = hex!("efe51695000000000000000000000000f24ff3a9cf04c71dbc94d0b566f7a27b94566cac0000000000000000000000000000000000000000000000000000000005f5e100").to_vec();
    assert_eq!(encoded, expected);

    let expected_sig = hex!("efe51695").to_vec();
    assert_eq!(mint_into.short_signature().to_vec(), expected_sig);

    let encoded2 = crate::mint_into_encode(account, 100_000_000u128);
    assert_eq!(encoded2, expected);
}

#[test]
fn burn_from_abi_encode() {
    #[allow(deprecated)]
    let burn_from = burn_from_abi();

    let account = H160::from_slice(&EVM_ADDR);
    let amount = U256::from(100_000_000);
    let mut uint = [0u8; 32];
    amount.to_big_endian(&mut uint[..]);

    let encoded = burn_from
        .encode_input(&[Token::Address(account), Token::Uint(uint.into())])
        .unwrap();

    let expected = hex!("0f536f84000000000000000000000000f24ff3a9cf04c71dbc94d0b566f7a27b94566cac0000000000000000000000000000000000000000000000000000000005f5e100").to_vec();
    assert_eq!(encoded, expected);

    let expected_sig = hex!("0f536f84").to_vec();
    assert_eq!(burn_from.short_signature().to_vec(), expected_sig);

    let encoded2 = crate::burn_from_encode(account, 100_000_000u128);
    assert_eq!(encoded2, expected);
}

#[test]
fn pause_should_work() {
    new_test_ext().execute_with(|| {
        assert_ok!(XAssetsBridge::register(
            Origin::signed(ALICE.into()),
            1,
            H160::from_slice(&ERC20_1)
        ));
        expect_event(XAssetsBridgeEvent::Register(1, H160::from_slice(&ERC20_1)));

        assert_noop!(
            XAssetsBridge::deposit(Origin::signed(BOB.into()), 1, 1),
            Error::<Test>::EthAddressHasNotMapped
        );

        assert_ok!(XAssetsBridge::pause(Origin::signed(ALICE.into()), Some(1)));
        expect_event(XAssetsBridgeEvent::Paused(1));

        assert_noop!(
            XAssetsBridge::deposit(Origin::signed(BOB.into()), 1, 1),
            Error::<Test>::InEmergency
        );
    })
}

#[test]
fn pause_should_not_work() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            XAssetsBridge::pause(Origin::signed(ALICE.into()), Some(1)),
            Error::<Test>::AssetIdHasNotMapped
        );

        assert_ok!(XAssetsBridge::register(
            Origin::signed(ALICE.into()),
            1,
            H160::from_slice(&ERC20_1)
        ));
        expect_event(XAssetsBridgeEvent::Register(1, H160::from_slice(&ERC20_1)));

        assert_noop!(
            XAssetsBridge::pause(Origin::signed(BOB.into()), Some(1)),
            Error::<Test>::RequireAdmin
        );
    })
}

#[test]
fn pause_after_pause_should_work() {
    new_test_ext().execute_with(|| {
        assert_ok!(XAssetsBridge::register(
            Origin::signed(ALICE.into()),
            1,
            H160::from_slice(&ERC20_1)
        ));
        expect_event(XAssetsBridgeEvent::Register(1, H160::from_slice(&ERC20_1)));

        assert_ok!(XAssetsBridge::register(
            Origin::signed(ALICE.into()),
            2,
            H160::from_slice(&ERC20_2)
        ));
        expect_event(XAssetsBridgeEvent::Register(2, H160::from_slice(&ERC20_2)));

        assert_noop!(
            XAssetsBridge::deposit(Origin::signed(BOB.into()), 1, 1),
            Error::<Test>::EthAddressHasNotMapped
        );

        // 1. pause(1)
        assert_ok!(XAssetsBridge::pause(Origin::signed(ALICE.into()), Some(1)));
        expect_event(XAssetsBridgeEvent::Paused(1));
        assert_eq!(XAssetsBridge::emergencies(), vec![1]);

        assert_noop!(
            XAssetsBridge::deposit(Origin::signed(BOB.into()), 1, 1),
            Error::<Test>::InEmergency
        );

        // 2. pause(1)
        assert_ok!(XAssetsBridge::pause(Origin::signed(ALICE.into()), Some(1)));
        expect_event(XAssetsBridgeEvent::Paused(1));
        assert_eq!(XAssetsBridge::emergencies(), vec![1]);

        // 3. pause all
        assert_ok!(XAssetsBridge::pause(Origin::signed(ALICE.into()), None));
        expect_event(XAssetsBridgeEvent::PausedAll);
        assert_eq!(XAssetsBridge::emergencies(), vec![1, 2]);

        // 4. pause(2)
        assert_ok!(XAssetsBridge::pause(Origin::signed(ALICE.into()), Some(2)));

        // 5. pause(3)
        assert_noop!(
            XAssetsBridge::pause(Origin::signed(ALICE.into()), Some(3)),
            Error::<Test>::AssetIdHasNotMapped
        );
    })
}

#[test]
fn unpause_should_work() {
    new_test_ext().execute_with(|| {
        assert_ok!(XAssetsBridge::register(
            Origin::signed(ALICE.into()),
            1,
            H160::from_slice(&ERC20_1)
        ));
        expect_event(XAssetsBridgeEvent::Register(1, H160::from_slice(&ERC20_1)));

        assert_ok!(XAssetsBridge::pause(Origin::signed(ALICE.into()), None));
        expect_event(XAssetsBridgeEvent::PausedAll);

        assert_noop!(
            XAssetsBridge::deposit(Origin::signed(BOB.into()), 1, 1),
            Error::<Test>::InEmergency
        );

        assert_ok!(XAssetsBridge::unpause(
            Origin::signed(ALICE.into()),
            Some(1)
        ));
        expect_event(XAssetsBridgeEvent::UnPaused(1));

        assert_noop!(
            XAssetsBridge::deposit(Origin::signed(BOB.into()), 1, 1),
            Error::<Test>::EthAddressHasNotMapped
        );
    })
}

#[test]
fn unpause_should_not_work() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            XAssetsBridge::unpause(Origin::signed(ALICE.into()), Some(1)),
            Error::<Test>::AssetIdHasNotMapped
        );

        assert_ok!(XAssetsBridge::register(
            Origin::signed(ALICE.into()),
            1,
            H160::from_slice(&ERC20_1)
        ));
        expect_event(XAssetsBridgeEvent::Register(1, H160::from_slice(&ERC20_1)));

        assert_noop!(
            XAssetsBridge::unpause(Origin::signed(BOB.into()), Some(1)),
            Error::<Test>::RequireAdmin
        );
    })
}

#[test]
fn unpause_after_unpause_should_work() {
    new_test_ext().execute_with(|| {
        assert_ok!(XAssetsBridge::register(
            Origin::signed(ALICE.into()),
            1,
            H160::from_slice(&ERC20_1)
        ));
        expect_event(XAssetsBridgeEvent::Register(1, H160::from_slice(&ERC20_1)));

        assert_ok!(XAssetsBridge::register(
            Origin::signed(ALICE.into()),
            2,
            H160::from_slice(&ERC20_2)
        ));
        expect_event(XAssetsBridgeEvent::Register(2, H160::from_slice(&ERC20_2)));

        assert_ok!(XAssetsBridge::unpause(
            Origin::signed(ALICE.into()),
            Some(1)
        ));
        assert!(XAssetsBridge::emergencies().is_empty());

        assert_ok!(XAssetsBridge::unpause(
            Origin::signed(ALICE.into()),
            Some(2)
        ));
        assert!(XAssetsBridge::emergencies().is_empty());

        assert_noop!(
            XAssetsBridge::pause(Origin::signed(ALICE.into()), Some(3)),
            Error::<Test>::AssetIdHasNotMapped
        );
        assert!(XAssetsBridge::emergencies().is_empty());

        assert_ok!(XAssetsBridge::pause(Origin::signed(ALICE.into()), Some(1)));

        assert_eq!(XAssetsBridge::emergencies(), vec![1]);

        assert_ok!(XAssetsBridge::pause(Origin::signed(ALICE.into()), Some(2)));

        assert_eq!(XAssetsBridge::emergencies(), vec![1, 2]);

        assert_noop!(
            XAssetsBridge::deposit(Origin::signed(BOB.into()), 1, 1),
            Error::<Test>::InEmergency
        );

        assert_noop!(
            XAssetsBridge::withdraw(Origin::signed(BOB.into()), 1, 1),
            Error::<Test>::InEmergency
        );

        assert_ok!(XAssetsBridge::unpause(
            Origin::signed(ALICE.into()),
            Some(2)
        ));
        expect_event(XAssetsBridgeEvent::UnPaused(2));

        assert_noop!(
            XAssetsBridge::withdraw(Origin::signed(BOB.into()), 2, 1),
            Error::<Test>::EthAddressHasNotMapped
        );
        assert_eq!(XAssetsBridge::emergencies(), vec![1]);

        assert_ok!(XAssetsBridge::unpause(Origin::signed(ALICE.into()), None));
        expect_event(XAssetsBridgeEvent::UnPausedAll);

        assert_noop!(
            XAssetsBridge::withdraw(Origin::signed(BOB.into()), 1, 1),
            Error::<Test>::EthAddressHasNotMapped
        );
        assert!(XAssetsBridge::emergencies().is_empty());
    })
}

#[test]
fn more_pause_and_unpause_should_work() {
    new_test_ext().execute_with(|| {
        assert_ok!(XAssetsBridge::register(
            Origin::signed(ALICE.into()),
            1,
            H160::from_slice(&ERC20_1)
        ));
        expect_event(XAssetsBridgeEvent::Register(1, H160::from_slice(&ERC20_1)));

        assert_ok!(XAssetsBridge::register(
            Origin::signed(ALICE.into()),
            2,
            H160::from_slice(&ERC20_2)
        ));
        expect_event(XAssetsBridgeEvent::Register(2, H160::from_slice(&ERC20_2)));

        assert!(XAssetsBridge::emergencies().is_empty());

        assert_noop!(
            XAssetsBridge::deposit(Origin::signed(BOB.into()), 1, 1),
            Error::<Test>::EthAddressHasNotMapped
        );

        assert_noop!(
            XAssetsBridge::deposit(Origin::signed(BOB.into()), 2, 1),
            Error::<Test>::EthAddressHasNotMapped
        );

        assert_ok!(XAssetsBridge::pause(Origin::signed(ALICE.into()), None));
        expect_event(XAssetsBridgeEvent::PausedAll);

        assert_eq!(XAssetsBridge::emergencies(), vec![1, 2]);

        assert_noop!(
            XAssetsBridge::deposit(Origin::signed(BOB.into()), 1, 1),
            Error::<Test>::InEmergency
        );

        assert_noop!(
            XAssetsBridge::deposit(Origin::signed(BOB.into()), 2, 1),
            Error::<Test>::InEmergency
        );

        assert_ok!(XAssetsBridge::unpause(
            Origin::signed(ALICE.into()),
            Some(2)
        ));
        expect_event(XAssetsBridgeEvent::UnPaused(2));

        assert_eq!(XAssetsBridge::emergencies(), vec![1]);

        assert_noop!(
            XAssetsBridge::deposit(Origin::signed(BOB.into()), 1, 1),
            Error::<Test>::InEmergency
        );

        assert_noop!(
            XAssetsBridge::deposit(Origin::signed(BOB.into()), 2, 1),
            Error::<Test>::EthAddressHasNotMapped
        );

        assert_ok!(XAssetsBridge::unpause(Origin::signed(ALICE.into()), None));
        expect_event(XAssetsBridgeEvent::UnPausedAll);

        assert!(XAssetsBridge::emergencies().is_empty());

        assert_noop!(
            XAssetsBridge::deposit(Origin::signed(BOB.into()), 1, 1),
            Error::<Test>::EthAddressHasNotMapped
        );
    })
}

#[test]
fn force_unregister_should_work() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            XAssetsBridge::force_unregister(Origin::root(), 1),
            Error::<Test>::AssetIdHasNotMapped
        );

        assert_eq!(XAssetsBridge::erc20s(1), None);
        assert_eq!(XAssetsBridge::asset_ids(H160::from_slice(&ERC20_1)), None);

        assert_ok!(XAssetsBridge::register(
            Origin::signed(ALICE.into()),
            1,
            H160::from_slice(&ERC20_1)
        ));
        expect_event(XAssetsBridgeEvent::Register(1, H160::from_slice(&ERC20_1)));

        assert_eq!(XAssetsBridge::erc20s(1), Some(H160::from_slice(&ERC20_1)));
        assert_eq!(
            XAssetsBridge::asset_ids(H160::from_slice(&ERC20_1)),
            Some(1)
        );

        assert_ok!(XAssetsBridge::pause(Origin::signed(ALICE.into()), None));
        expect_event(XAssetsBridgeEvent::PausedAll);
        assert_eq!(XAssetsBridge::emergencies(), vec![1]);

        assert_ok!(XAssetsBridge::force_unregister(Origin::root(), 1));
        expect_event(XAssetsBridgeEvent::ForceUnRegister(
            1,
            H160::from_slice(&ERC20_1),
        ));

        assert_eq!(XAssetsBridge::erc20s(1), None);
        assert_eq!(XAssetsBridge::asset_ids(H160::from_slice(&ERC20_1)), None);
        assert!(XAssetsBridge::emergencies().is_empty());
    })
}

#[test]
fn bridge_accounts_should_equal() {
    // 5TPu4DCQRSbNS9ESUcNGUn9HcF9AzrHiDP395bDxM9ZAqSD8
    let bridge_admin1 = hex!["a62add1af3bcf9256aa2def0fea1b9648cb72517ccee92a891dc2903a9093e52"];
    let bridge_admin2 = [
        166u8, 42, 221, 26, 243, 188, 249, 37, 106, 162, 222, 240, 254, 161, 185, 100, 140, 183,
        37, 23, 204, 238, 146, 168, 145, 220, 41, 3, 169, 9, 62, 82,
    ];

    assert_eq!(bridge_admin1, bridge_admin2);
}
