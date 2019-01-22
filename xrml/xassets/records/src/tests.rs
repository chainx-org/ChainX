// Copyright 2018 Chainpool.

extern crate srml_consensus;

use substrate_primitives::{Blake2Hasher, H256};

use runtime_io;
use runtime_io::with_externalities;
use runtime_primitives::testing::{Digest, DigestItem, Header, UintAuthorityId};
use runtime_primitives::traits::BlakeTwo256;
use runtime_primitives::BuildStorage;

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

impl srml_consensus::Trait for Test {
    const NOTE_OFFLINE_POSITION: u32 = 1;
    type Log = DigestItem;
    type SessionKey = UintAuthorityId;
    type InherentOfflineReport = ();
}

impl timestamp::Trait for Test {
    const TIMESTAMP_SET_POSITION: u32 = 0;
    type Moment = u64;
    type OnTimestampSet = ();
}

// assets
impl xassets::Trait for Test {
    type Event = ();
    type OnAssetChanged = ();
    type OnAssetRegisterOrRevoke = ();
}

impl Trait for Test {
    type Event = ();
}

pub fn new_test_ext() -> runtime_io::TestExternalities<Blake2Hasher> {
    let mut r = system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap()
        .0;
    // balance
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
        .unwrap()
        .0,
    );

    let btc_asset = xassets::Asset::new(
        b"BTC".to_vec(),     // token
        b"Bitcoin".to_vec(), // token
        Chain::Bitcoin,
        8, // bitcoin precision
        b"BTC chainx".to_vec(),
    )
    .unwrap();

    let eth_asset = xassets::Asset::new(
        b"ETH".to_vec(),      // token
        b"Ethereum".to_vec(), // token
        Chain::Ethereum,
        8, // bitcoin precision
        b"ETH chainx".to_vec(),
    )
    .unwrap();

    r.extend(
        xassets::GenesisConfig::<Test> {
            pcx: (b"PolkadotChainX".to_vec(), 3, b"PCX onchain token".to_vec()),
            memo_len: 128,
            // asset, is_psedu_intention, init for account
            // Vec<(Asset, bool, Vec<(T::AccountId, u64)>)>;
            asset_list: vec![
                (btc_asset, true, vec![(3, 100)]),
                (eth_asset, true, vec![(3, 100)]),
            ],
        }
        .build_storage()
        .unwrap()
        .0,
    );

    r.into()
}

type Records = Module<Test>;
type XAssets = xassets::Module<Test>;

#[test]
fn test_normal() {
    with_externalities(&mut new_test_ext(), || {
        let a: u64 = 1; // accountid
        let btc_token = b"BTC".to_vec();

        // deposit
        assert_ok!(Records::deposit(&a, &btc_token, 100));
        assert_eq!(XAssets::free_balance(&a, &btc_token), 100);

        // withdraw
        assert_ok!(Records::withdrawal(
            &a,
            &btc_token,
            50,
            b"addr".to_vec(),
            b"ext".to_vec()
        ));

        let numbers = Records::withdrawal_application_numbers(Chain::Bitcoin, 10).unwrap();
        assert_eq!(numbers.len(), 1);

        for i in numbers {
            assert_ok!(Records::withdrawal_finish(i, true));
        }
        assert_eq!(XAssets::free_balance(&a, &btc_token), 50);
    })
}

#[test]
fn test_normal2() {
    with_externalities(&mut new_test_ext(), || {
        let a: u64 = 1; // accountid
        let btc_token = b"BTC".to_vec();
        let eth_token = b"ETH".to_vec();

        // deposit
        assert_ok!(Records::deposit(&a, &btc_token, 100));
        assert_eq!(XAssets::free_balance(&a, &btc_token), 100);
        assert_ok!(Records::deposit(&a, &eth_token, 500));
        assert_eq!(XAssets::free_balance(&a, &eth_token), 500);

        // withdraw
        assert_ok!(Records::withdrawal(
            &a,
            &btc_token,
            50,
            b"addr".to_vec(),
            b"ext".to_vec()
        ));
        // withdrawal twice at once
        assert_ok!(Records::withdrawal(
            &a,
            &eth_token,
            100,
            b"addr".to_vec(),
            b"ext".to_vec()
        ));
        assert_ok!(Records::withdrawal(
            &a,
            &eth_token,
            50,
            b"addr".to_vec(),
            b"ext".to_vec()
        ));

        let mut numbers1 = Records::withdrawal_application_numbers(Chain::Bitcoin, 10).unwrap();
        assert_eq!(numbers1.len(), 1);

        let numbers2 = Records::withdrawal_application_numbers(Chain::Ethereum, 10).unwrap();
        assert_eq!(numbers2.len(), 2);

        numbers1.extend(numbers2);

        for i in numbers1 {
            assert_ok!(Records::withdrawal_finish(i, true));
        }
        assert_eq!(XAssets::free_balance(&a, &btc_token), 50);
        assert_eq!(XAssets::free_balance(&a, &eth_token), 500 - 50 - 100);
    })
}

#[test]
fn test_withdrawal_larger() {
    with_externalities(&mut new_test_ext(), || {
        let a: u64 = 1; // accountid
        let btc_token = b"BTC".to_vec();
        assert_ok!(Records::deposit(&a, &btc_token, 10));

        assert_err!(
            Records::withdrawal(&a, &btc_token, 50, b"addr".to_vec(), b"ext".to_vec()),
            "balance too low for this account"
        );
    })
}

#[test]
fn test_withdrawal_chainx() {
    with_externalities(&mut new_test_ext(), || {
        let a: u64 = 1; // accountid
        let chainx_token = XAssets::TOKEN.to_vec();
        assert_err!(
            Records::deposit(&a, &chainx_token, 10),
            "can\'t deposit/withdrawal chainx token"
        );

        assert_err!(
            Records::withdrawal(&a, &chainx_token, 50, b"addr".to_vec(), b"ext".to_vec()),
            "can\'t deposit/withdrawal chainx token"
        );
    })
}

#[test]
fn test_withdrawal_first() {
    with_externalities(&mut new_test_ext(), || {
        let a: u64 = 1; // accountid
        let btc_token = b"BTC".to_vec();
        assert_err!(
            Records::withdrawal(&a, &btc_token, 50, vec![], vec![]),
            "not a valid token for this account"
        );
    })
}
