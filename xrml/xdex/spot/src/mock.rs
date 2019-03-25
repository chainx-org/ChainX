// Copyright 2018 Chainpool.
use substrate_primitives::{Blake2Hasher, H256};

use super::*;
use primitives::testing::{Digest, DigestItem, Header, UintAuthorityId};
use primitives::traits::BlakeTwo256;
use primitives::BuildStorage;
use primitives::StorageOverlay;
use runtime_io;
use runtime_io::with_externalities;
use runtime_support::impl_outer_origin;
use xaccounts::IntentionJackpotAccountIdFor;
use xassets::{self, Asset, AssetType, Chain, ChainT, Token};
use xsystem::{Validator, ValidatorList};

impl_outer_origin! {
    pub enum Origin for Test {}
}

#[derive(Clone, Eq, PartialEq)]
pub struct Test;

impl consensus::Trait for Test {
    type Log = DigestItem;
    type SessionKey = UintAuthorityId;
    type InherentOfflineReport = ();
}

impl system::Trait for Test {
    type Origin = Origin;
    type Index = u64;
    type BlockNumber = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type Digest = Digest;
    type AccountId = u64;
    type Lookup = Indices;
    type Header = Header;
    type Event = ();
    type Log = DigestItem;
}

impl indices::Trait for Test {
    type AccountIndex = u32;
    type IsDeadAccount = Balances;
    type ResolveHint = indices::SimpleResolveHint<Self::AccountId, Self::AccountIndex>;
    type Event = ();
}

impl balances::Trait for Test {
    type Balance = u64;
    type OnFreeBalanceZero = ();
    type OnNewAccount = Indices;
    type Event = ();
}

impl timestamp::Trait for Test {
    type Moment = u64;
    type OnTimestampSet = ();
}

impl xbitcoin::Trait for Test {
    type Event = ();
}

impl xaccounts::Trait for Test {
    type Event = ();
    type DetermineIntentionJackpotAccountId = DummyDetermineIntentionJackpotAccountId;
}

impl xassets::Trait for Test {
    type Event = ();
    type OnAssetChanged = ();
    type OnAssetRegisterOrRevoke = ();
}

impl xrecords::Trait for Test {
    type Event = ();
}

pub struct DummyDetermineValidatorList;
impl ValidatorList<u64> for DummyDetermineValidatorList {
    fn validator_list() -> Vec<u64> {
        vec![]
    }
}
pub struct DummyDetermineValidator;
impl Validator<u64> for DummyDetermineValidator {
    fn get_validator_by_name(_name: &[u8]) -> Option<u64> {
        Some(0)
    }
}

impl xsystem::Trait for Test {
    type ValidatorList = DummyDetermineValidatorList;
    type Validator = DummyDetermineValidator;
}

impl fee_manager::Trait for Test {}

pub struct DummyDetermineIntentionJackpotAccountId;
impl IntentionJackpotAccountIdFor<u64> for DummyDetermineIntentionJackpotAccountId {
    fn accountid_for(origin: &u64) -> u64 {
        origin + 100
    }
}
impl Trait for Test {
    type Price = u64;
    type Event = ();
}
// This function basically just builds a genesis storage key/value store according to
// our desired mockup.
pub fn new_test_ext() -> runtime_io::TestExternalities<Blake2Hasher> {
    let mut t = system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap()
        .0;

    t.extend(
        xassets::GenesisConfig::<Test> {
            memo_len: 128,
            _genesis_phantom_data: Default::default(),
        }
        .build_storage()
        .unwrap()
        .0,
    );

    t.extend(
        GenesisConfig::<Test> {
            price_volatility: 10,
            _genesis_phantom_data: Default::default(),
        }
        .build_storage()
        .unwrap()
        .0,
    );

    let mut init: runtime_io::TestExternalities<Blake2Hasher> = t.into();

    let pcx = (
        b"Polkadot ChainX".to_vec(),
        8_u16,
        b"ChainX's crypto currency in Polkadot ecology".to_vec(),
    );

    let btc_asset = Asset::new(
        <xbitcoin::Module<Test> as ChainT>::TOKEN.to_vec(), // token
        b"X-BTC".to_vec(),
        Chain::Bitcoin,
        8, // bitcoin precision
        b"ChainX's Cross-chain Bitcoin".to_vec(),
    )
    .unwrap();

    let sdot_asset = Asset::new(
        b"SDOT".to_vec(), // token
        b"Shadow DOT".to_vec(),
        Chain::Ethereum,
        3, //  precision
        b"ChainX's Shadow Polkadot from Ethereum".to_vec(),
    )
    .unwrap();

    let asset_list = vec![
        (btc_asset.clone(), true, true, vec![]),
        (sdot_asset.clone(), true, true, vec![]),
    ];

    let pair_list = vec![
        (
            xassets::Module::<Test>::TOKEN.to_vec(),
            xbitcoin::Module::<Test>::TOKEN.to_vec(),
            9,
            2,
            100000,
            true,
        ),
        (
            sdot_asset.token(),
            xassets::Module::<Test>::TOKEN.to_vec(),
            4,
            2,
            100000,
            true,
        ),
    ];

    with_externalities(&mut init, || {
        // xassets
        let chainx: Token = <xassets::Module<Test> as ChainT>::TOKEN.to_vec();

        let pcx = Asset::new(chainx, pcx.0.clone(), Chain::ChainX, pcx.1, pcx.2.clone()).unwrap();

        xassets::Module::<Test>::bootstrap_register_asset(pcx, true, false, Zero::zero()).unwrap();

        // init for asset_list
        for (asset, is_online, is_psedu_intention, init_list) in asset_list.iter() {
            let token = asset.token();
            xassets::Module::<Test>::bootstrap_register_asset(
                asset.clone(),
                *is_online,
                *is_psedu_intention,
                Zero::zero(),
            )
            .unwrap();

            for (accountid, value) in init_list {
                let value: u64 = *value;
                let total_free_token =
                    xassets::Module::<Test>::total_asset_balance(&token, AssetType::Free);
                let free_token = xassets::Module::<Test>::free_balance(&accountid, &token);
                xassets::Module::<Test>::bootstrap_set_total_asset_balance(
                    &token,
                    AssetType::Free,
                    total_free_token + value,
                );
                // not create account
                xassets::Module::<Test>::bootstrap_set_asset_balance(
                    &accountid,
                    &token,
                    AssetType::Free,
                    free_token + value,
                );
            }
        }

        for (base, quote, pip_precision, tick_precision, price, status) in pair_list.iter() {
            let _ = Spot::add_trading_pair(
                CurrencyPair::new(base.clone(), quote.clone()),
                *pip_precision,
                *tick_precision,
                *price,
                *status,
            );
        }
    });

    let init: StorageOverlay = init.into();
    runtime_io::TestExternalities::new(init)
}

pub type Indices = indices::Module<Test>;
pub type Assets = xassets::Module<Test>;
pub type Balances = balances::Module<Test>;
pub type Spot = Module<Test>;
