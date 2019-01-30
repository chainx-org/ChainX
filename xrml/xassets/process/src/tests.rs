// Copyright 2018 Chainpool.

extern crate xrml_xaccounts;

use substrate_primitives::{Blake2Hasher, H256};

use runtime_io;
use runtime_io::with_externalities;
use runtime_primitives::testing::{Digest, DigestItem, Header, UintAuthorityId};
use runtime_primitives::traits::{BlakeTwo256, IdentityLookup};
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
    type Lookup = IdentityLookup<u64>;
    type Header = Header;
    type Event = ();
    type Log = DigestItem;
}

impl balances::Trait for Test {
    type Balance = u64;
    type OnFreeBalanceZero = ();
    type OnNewAccount = ();
    type EnsureAccountLiquid = ();
    type Event = ();
}

impl consensus::Trait for Test {
    type Log = DigestItem;
    type SessionKey = UintAuthorityId;
    type InherentOfflineReport = ();
}

impl timestamp::Trait for Test {
    type Moment = u64;
    type OnTimestampSet = ();
}

pub struct MockDeterminator;

impl xrml_xaccounts::IntentionJackpotAccountIdFor<u64> for MockDeterminator {
    fn accountid_for(_: &u64) -> u64 {
        1000
    }
}

impl xrml_xaccounts::Trait for Test {
    type Event = ();
    type DetermineIntentionJackpotAccountId = MockDeterminator;
}

impl xassets::Trait for Test {
    type Event = ();
    type OnAssetChanged = ();
    type OnAssetRegisterOrRevoke = ();
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
        }
        .build_storage()
        .unwrap()
        .0,
    );
    // token balance
    let btc_asset = Asset::new(
        b"BTC".to_vec(),     // token
        b"Bitcoin".to_vec(), // token
        Chain::Bitcoin,
        8, // bitcoin precision
        b"BTC chainx".to_vec(),
    )
    .unwrap();

    // token balance
    let xdot_asset = Asset::new(
        b"XDOT".to_vec(), // token
        b"XDOT".to_vec(), // token
        Chain::Ethereum,
        3,
        b"XDOT chainx".to_vec(),
    )
    .unwrap();

    // bridge btc
    r.extend(
        xbitcoin::GenesisConfig::<Test> {
            // start genesis block: (genesis, blocknumber)
            genesis: Default::default(),
            params_info: Default::default(),
            network_id: 1,
            irr_block: 3,
            reserved: 2100,
            btc_fee: 10,
            max_withdraw_amount: 100,
            cert_address: Default::default(),
            cert_redeem_script: b"522102e34d10113f2dd162e8d8614a4afbb8e2eb14eddf4036042b35d12cf5529056a2210311252930af8ba766b9c7a6580d8dc4bbf9b0befd17a8ef7fabac275bba77ae402103ece1a20b5468b12fd7beda3e62ef6b2f6ad9774489e9aff1c8bc684d87d7078053ae".to_vec(),
            trustee_address: Default::default(),
            trustee_redeem_script: b"52210311252930af8ba766b9c7a6580d8dc4bbf9b0befd17a8ef7fabac275bba77ae402102e34d10113f2dd162e8d8614a4afbb8e2eb14eddf4036042b35d12cf5529056a221023e505c48a955e759ce61145dc4a9a7447425290b8483f4e36f05169e7967c86d53ae".to_vec(),
            _genesis_phantom_data: Default::default(),
        }.build_storage()
            .unwrap()
            .0,
    );

    r.extend(
        GenesisConfig::<Test> {
            token_black_list: vec![xdot_asset.token()],
            _genesis_phantom_data: Default::default(),
        }
        .build_storage()
        .unwrap()
        .0,
    );

    r.extend(
        xassets::GenesisConfig::<Test> {
            pcx: (b"PlokadotChainX".to_vec(), 3, b"PCX onchain token".to_vec()),
            memo_len: 128,
            // asset, is_psedu_intention, init for account
            // Vec<(Asset, bool, Vec<(T::AccountId, u64)>)>;
            asset_list: vec![
                (btc_asset, true, vec![(3, 100)]),
                (xdot_asset, true, vec![(3, 100)]),
            ],
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
            assert_ok!(xrecords::Module::<Test>::withdrawal_finish(n, true));
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

#[test]
fn test_check_min_withdrawal() {
    with_externalities(&mut new_test_ext(), || {
        assert_ok!(xassets::Module::<Test>::issue(&b"BTC".to_vec(), &1, 1000));

        // less
        let origin = system::RawOrigin::Signed(1).into();
        assert_err!(
            Module::<Test>::withdraw(
                origin,
                b"BTC".to_vec(),
                5,
                b"mjKE11gjVN4JaC9U8qL6ZB5vuEBgmwik7b".to_vec(),
                b"".to_vec()
            ),
            "withdrawal value should larger than requirement"
        );
        // equal
        let origin = system::RawOrigin::Signed(1).into();
        assert_err!(
            Module::<Test>::withdraw(
                origin,
                b"BTC".to_vec(),
                10,
                b"mjKE11gjVN4JaC9U8qL6ZB5vuEBgmwik7b".to_vec(),
                b"".to_vec()
            ),
            "withdrawal value should larger than requirement"
        );
        // success
        let origin = system::RawOrigin::Signed(1).into();
        assert_ok!(Module::<Test>::withdraw(
            origin,
            b"BTC".to_vec(),
            11,
            b"mjKE11gjVN4JaC9U8qL6ZB5vuEBgmwik7b".to_vec(),
            b"".to_vec()
        ));
    });
}

#[test]
fn test_check_blacklist() {
    with_externalities(&mut new_test_ext(), || {
        assert_ok!(xassets::Module::<Test>::issue(&b"BTC".to_vec(), &1, 1000));

        // success
        let origin = system::RawOrigin::Signed(1).into();
        assert_ok!(Module::<Test>::withdraw(
            origin,
            b"BTC".to_vec(),
            11,
            b"mjKE11gjVN4JaC9U8qL6ZB5vuEBgmwik7b".to_vec(),
            b"".to_vec()
        ));

        // failed
        let origin = system::RawOrigin::Signed(1).into();
        assert_err!(
            Module::<Test>::withdraw(
                origin,
                b"XDOT".to_vec(),
                11,
                b"xxx".to_vec(),
                b"xxx".to_vec()
            ),
            "this token is in blacklist"
        );

        // failed
        let origin = system::RawOrigin::Signed(1).into();
        assert_err!(
            Module::<Test>::withdraw(
                origin,
                b"PCX".to_vec(),
                11,
                b"xxx".to_vec(),
                b"xxx".to_vec()
            ),
            "Can\'t withdraw the asset on ChainX"
        );
    });
}
