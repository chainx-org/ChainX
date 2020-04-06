// Copyright 2018-2019 Chainpool.

#![cfg(test)]

use super::*;

use std::str::FromStr;
// Substrate
use primitives::testing::{ConvertUintAuthorityId, Digest, DigestItem, Header, UintAuthorityId};
use primitives::traits::{BlakeTwo256, IdentityLookup};
use primitives::{BuildStorage, StorageOverlay};
use runtime_io::with_externalities;
use substrate_primitives::{Blake2Hasher, H256};
use support::impl_outer_origin;

// ChainX
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

impl xassets::Trait for Test {
    type Balance = u64;
    type OnNewAccount = ();
    type Event = ();
    type OnAssetChanged = ();
    type OnAssetRegisterOrRevoke = ();
    type DetermineTokenJackpotAccountId = ();
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

impl xsystem::Trait for Test {
    type ValidatorList = MockValidatorList;
    type Validator = MockValidator;
}

pub struct MockValidatorList;

impl xsystem::ValidatorList<u64> for MockValidatorList {
    fn validator_list() -> Vec<u64> {
        vec![]
    }
}

pub struct MockValidator;

impl xsystem::Validator<u64> for MockValidator {
    fn get_validator_by_name(_name: &[u8]) -> Option<u64> {
        Some(0)
    }
    fn get_validator_name(_: &u64) -> Option<Vec<u8>> {
        None
    }
}

impl xaccounts::Trait for Test {
    type DetermineIntentionJackpotAccountId = MockDeterminator;
}

pub struct MockDeterminator;

impl xaccounts::IntentionJackpotAccountIdFor<u64> for MockDeterminator {
    fn accountid_for_unsafe(_: &u64) -> u64 {
        1000
    }
    fn accountid_for_safe(_: &u64) -> Option<u64> {
        Some(1000)
    }
}

impl xrecords::Trait for Test {
    type Event = ();
}

impl xfee_manager::Trait for Test {
    type Event = ();
}

impl xbitcoin::lockup::Trait for Test {
    type Event = ();
}

impl xbridge_common::Trait for Test {
    type Event = ();
}

impl xbitcoin::Trait for Test {
    type XBitcoinLockup = Self;
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
pub struct DummyBitcoinTrusteeMultiSig;
impl xbridge_common::traits::TrusteeMultiSig<u64> for DummyBitcoinTrusteeMultiSig {
    fn multisig_for_trustees() -> u64 {
        777
    }
}

pub struct DummyMultiSigIdFor;
impl xbridge_features::TrusteeMultiSigFor<u64> for DummyMultiSigIdFor {
    fn multi_sig_addr_for_trustees(_chain: xassets::Chain, _who: &Vec<u64>) -> u64 {
        1
    }
}

impl xbridge_features::Trait for Test {
    type TrusteeMultiSig = DummyMultiSigIdFor;
    type Event = ();
}

impl xmultisig::Trait for Test {
    type MultiSig = DummyMultiSig;
    type GenesisMultiSig = DummyGenesisMultiSig;
    type Proposal = DummyCall;
    type TrusteeCall = TrusteeCall;
    type Event = ();
}

pub struct DummyGenesisMultiSig;
impl xmultisig::GenesisMultiSig<u64> for DummyGenesisMultiSig {
    fn gen_genesis_multisig() -> (u64, u64) {
        (666, 888)
    }
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
pub struct DummyCall;
pub struct TrusteeCall(DummyCall);
impl From<DummyCall> for TrusteeCall {
    fn from(call: DummyCall) -> Self {
        TrusteeCall(call)
    }
}
impl xmultisig::LimitedCall<u64> for TrusteeCall {
    fn allow(&self) -> bool {
        true
    }

