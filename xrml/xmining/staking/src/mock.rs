// Copyright 2018-2019 Chainpool.
//! Test utilities
#![cfg(test)]

use super::*;

// Substrate
use parity_codec::{Decode, Encode};
use primitives::{
    testing::{ConvertUintAuthorityId, Digest, DigestItem, Header, UintAuthorityId},
    traits::BlakeTwo256,
    BuildStorage, StorageOverlay,
};
use runtime_io::with_externalities;
use substrate_primitives::{Blake2Hasher, H256};
use support::impl_outer_origin;

// ChainX
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
    type DetermineIntentionJackpotAccountId = DummyDetermineIntentionJackpotAccountId;
}

impl xbridge_features::Trait for Test {
    type TrusteeMultiSig = DummyMultiSigIdFor;
    type Event = ();
}

impl xmultisig::Trait for Test {
    type MultiSig = DummyMultiSig;
    type GenesisMultiSig = DummyGenesisMultiSig;
    type Proposal = DummyTrusteeCall;
    type Event = ();
}

pub struct DummyMultiSig;
impl xmultisig::MultiSigFor<u64, H256> for DummyMultiSig {
    fn multi_sig_addr_for(who: &u64) -> u64 {
        who + 2
    }

    fn multi_sig_id_for(_who: &u64, _addr: &u64, _data: &[u8]) -> H256 {
        H256::default()
    }
}

#[derive(Clone, Eq, PartialEq, Encode, Decode, Debug)]
pub struct DummyTrusteeCall;
impl xmultisig::TrusteeCall<u64> for DummyTrusteeCall {
    fn allow(&self) -> bool {
        true
    }

    fn exec(&self, _execiser: &u64) -> Result {
        Ok(())
    }
}

impl support::dispatch::Dispatchable for DummyTrusteeCall {
    type Origin = Origin;
    type Trait = DummyTrusteeCall;
    fn dispatch(self, _origin: Origin) -> support::dispatch::Result {
        Ok(())
    }
}

pub struct DummyMultiSigIdFor;
impl xbridge_features::TrusteeMultiSigFor<u64> for DummyMultiSigIdFor {
    fn multi_sig_addr_for_trustees(_chain: xassets::Chain, _who: &Vec<u64>) -> u64 {
        1
    }
}

pub struct DummyBitcoinTrusteeMultiSig;
impl xbridge_common::traits::TrusteeMultiSig<u64> for DummyBitcoinTrusteeMultiSig {
    fn multisig_for_trustees() -> u64 {
        777
    }
}

pub struct DummyGenesisMultiSig;
impl xmultisig::GenesisMultiSig<u64> for DummyGenesisMultiSig {
    fn gen_genesis_multisig() -> (u64, u64) {
        (666, 888)
    }
}

pub struct DummyDetermineIntentionJackpotAccountId;
impl xaccounts::IntentionJackpotAccountIdFor<u64> for DummyDetermineIntentionJackpotAccountId {
    fn accountid_for_unsafe(origin: &u64) -> u64 {
        origin + 100
    }
    fn accountid_for_safe(origin: &u64) -> Option<u64> {
        Some(origin + 100)
    }
}

impl xassets::Trait for Test {
    type Balance = u64;
    type OnNewAccount = Indices;
    type Event = ();
    type OnAssetChanged = ();
    type OnAssetRegisterOrRevoke = ();
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
    type AccountExtractor = DummyExtractor;
    type TrusteeSessionProvider = XBridgeFeatures;
    type TrusteeMultiSigProvider = DummyBitcoinTrusteeMultiSig;
    type CrossChainProvider = XBridgeFeatures;
    type Event = ();
}

pub struct DummyExtractor;
impl xbridge_common::traits::Extractable<u64> for DummyExtractor {
    fn account_info(_data: &[u8], _: u8) -> Option<(u64, Option<Vec<u8>>)> {
        Some((999, None))
    }
}

impl xrecords::Trait for Test {
    type Event = ();
}

impl Trait for Test {
    type Event = ();
    type OnRewardCalculation = ();
    type OnReward = ();
}

pub fn new_test_ext() -> runtime_io::TestExternalities<Blake2Hasher> {
    let mut t = system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap()
        .0;
    let pcx_precision = 8;
    let apply_prec = |x| x * 10_u64.pow(pcx_precision as u32);
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
            validators: vec![(10, 10), (20, 20), (30, 30), (40, 40)],
            keys: vec![
                (10, UintAuthorityId(10)),
                (20, UintAuthorityId(20)),
                (30, UintAuthorityId(30)),
                (40, UintAuthorityId(40)),
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
        GenesisConfig::<Test> {
            initial_reward: apply_prec(50),
            current_era: 0,
            validator_count: 8,
            minimum_validator_count: 4,
            bonding_duration: 1,
            intention_bonding_duration: 10,
            sessions_per_era: 1,
            sessions_per_epoch: 10,
            minimum_penalty: 10_000_000, // 0.1 PCX by default
            missed_blocks_severity: 3,
            maximum_intention_count: 1000,
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
        XAssets::bootstrap_register_asset(pcx, true, false).unwrap();
        XAssets::pcx_issue(&1, 10).unwrap();
        XAssets::pcx_issue(&2, 20).unwrap();
        XAssets::pcx_issue(&3, 30).unwrap();
        XAssets::pcx_issue(&4, 40).unwrap();
        XAssets::pcx_issue(&6, 30).unwrap();
    });
    let init: StorageOverlay = init.into();
    runtime_io::TestExternalities::new(init)
}

pub type Indices = indices::Module<Test>;
pub type System = system::Module<Test>;
pub type XSession = xsession::Module<Test>;
pub type XAssets = xassets::Module<Test>;
pub type XAccounts = xaccounts::Module<Test>;
pub type XBridgeFeatures = xbridge_features::Module<Test>;
pub type XStaking = Module<Test>;
