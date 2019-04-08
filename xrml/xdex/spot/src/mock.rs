// Copyright 2018-2019 Chainpool.

#![cfg(test)]

use super::*;

// Substrate
use primitives::testing::{Digest, DigestItem, Header, UintAuthorityId};
use primitives::traits::BlakeTwo256;
use primitives::BuildStorage;
use primitives::StorageOverlay;
use runtime_io::with_externalities;
use substrate_primitives::{Blake2Hasher, H256};
use support::impl_outer_origin;

// ChainX
use xassets::{Asset, Chain, ChainT, Token};

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
    type IsDeadAccount = XAssets;
    type ResolveHint = indices::SimpleResolveHint<Self::AccountId, Self::AccountIndex>;
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

pub struct DummyDetermineIntentionJackpotAccountId;
impl xaccounts::IntentionJackpotAccountIdFor<u64> for DummyDetermineIntentionJackpotAccountId {
    fn accountid_for(origin: &u64) -> u64 {
        origin + 100
    }
}

impl xassets::Trait for Test {
    type Event = ();
    type Balance = u64;
    type OnNewAccount = Indices;
    type OnAssetChanged = ();
    type OnAssetRegisterOrRevoke = ();
}

impl xrecords::Trait for Test {
    type Event = ();
}

impl xsystem::Trait for Test {
    type ValidatorList = DummyDetermineValidatorList;
    type Validator = DummyDetermineValidator;
}

pub struct DummyDetermineValidatorList;
impl xsystem::ValidatorList<u64> for DummyDetermineValidatorList {
    fn validator_list() -> Vec<u64> {
        vec![]
    }
}
pub struct DummyDetermineValidator;
impl xsystem::Validator<u64> for DummyDetermineValidator {
    fn get_validator_by_name(_name: &[u8]) -> Option<u64> {
        Some(0)
    }
    fn get_validator_name(_accountid: &u64) -> Option<Vec<u8>> {
        None
    }
}

impl xfee_manager::Trait for Test {
    type Event = ();
}

impl Trait for Test {
    type Price = u64;
    type Event = ();
}

pub type Indices = indices::Module<Test>;
pub type XAssets = xassets::Module<Test>;
pub type XBitcoin = xbitcoin::Module<Test>;
pub type XSpot = Module<Test>;

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
        <XBitcoin as ChainT>::TOKEN.to_vec(), // token
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
            XAssets::TOKEN.to_vec(),
            XBitcoin::TOKEN.to_vec(),
            9,
            2,
            100000,
            true,
        ),
        (
            sdot_asset.token(),
            XAssets::TOKEN.to_vec(),
            4,
            2,
            100000,
            true,
        ),
    ];

    with_externalities(&mut init, || {
        // xassets
        let chainx: Token = <XAssets as ChainT>::TOKEN.to_vec();

        let pcx = Asset::new(chainx, pcx.0.clone(), Chain::ChainX, pcx.1, pcx.2.clone()).unwrap();

        XAssets::bootstrap_register_asset(pcx, true, false).unwrap();

        // init for asset_list
        for (asset, is_online, is_psedu_intention, init_list) in asset_list.iter() {
            let token = asset.token();
            XAssets::bootstrap_register_asset(asset.clone(), *is_online, *is_psedu_intention)
                .unwrap();

            for (accountid, value) in init_list {
                let value: u64 = *value;
                XAssets::issue(&token, &accountid, value).unwrap();
            }
        }

        for (base, quote, pip_precision, tick_precision, price, status) in pair_list.iter() {
            let _ = XSpot::add_trading_pair(
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
