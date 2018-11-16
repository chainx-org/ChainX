use substrate_primitives::{Blake2Hasher, H256};

use runtime_io;
use runtime_io::with_externalities;
use runtime_primitives::testing::{Digest, DigestItem, Header};
use runtime_primitives::traits::BlakeTwo256;
use runtime_primitives::BuildStorage;

use super::*;
use tokenbalances::{DescString, SymbolString, Token};

use base58::FromBase58;

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

impl consensus::Trait for Test {
    const NOTE_OFFLINE_POSITION: u32 = 1;
    type Log = DigestItem;
    type SessionKey = u64;
    type OnOfflineValidator = ();
}

impl timestamp::Trait for Test {
    const TIMESTAMP_SET_POSITION: u32 = 0;
    type Moment = u64;
}

// define tokenbalances module type
pub type TokenBalance = u128;

impl tokenbalances::Trait for Test {
    const CHAINX_SYMBOL: SymbolString = b"pcx";
    const CHAINX_TOKEN_DESC: DescString = b"this is pcx for mock";
    type TokenBalance = TokenBalance;
    type Event = ();
}

impl financialrecords::Trait for Test {
    type Event = ();
}

impl btc::Trait for Test {
    type Event = ();
}

impl Trait for Test {}

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
    let t: Token = Token::new(
        btc::Module::<Test>::SYMBOL.to_vec(),
        b"btc token".to_vec(),
        8,
    );

    r.extend(
        tokenbalances::GenesisConfig::<Test> {
            chainx_precision: 8,
            token_list: vec![(t, vec![])],
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

#[test]
fn test_check_btc_addr() {
    with_externalities(&mut new_test_ext(), || {
        assert_ok!(financialrecords::Module::<Test>::deposit(
            &1,
            &b"btc".to_vec(),
            1000
        ));

        let origin = system::RawOrigin::Signed(1).into();
        assert_err!(
            Module::<Test>::withdraw(
                origin,
                b"btc".to_vec(),
                100,
                b"sdfds".to_vec(),
                b"".to_vec()
            ),
            "verify btc addr err"
        );

        let origin = system::RawOrigin::Signed(1).into();
        assert_ok!(Module::<Test>::withdraw(
            origin,
            b"btc".to_vec(),
            100,
            "mjKE11gjVN4JaC9U8qL6ZB5vuEBgmwik7b".from_base58().unwrap(),
            b"".to_vec()
        ));
    })
}
