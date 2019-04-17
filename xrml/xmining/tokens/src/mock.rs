// Copyright 2018-2019 Chainpool.
//! Test utilities
#![cfg(test)]

use super::*;

use primitives::{
    testing::{ConvertUintAuthorityId, Digest, DigestItem, Header, UintAuthorityId},
    traits::BlakeTwo256,
    BuildStorage, StorageOverlay,
};
use runtime_io::with_externalities;
use substrate_primitives::{Blake2Hasher, H256};
use support::impl_outer_origin;

use xassets::{Asset, Chain, ChainT, Token};

impl_outer_origin! {
    pub enum Origin for Test {}
}

// Workaround for https://github.com/rust-lang/rust/issues/26925 . Remove when sorted.
#[derive(Clone, PartialEq, Eq, Debug)]
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
    type Balance = u64;
    type OnNewAccount = Indices;
    type Event = ();
    type OnAssetChanged = (XTokens);
    type OnAssetRegisterOrRevoke = (XTokens);
}

impl xfee_manager::Trait for Test {
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

impl xsession::Trait for Test {
    type ConvertAccountIdToSessionKey = ConvertUintAuthorityId;
    type OnSessionChange = XStaking;
    type Event = ();
}

impl xbitcoin::Trait for Test {
    type Event = ();
}

impl xrecords::Trait for Test {
    type Event = ();
}

impl xstaking::Trait for Test {
    type Event = ();
    type OnRewardCalculation = XTokens;
    type OnReward = XTokens;
}

impl xspot::Trait for Test {
    type Price = <Self as xassets::Trait>::Balance;
    type Event = ();
}

impl xsdot::Trait for Test {
    type Event = ();
}

impl Trait for Test {
    type Event = ();
    type DetermineTokenJackpotAccountId = DummyDetermineTokenJackpotAccountId;
}

pub struct DummyDetermineTokenJackpotAccountId;
impl TokenJackpotAccountIdFor<u64, u64> for DummyDetermineTokenJackpotAccountId {
    fn accountid_for(_token: &Token) -> u64 {
        10
    }
}

pub fn new_test_ext() -> runtime_io::TestExternalities<Blake2Hasher> {
    let mut t = system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap()
        .0;
    let pcx_precision = 8;
    let apply_prec = |x| x * 10_u64.pow(pcx_precision as u32);

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

    t.extend(
        consensus::GenesisConfig::<Test> {
            code: vec![],
            authorities: vec![],
        }
        .build_storage()
        .unwrap()
        .0,
    );
    t.extend(
        timestamp::GenesisConfig::<Test> { minimum_period: 3 }
            .build_storage()
            .unwrap()
            .0,
    );
    t.extend(
        xsession::GenesisConfig::<Test> {
            session_length: 1,
            validators: vec![
                (1, 125_000_000),
                (2, 125_000_000),
                (3, 125_000_000),
                (4, 125_000_000),
            ],
            keys: vec![
                (1, UintAuthorityId(1)),
                (2, UintAuthorityId(2)),
                (3, UintAuthorityId(3)),
                (4, UintAuthorityId(4)),
            ],
        }
        .build_storage()
        .unwrap()
        .0,
    );
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
        xstaking::GenesisConfig::<Test> {
            initial_reward: apply_prec(50),
            current_era: 0,
            validator_count: 8,
            minimum_validator_count: 4,
            bonding_duration: 1,
            intention_bonding_duration: 10,
            sessions_per_era: 1,
            sessions_per_epoch: 10,
            minimum_penalty: 10_000_000, // 0.1 PCX by default
        }
        .build_storage()
        .unwrap()
        .0,
    );

    t.extend(
        GenesisConfig::<Test> {
            token_discount: vec![(sdot_asset.token(), 50)],
            _genesis_phantom_data: Default::default(),
        }
        .build_storage()
        .unwrap()
        .0,
    );

    let mut init: runtime_io::TestExternalities<Blake2Hasher> = t.into();

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

        let intentions = vec![
            (1, 125_000_000, b"".to_vec(), b"".to_vec()),
            (2, 125_000_000, b"".to_vec(), b"".to_vec()),
            (3, 125_000_000, b"".to_vec(), b"".to_vec()),
            (4, 125_000_000, b"".to_vec(), b"".to_vec()),
        ];

        // xstaking
        let pcx = XAssets::TOKEN.to_vec();
        for (intention, value, name, url) in intentions.clone().into_iter() {
            XStaking::bootstrap_register(&intention, name).unwrap();

            XAssets::pcx_issue(&intention, value).unwrap();

            XAssets::move_balance(
                &pcx,
                &intention,
                xassets::AssetType::Free,
                &intention,
                xassets::AssetType::ReservedStaking,
                value,
            )
            .unwrap();

            XStaking::bootstrap_refresh(&intention, Some(url), Some(true), None, None);
            XStaking::bootstrap_update_vote_weight(&intention, &intention, value, true);
        }

        xaccounts::TeamAddress::<Test>::put(666);
        xaccounts::CouncilAddress::<Test>::put(888);
    });
    let init: StorageOverlay = init.into();
    runtime_io::TestExternalities::new(init)
}

pub type Indices = indices::Module<Test>;
pub type System = system::Module<Test>;
pub type XSession = xsession::Module<Test>;
pub type XAssets = xassets::Module<Test>;
pub type XStaking = xstaking::Module<Test>;
pub type XBitcoin = xbitcoin::Module<Test>;
pub type XSdot = xsdot::Module<Test>;
pub type XTokens = Module<Test>;
