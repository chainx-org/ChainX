// Copyright 2018-2019 Chainpool.

#![cfg(test)]

use super::*;

// Substrate
use parity_codec::{Decode, Encode};
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

impl support::dispatch::Dispatchable for DummyTrusteeCall {
    type Origin = Origin;
    type Trait = DummyTrusteeCall;
    fn dispatch(self, _origin: Origin) -> support::dispatch::Result {
        Ok(())
    }
}

impl xaccounts::Trait for Test {
    type DetermineIntentionJackpotAccountId = DummyDetermineIntentionJackpotAccountId;
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

impl xbridge_common::Trait for Test {
    type Event = ();
}

impl xassets::Trait for Test {
    type Event = ();
    type Balance = u64;
    type OnNewAccount = Indices;
    type OnAssetChanged = ();
    type OnAssetRegisterOrRevoke = ();
    type DetermineTokenJackpotAccountId = ();
}

impl xrecords::Trait for Test {
    type Event = ();
}

impl xsystem::Trait for Test {
    type ValidatorList = DummyDetermineValidatorList;
    type Validator = DummyDetermineValidator;
}

impl xsdot::Trait for Test {
    type AccountExtractor = DummyExtractor;
    type CrossChainProvider = XBridgeFeatures;
    type Event = ();
}

impl xbitcoin::lockup::Trait for Test {
    type Event = ();
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

impl xbridge_features::Trait for Test {
    type TrusteeMultiSig = DummyMultiSigIdFor;
    type Event = ();
}

pub struct DummyBitcoinTrusteeMultiSig;
impl xbridge_common::traits::TrusteeMultiSig<u64> for DummyBitcoinTrusteeMultiSig {
    fn multisig_for_trustees() -> u64 {
        777
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

pub struct DummyGenesisMultiSig;
impl xmultisig::GenesisMultiSig<u64> for DummyGenesisMultiSig {
    fn gen_genesis_multisig() -> (u64, u64) {
        (666, 888)
    }
}

pub struct DummyMultiSigIdFor;
impl xbridge_features::TrusteeMultiSigFor<u64> for DummyMultiSigIdFor {
    fn multi_sig_addr_for_trustees(_chain: xassets::Chain, _who: &Vec<u64>) -> u64 {
        1
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

pub type XBridgeFeatures = xbridge_features::Module<Test>;