    fn exec(&self, _execiser: &u64) -> Result {
        Ok(())
    }
}

impl support::dispatch::Dispatchable for DummyCall {
    type Origin = Origin;
    type Trait = DummyCall;
    fn dispatch(self, _origin: Origin) -> support::dispatch::Result {
        Ok(())
    }
}

impl xstaking::Trait for Test {
    type Event = ();
    type OnDistributeAirdropAsset = ();
    type OnDistributeCrossChainAsset = ();
    type OnReward = ();
}

impl xsession::Trait for Test {
    type ConvertAccountIdToSessionKey = ConvertUintAuthorityId;
    type OnSessionChange = XStaking;
    type Event = ();
}

impl Trait for Test {}

pub type XAssets = xassets::Module<Test>;
pub type XRecords = xrecords::Module<Test>;
pub type XBitCoin = xbitcoin::Module<Test>;
pub type XProcess = Module<Test>;
pub type XBridgeFeatures = xbridge_features::Module<Test>;
pub type XStaking = xstaking::Module<Test>;

// This function basically just builds a genesis storage key/value store according to
// our desired mockup.
pub fn new_test_ext() -> runtime_io::TestExternalities<Blake2Hasher> {
    let mut r = system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap()
        .0;
    // balance
    r.extend(
        xassets::GenesisConfig::<Test> {
            memo_len: 128,
            _genesis_phantom_data: Default::default(),
        }
        .build_storage()
        .unwrap()
        .0,
    );
    // token balance
    let _btc_asset = Asset::new(
        b"BTC".to_vec(),     // token
        b"Bitcoin".to_vec(), // token
        Chain::Bitcoin,
        8, // bitcoin precision
        b"BTC chainx".to_vec(),
    )
    .unwrap();

    r.extend(
        GenesisConfig::<Test> {
            token_black_list: vec![b"SDOT".to_vec()],
            _genesis_phantom_data: Default::default(),
        }
        .build_storage()
        .unwrap()
        .0,
    );

    r.extend(
        xassets::GenesisConfig::<Test> {
            memo_len: 128,
            _genesis_phantom_data: Default::default(),
        }
        .build_storage()
        .unwrap()
        .0,
    );

    let mut init: runtime_io::TestExternalities<Blake2Hasher> = r.into();
    with_externalities(&mut init, || {
        let chainx: Token = <XAssets as ChainT>::TOKEN.to_vec();

        let pcx = Asset::new(
            chainx.clone(),
            b"PolkadotChainX".to_vec(),
            Chain::ChainX,
            8,
            b"PCX onchain token".to_vec(),
        )
        .unwrap();
        // token balance
        let sdot = Asset::new(
            b"SDOT".to_vec(), // token
            b"SDOT".to_vec(), // token
            Chain::Ethereum,
            3,
            b"SDOT chainx".to_vec(),
        )
        .unwrap();
        let btc = Asset::new(
            b"BTC".to_vec(),
            b"X-BTC".to_vec(),
            Chain::Bitcoin,
            8, // bitcoin precision
            b"ChainX's Cross-chain Bitcoin".to_vec(),
        )
        .unwrap();
        XAssets::bootstrap_register_asset(pcx, true, false).unwrap();
        XAssets::bootstrap_register_asset(btc, true, true).unwrap();
        XAssets::bootstrap_register_asset(sdot, true, true).unwrap();

        XBridgeFeatures::set_trustee_info_config(
            Chain::Bitcoin,
            xbridge_common::types::TrusteeInfoConfig {
                min_trustee_count: 3,
                max_trustee_count: 15,
            },
        )
        .unwrap();

        XAssets::pcx_issue(&1, 10).unwrap();
        XAssets::pcx_issue(&2, 20).unwrap();
        XAssets::pcx_issue(&3, 30).unwrap();
        XAssets::pcx_issue(&4, 40).unwrap();
        XAssets::pcx_issue(&5, 50).unwrap();

        let intentions = vec![
            (1, b"name10".to_vec()),
            (2, b"name20".to_vec()),
            (3, b"name30".to_vec()),
            (4, b"name40".to_vec()),
        ];
        for (intention, name) in intentions.into_iter() {
            XStaking::bootstrap_register(&intention, name).unwrap();
        }

        let trustee_intentions = vec![
            (
                1,
                "0384106cbc714c3a7f9a1a6cc763525d1f65e4993721f5023fc0954a185aa2fd1d",
                "022f0fe2f0801f5dc95d93254e5e2226e919c4759f24f38e3998f695e35c967984",
            ),
            (
                2,
                "02161422b1b2da8d3f9986b0df694a9008d535fb364755858b14cb94ea41a339e7",
                "02183ffa596f67f0445ab945933c2229c3f4f5dcb346cc8d5dfaae30d5d64e8ac4",
            ),
            (
                3,
                "026210f0c305bc8131ba929a55b3a4de504afed8cbf985c05d487e40675ba40383",
                "02b9127e1c4f25e6b15e488006c1222dc55327b38100de7b890cd50b5e2cdb9804",
            ),
            (
                4,
                "026210f0c305bc5551ba929a55b3a4de504afed8cbf985c05d487e40675ba40383",
                "02b9127e1c4f25e6b15e666006c1222dc55327b38100de7b890cd50b5e2cdb9804",
            ),
        ];
        let mut trustees = Vec::new();
        for (i, hot_entity, cold_entity) in trustee_intentions.into_iter() {
            trustees.push(i.clone());
            XBridgeFeatures::setup_bitcoin_trustee_impl(
                i,
                b"ChainX init".to_vec(),
                xbridge_features::H264::from_str(hot_entity).unwrap(),
                xbridge_features::H264::from_str(cold_entity).unwrap(),
            )
            .unwrap();
        }
        XBridgeFeatures::deploy_trustee_in_genesis(vec![(Chain::Bitcoin, trustees)]).unwrap();
    });
    let init: StorageOverlay = init.into();
    runtime_io::TestExternalities::new(init)
}
