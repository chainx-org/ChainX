// Copyright 2018 Chainpool.
//! Test utilities

#![cfg(test)]

use super::JackpotAccountIdFor;
use runtime_io;
use runtime_primitives::testing::{
    ConvertUintAuthorityId, Digest, DigestItem, Header, UintAuthorityId,
};
use runtime_primitives::traits::BlakeTwo256;
use runtime_primitives::BuildStorage;
use runtime_primitives::Perbill;
use substrate_primitives::{Blake2Hasher, H256};
use {balances, consensus, session, system, timestamp, xassets, GenesisConfig, Module, Trait};

impl_outer_origin! {
    pub enum Origin for Test {}
}

// Workaround for https://github.com/rust-lang/rust/issues/26925 . Remove when sorted.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Test;
impl consensus::Trait for Test {
    const NOTE_OFFLINE_POSITION: u32 = 1;
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
impl xaccounts::Trait for Test {
    type Event = ();
}
impl xassets::Trait for Test {
    type Event = ();
    type OnAssetChanged = ();
    type OnAssetRegisterOrRevoke = ();
}
impl xsystem::Trait for Test {
    const XSYSTEM_SET_POSITION: u32 = 3;
}
impl timestamp::Trait for Test {
    const TIMESTAMP_SET_POSITION: u32 = 0;
    type Moment = u64;
    type OnTimestampSet = ();
}
impl session::Trait for Test {
    type ConvertAccountIdToSessionKey = ConvertUintAuthorityId;
    type OnSessionChange = Staking;
    type Event = ();
}
impl Trait for Test {
    type OnRewardMinted = ();
    type OnRewardCalculation = ();
    type OnReward = ();
    type Event = ();
    type DetermineJackpotAccountId = DummyAccountIdFor;
}

pub struct DummyAccountIdFor;
impl JackpotAccountIdFor<u64> for DummyAccountIdFor {
    fn accountid_for(origin: &u64) -> u64 {
        origin + 100
    }
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
            validators: vec![10, 20],
        }
        .build_storage()
        .unwrap()
        .0,
    );
    t.extend(
        balances::GenesisConfig::<Test> {
            balances: vec![(1, 10), (2, 20), (3, 30), (4, 40), (10, 100), (20, 100)],
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
    t.extend(
        xaccounts::GenesisConfig::<Test> {
            shares_per_cert: 50,
            activation_per_share: 100_000_000,
            maximum_cert_count: 178,
            total_issued: 2,
            cert_owner: 1,
        }
        .build_storage()
        .unwrap()
        .0,
    );
    t.extend(
        xassets::GenesisConfig::<Test> {
            pcx: (3, b"PCX".to_vec()),
            memo_len: 128,
            asset_list: vec![],
        }
        .build_storage()
        .unwrap()
        .0,
    );
    t.extend(
        GenesisConfig::<Test> {
            intentions: vec![10, 20],
            current_era: 0,
            current_session_reward: 100,
            validator_count: 2,
            bonding_duration: 1,
            minimum_validator_count: 0,
            sessions_per_era: 1,
            offline_slash: Perbill::zero(),
            current_offline_slash: 20,
            offline_slash_grace: 0,
        }
        .build_storage()
        .unwrap()
        .0,
    );
    runtime_io::TestExternalities::new(t)
}

pub type System = system::Module<Test>;
pub type Session = session::Module<Test>;
pub type XAssets = xassets::Module<Test>;
pub type XAccounts = xaccounts::Module<Test>;
pub type Staking = Module<Test>;
