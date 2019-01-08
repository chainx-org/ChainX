// Copyright 2018 Chainpool.

use substrate_primitives::{Blake2Hasher, H256};

use runtime_io;
use runtime_io::with_externalities;
use runtime_primitives::testing::{Digest, DigestItem, Header};
use runtime_primitives::traits::BlakeTwo256;
use runtime_primitives::BuildStorage;

use super::*;
use xassets::{Asset, Chain};

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

impl consensus::Trait for Test {
    const NOTE_OFFLINE_POSITION: u32 = 1;
    type Log = DigestItem;
    type SessionKey = u64;
    type InherentOfflineReport = ();
}

impl timestamp::Trait for Test {
    const TIMESTAMP_SET_POSITION: u32 = 0;
    type Moment = u64;
    type OnTimestampSet = ();
}

impl xassets::Trait for Test {
    type Event = ();
    type OnAssetChanged = ();
    type OnAssetRegistration = ();
}

impl xrecords::Trait for Test {
    type Event = ();
}

impl xbitcoin::Trait for Test {
    type Event = ();
}

impl Trait for Test {}

// This function basically just builds a genesis storage key/value store according to
// our desired mockup.
pub fn new_test_ext() -> runtime_io::TestExternalities<Blake2Hasher> {
    let mut r = system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap()
        .0;
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
        .unwrap()
        .0,
    );
    // token balance
    let btc_asset = Asset::new(
        b"BTC".to_vec(), // token
        Chain::Bitcoin,
        8, // bitcoin precision
        b"BTC chainx".to_vec(),
    )
    .unwrap();

    r.extend(
        xassets::GenesisConfig::<Test> {
            pcx: (3, b"PCX onchain token".to_vec()),
            memo_len: 128,
            // asset, is_psedu_intention, init for account
            // Vec<(Asset, bool, Vec<(T::AccountId, u64)>)>;
            asset_list: vec![(btc_asset, true, vec![(3, 100)])],
        }
        .build_storage()
        .unwrap()
        .0,
    );

    r.into()
}

#[test]
fn test_check_btc_addr() {
    with_externalities(&mut new_test_ext(), || {
        assert_ok!(xrecords::Module::<Test>::deposit(
            &1,
            &b"BTC".to_vec(),
            1000
        ));

        let origin = system::RawOrigin::Signed(1).into();
        assert_err!(
            Module::<Test>::withdraw(
                origin,
                b"BTC".to_vec(),
                100,
                b"sdfds".to_vec(),
                b"".to_vec()
            ),
            "verify btc addr err"
        );

        let origin = system::RawOrigin::Signed(1).into();
        assert_ok!(Module::<Test>::withdraw(
            origin,
            b"BTC".to_vec(),
            100,
            b"mjKE11gjVN4JaC9U8qL6ZB5vuEBgmwik7b".to_vec(),
            b"".to_vec()
        ));

        assert_eq!(
            xassets::Module::<Test>::free_balance(&1, &b"BTC".to_vec()),
            900
        );

        let nums =
            xrecords::Module::<Test>::withdrawal_application_numbers(Chain::Bitcoin, 10).unwrap();
        for n in nums {
            assert_ok!(xrecords::Module::<Test>::withdrawal_finish(n));
        }
        assert_eq!(
            xassets::Module::<Test>::all_type_balance_of(&1, &b"BTC".to_vec()),
            900
        )
    })
}

#[test]
fn test_check_btc_addr2() {
    with_externalities(&mut new_test_ext(), || {
        let r = Module::<Test>::verify_addr(
            &xbitcoin::Module::<Test>::TOKEN.to_vec(),
            b"2N8tR484JD32i1DY2FnRPLwBVaNuXSfzoAv",
            b"",
        );
        assert_eq!(r, Ok(()));

        let r = Module::<Test>::verify_addr(
            &xbitcoin::Module::<Test>::TOKEN.to_vec(),
            b"mjKE11gjVN4JaC9U8qL6ZB5vuEBgmwik7b",
            b"",
        );
        assert_eq!(r, Ok(()));
    })
}
