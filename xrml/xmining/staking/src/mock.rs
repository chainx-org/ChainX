// Copyright 2018 Chainpool.
//! Test utilities
#![cfg(test)]

use super::*;
use crate::{GenesisConfig, Module, Trait};
use primitives::testing::{ConvertUintAuthorityId, Digest, DigestItem, Header, UintAuthorityId};
use primitives::StorageOverlay;
use primitives::{traits::BlakeTwo256, BuildStorage};
use runtime_io;
use runtime_io::with_externalities;
use runtime_support::impl_outer_origin;
use substrate_primitives::{Blake2Hasher, H256};
use xaccounts::IntentionJackpotAccountIdFor;
use xassets::assetdef::{Asset, Chain, ChainT, Token};
use xsystem::{Validator, ValidatorList};
use {balances, consensus, indices, session, system, timestamp, xassets};

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
    type IsDeadAccount = Balances;
    type ResolveHint = indices::SimpleResolveHint<Self::AccountId, Self::AccountIndex>;
    type Event = ();
}
impl balances::Trait for Test {
    type Balance = u64;
    type OnNewAccount = Indices;
    type OnFreeBalanceZero = ();
    type Event = ();
}
impl xaccounts::Trait for Test {
    type Event = ();
    type DetermineIntentionJackpotAccountId = DummyDetermineIntentionJackpotAccountId;
}
pub struct DummyDetermineIntentionJackpotAccountId;
impl IntentionJackpotAccountIdFor<u64> for DummyDetermineIntentionJackpotAccountId {
    fn accountid_for(origin: &u64) -> u64 {
        origin + 100
    }
}
impl xassets::Trait for Test {
    type Event = ();
    type OnAssetChanged = ();
    type OnAssetRegisterOrRevoke = ();
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
impl timestamp::Trait for Test {
    type Moment = u64;
    type OnTimestampSet = ();
}
impl session::Trait for Test {
    type ConvertAccountIdToSessionKey = ConvertUintAuthorityId;
    type OnSessionChange = Staking;
    type Event = ();
}
impl xbitcoin::Trait for Test {
    type Event = ();
}
impl xrecords::Trait for Test {
    type Event = ();
}
impl Trait for Test {
    type OnRewardCalculation = ();
    type OnReward = ();
    type Event = ();
}

pub fn new_test_ext() -> runtime_io::TestExternalities<Blake2Hasher> {
    let mut t = system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap()
        .0;
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
        timestamp::GenesisConfig::<Test> { period: 3 }
            .build_storage()
            .unwrap()
            .0,
    );
    t.extend(
        session::GenesisConfig::<Test> {
            session_length: 1,
            validators: vec![(10, 10), (20, 20), (30, 30), (40, 40)],
        }
        .build_storage()
        .unwrap()
        .0,
    );
    t.extend(
        balances::GenesisConfig::<Test> {
            balances: vec![(1, 10), (2, 20), (3, 30), (4, 40)],
            existential_deposit: 0,
            transfer_fee: 0,
            creation_fee: 0,
            vesting: vec![],
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

    let pcx_precision = 8;
    let apply_prec = |x| x * 10_u64.pow(pcx_precision as u32);
    let full_endowed = vec![
        (
            10u64,
            apply_prec(10),
            b"10".to_vec(),
            b"10.com".to_vec(),
            b"03f72c448a0e59f48d4adef86cba7b278214cece8e56ef32ba1d179e0a8129bdba".to_vec(), // hot_entity
            b"02a79800dfed17ad4c78c52797aa3449925692bc8c83de469421080f42d27790ee".to_vec(),
        ), // cold_entity
        (
            20u64,
            apply_prec(20),
            b"20".to_vec(),
            b"Bob.com".to_vec(),
            b"0306117a360e5dbe10e1938a047949c25a86c0b0e08a0a7c1e611b97de6b2917dd".to_vec(),
            b"03ece1a20b5468b12fd7beda3e62ef6b2f6ad9774489e9aff1c8bc684d87d70780".to_vec(),
        ),
        (
            30u64,
            apply_prec(30),
            b"30".to_vec(),
            b"30".to_vec(),
            b"0311252930af8ba766b9c7a6580d8dc4bbf9b0befd17a8ef7fabac275bba77ae40".to_vec(),
            b"02e34d10113f2dd162e8d8614a4afbb8e2eb14eddf4036042b35d12cf5529056a2".to_vec(),
        ),
        (
            40u64,
            apply_prec(30),
            b"40".to_vec(),
            b"40".to_vec(),
            b"0227e54b65612152485a812b8856e92f41f64788858466cc4d8df674939a5538c3".to_vec(),
            b"020699bf931859cafdacd8ac4d3e055eae7551427487e281e3efba618bdd395f2f".to_vec(),
        ),
    ];
    t.extend(
        GenesisConfig::<Test> {
            initial_reward: apply_prec(50),
            current_era: 0,
            validator_count: 8,
            minimum_validator_count: 4,
            trustee_count: 8,
            minimum_trustee_count: 4,
            bonding_duration: 1,
            intention_bonding_duration: 10,
            sessions_per_era: 1,
            sessions_per_epoch: 10,
            minimum_penalty: 10_000_000, // 0.1 PCX by default
            validator_stake_threshold: 1,
        }
        .build_storage()
        .unwrap()
        .0,
    );
    let mut init: runtime_io::TestExternalities<Blake2Hasher> = t.into();
    let pcx_token_name = b"PolkadotChainX".to_vec();
    let pcx_desc = b"PCX onchain token".to_vec();
    with_externalities(&mut init, || {
        // xassets
        let chainx: Token = <XAssets as ChainT>::TOKEN.to_vec();

        let pcx = Asset::new(
            chainx.clone(),
            pcx_token_name,
            Chain::ChainX,
            pcx_precision,
            pcx_desc,
        )
        .unwrap();
        XAssets::bootstrap_register_asset(pcx, true, false, Zero::zero()).unwrap();
        XAssets::bootstrap_set_asset_balance(&6, &chainx, xassets::AssetType::Free, 30);
    });
    let init: StorageOverlay = init.into();
    runtime_io::TestExternalities::new(init)
}

pub type Indices = indices::Module<Test>;
pub type System = system::Module<Test>;
pub type Session = session::Module<Test>;
pub type XAssets = xassets::Module<Test>;
pub type XAccounts = xaccounts::Module<Test>;
pub type Balances = balances::Module<Test>;
pub type Staking = Module<Test>;
