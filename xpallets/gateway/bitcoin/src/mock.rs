// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use std::{cell::RefCell, collections::BTreeMap, time::Duration};

use hex_literal::hex;

use frame_support::{impl_outer_origin, parameter_types, sp_io, traits::UnixTime, weights::Weight};
use frame_system::EnsureSignedBy;
use sp_core::H256;
use sp_keyring::sr25519;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
    AccountId32, Perbill,
};

use chainx_primitives::AssetId;
use xp_assets_registrar::Chain;
pub use xp_protocol::{X_BTC, X_ETH};
use xpallet_assets::AssetRestrictions;
use xpallet_assets_registrar::AssetInfo;
use xpallet_gateway_common::types::TrusteeInfoConfig;

use light_bitcoin::{
    chain::BlockHeader as BtcHeader,
    keys::Network as BtcNetwork,
    primitives::{h256_rev, Compact},
    serialization::{self, Reader},
};

use crate::{
    types::{BtcParams, BtcTxVerifier},
    Config, Error, GenesisConfig, Module,
};

/// The AccountId alias in this test module.
pub(crate) type AccountId = AccountId32;
pub(crate) type BlockNumber = u64;
pub(crate) type Balance = u128;
pub(crate) type Amount = i128;

impl_outer_origin! {
    pub enum Origin for Test where system = frame_system {}
}

#[derive(Clone, Eq, PartialEq)]
pub struct Test;

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const MaximumBlockWeight: Weight = 1024;
    pub const MaximumBlockLength: u32 = 2 * 1024;
    pub const AvailableBlockRatio: Perbill = Perbill::from_percent(75);
    pub const SS58Prefix: u8 = 42;
}

impl frame_system::Config for Test {
    type BaseCallFilter = ();
    type BlockWeights = ();
    type BlockLength = ();
    type Origin = Origin;
    type Call = ();
    type Index = u64;
    type BlockNumber = BlockNumber;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type Event = ();
    type BlockHashCount = BlockHashCount;
    type DbWeight = ();
    type Version = ();
    type PalletInfo = ();
    type AccountData = pallet_balances::AccountData<Balance>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = SS58Prefix;
}

parameter_types! {
    pub const ExistentialDeposit: u64 = 0;
}
impl pallet_balances::Config for Test {
    type MaxLocks = ();
    type Balance = Balance;
    type DustRemoval = ();
    type Event = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
}

// assets
parameter_types! {
    pub const ChainXAssetId: AssetId = 0;
}

impl xpallet_assets_registrar::Config for Test {
    type Event = ();
    type NativeAssetId = ChainXAssetId;
    type RegistrarHandler = ();
    type WeightInfo = ();
}

impl xpallet_assets::Config for Test {
    type Event = ();
    type Currency = Balances;
    type Amount = Amount;
    type TreasuryAccount = ();
    type OnCreatedAccount = frame_system::CallOnCreatedAccount<Test>;
    type OnAssetChanged = ();
    type WeightInfo = ();
}

impl xpallet_gateway_records::Config for Test {
    type Event = ();
    type WeightInfo = ();
}

impl xpallet_gateway_common::Config for Test {
    type Event = ();
    type Validator = ();
    type DetermineMultisigAddress = ();
    type Bitcoin = XGatewayBitcoin;
    type BitcoinTrustee = XGatewayBitcoin;
    type WeightInfo = ();
}

thread_local! {
    pub static NOW: RefCell<Option<Duration>> = RefCell::new(None);
}

pub struct Timestamp;
impl UnixTime for Timestamp {
    fn now() -> Duration {
        NOW.with(|m| {
            m.borrow().unwrap_or_else(|| {
                use std::time::{SystemTime, UNIX_EPOCH};
                let start = SystemTime::now();
                let since_the_epoch = start
                    .duration_since(UNIX_EPOCH)
                    .expect("Time went backwards");
                since_the_epoch
            })
        })
    }
}

impl Config for Test {
    type Event = ();
    type UnixTime = Timestamp;
    type AccountExtractor = xp_gateway_bitcoin::OpReturnExtractor;
    type TrusteeSessionProvider =
        xpallet_gateway_common::trustees::bitcoin::BtcTrusteeSessionManager<Test>;
    type TrusteeOrigin = EnsureSignedBy<
        xpallet_gateway_common::trustees::bitcoin::BtcTrusteeMultisig<Test>,
        AccountId,
    >;
    type ReferralBinding = XGatewayCommon;
    type AddressBinding = XGatewayCommon;
    type WeightInfo = ();
}

pub type System = frame_system::Module<Test>;
pub type Balances = pallet_balances::Module<Test>;
pub type XAssets = xpallet_assets::Module<Test>;
pub type XGatewayRecords = xpallet_gateway_records::Module<Test>;
pub type XGatewayCommon = xpallet_gateway_common::Module<Test>;
pub type XGatewayBitcoin = Module<Test>;
pub type XGatewayBitcoinErr = Error<Test>;

pub(crate) fn btc() -> (AssetId, AssetInfo, AssetRestrictions) {
    (
        X_BTC,
        AssetInfo::new::<Test>(
            b"X-BTC".to_vec(),
            b"X-BTC".to_vec(),
            Chain::Bitcoin,
            8,
            b"ChainX's cross-chain Bitcoin".to_vec(),
        )
        .unwrap(),
        AssetRestrictions::DESTROY_USABLE,
    )
}

pub struct ExtBuilder;
impl Default for ExtBuilder {
    fn default() -> Self {
        Self
    }
}
impl ExtBuilder {
    pub fn build_mock(
        self,
        btc_genesis: (BtcHeader, u32),
        btc_network: BtcNetwork,
    ) -> sp_io::TestExternalities {
        let mut storage = frame_system::GenesisConfig::default()
            .build_storage::<Test>()
            .unwrap();

        let btc_assets = btc();
        let assets = vec![(btc_assets.0, btc_assets.1, btc_assets.2, true, true)];

        let mut init_assets = vec![];
        let mut assets_restrictions = vec![];
        for (a, b, c, d, e) in assets {
            init_assets.push((a, b, d, e));
            assets_restrictions.push((a, c))
        }

        let _ = xpallet_assets_registrar::GenesisConfig {
            assets: init_assets,
        }
        .assimilate_storage::<Test>(&mut storage);

        let _ = xpallet_assets::GenesisConfig::<Test> {
            assets_restrictions,
            endowed: Default::default(),
        }
        .assimilate_storage(&mut storage);

        // let (genesis_info, genesis_hash, network_id) = load_mock_btc_genesis_header_info();
        let genesis_hash = btc_genesis.0.hash();
        let network_id = btc_network;
        let _ = GenesisConfig::<Test> {
            genesis_trustees: vec![],
            genesis_info: btc_genesis,
            genesis_hash,
            network_id,
            params_info: BtcParams::new(
                486604799,            // max_bits
                2 * 60 * 60,          // block_max_future
                2 * 7 * 24 * 60 * 60, // target_timespan_seconds
                10 * 60,              // target_spacing_seconds
                4,                    // retargeting_factor
            ), // retargeting_factor
            verifier: BtcTxVerifier::Recover,
            confirmation_number: 4,
            btc_withdrawal_fee: 500000,
            max_withdrawal_count: 100,
        }
        .assimilate_storage(&mut storage);

        let ext = sp_io::TestExternalities::new(storage);
        ext
    }

    pub fn build(self) -> sp_io::TestExternalities {
        let mut storage = frame_system::GenesisConfig::default()
            .build_storage::<Test>()
            .unwrap();

        let btc_assets = btc();
        let assets = vec![(btc_assets.0, btc_assets.1, btc_assets.2, true, true)];
        // let mut endowed = BTreeMap::new();
        // let endowed_info = vec![(ALICE, 100), (BOB, 200), (CHARLIE, 300), (DAVE, 400)];
        // endowed.insert(btc_assets.0, endowed_info.clone());
        // endowed.insert(eth_assets.0, endowed_info);

        let mut init_assets = vec![];
        let mut assets_restrictions = vec![];
        for (a, b, c, d, e) in assets {
            init_assets.push((a, b, d, e));
            assets_restrictions.push((a, c))
        }

        let _ = xpallet_assets_registrar::GenesisConfig {
            assets: init_assets,
        }
        .assimilate_storage::<Test>(&mut storage);

        let _ = xpallet_assets::GenesisConfig::<Test> {
            assets_restrictions,
            endowed: Default::default(),
        }
        .assimilate_storage(&mut storage);

        let info = trustees_info();
        let genesis_trustees = info
            .iter()
            .find_map(|(chain, _, trustee_params)| {
                if *chain == Chain::Bitcoin {
                    Some(
                        trustee_params
                            .iter()
                            .map(|i| (i.0).clone())
                            .collect::<Vec<_>>(),
                    )
                } else {
                    None
                }
            })
            .unwrap();

        let _ = xpallet_gateway_common::GenesisConfig::<Test> { trustees: info }
            .assimilate_storage(&mut storage);

        let (genesis_info, genesis_hash, network_id) = load_mainnet_btc_genesis_header_info();

        let _ = GenesisConfig::<Test> {
            genesis_trustees,
            genesis_info,
            genesis_hash,
            network_id,
            params_info: BtcParams::new(
                486604799,            // max_bits
                2 * 60 * 60,          // block_max_future
                2 * 7 * 24 * 60 * 60, // target_timespan_seconds
                10 * 60,              // target_spacing_seconds
                4,                    // retargeting_factor
            ), // retargeting_factor
            verifier: BtcTxVerifier::Recover,
            confirmation_number: 4,
            btc_withdrawal_fee: 500000,
            max_withdrawal_count: 100,
        }
        .assimilate_storage(&mut storage);

        let ext = sp_io::TestExternalities::new(storage);
        ext
    }
    pub fn build_and_execute(self, test: impl FnOnce() -> ()) {
        let mut ext = self.build();
        ext.execute_with(|| System::set_block_number(1));
        ext.execute_with(test);
    }
}

pub fn alice() -> AccountId32 {
    sr25519::Keyring::Alice.to_account_id()
}
pub fn bob() -> AccountId32 {
    sr25519::Keyring::Bob.to_account_id()
}
pub fn charlie() -> AccountId32 {
    sr25519::Keyring::Charlie.to_account_id()
}
pub fn trustees() -> Vec<(AccountId32, Vec<u8>, Vec<u8>, Vec<u8>)> {
    vec![
        (
            alice(),
            b"Alice".to_vec(),
            hex!("02df92e88c4380778c9c48268460a124a8f4e7da883f80477deaa644ced486efc6").to_vec(),
            hex!("0386b58f51da9b37e59c40262153173bdb59d7e4e45b73994b99eec4d964ee7e88").to_vec(),
        ),
        (
            bob(),
            b"Bob".to_vec(),
            hex!("0244d81efeb4171b1a8a433b87dd202117f94e44c909c49e42e77b69b5a6ce7d0d").to_vec(),
            hex!("02e4631e46255571122d6e11cda75d5d601d5eb2585e65e4e87fe9f68c7838a278").to_vec(),
        ),
        (
            charlie(),
            b"Charlie".to_vec(),
            hex!("03a36339f413da869df12b1ab0def91749413a0dee87f0bfa85ba7196e6cdad102").to_vec(),
            hex!("0263d46c760d3e04883d4b433c9ce2bc32130acd9faad0192a2b375dbba9f865c3").to_vec(),
        ),
    ]
}

pub fn load_mainnet_btc_genesis_header_info() -> ((BtcHeader, u32), crate::H256, BtcNetwork) {
    (
        (
            BtcHeader {
                version: 536870912,
                previous_header_hash: h256_rev(
                    "0000000000000000000a4adf6c5192128535d4dcb56cfb5753755f8d392b26bf",
                ),
                merkle_root_hash: h256_rev(
                    "1d21e60acb0b12e5cfd3f775edb647f982a2d666f9886b2f61ea5e72577b0f5e",
                ),
                time: 1558168296,
                bits: Compact::new(388627269),
                nonce: 1439505020,
            },
            576576,
        ),
        h256_rev("0000000000000000001721f58deb88b0710295a02551f0dde1e2e231a15f1882"),
        BtcNetwork::Mainnet,
    )
}

fn trustees_info() -> Vec<(
    Chain,
    TrusteeInfoConfig,
    Vec<(AccountId, Vec<u8>, Vec<u8>, Vec<u8>)>,
)> {
    let btc_trustees = trustees();
    let btc_config = TrusteeInfoConfig {
        min_trustee_count: 3,
        max_trustee_count: 15,
    };
    vec![(Chain::Bitcoin, btc_config, btc_trustees)]
}

pub fn generate_blocks_576576_578692() -> BTreeMap<u32, BtcHeader> {
    let headers = include_str!("./res/headers-576576-578692.json");
    let headers: Vec<(u32, String)> = serde_json::from_str(headers).unwrap();
    headers
        .into_iter()
        .map(|(height, header_hex)| {
            let data = hex::decode(header_hex).unwrap();
            let header = serialization::deserialize(Reader::new(&data)).unwrap();
            (height, header)
        })
        .collect()
}

pub fn generate_blocks_478557_478563() -> (u32, Vec<BtcHeader>, Vec<BtcHeader>) {
    let b0 = BtcHeader {
        version: 0x20000002,
        previous_header_hash: h256_rev(
            "0000000000000000004801aaa0db00c30a6c8d89d16fd30a2115dda5a9fc3469",
        ),
        merkle_root_hash: h256_rev(
            "b2f6c37fb65308f2ff12cfc84e3b4c8d49b02534b86794d7f1dd6d6457327200",
        ),
        time: 1501593084,
        bits: Compact::new(0x18014735),
        nonce: 0x7a511539,
    }; // 478557  btc/bch common use

    let b1: BtcHeader = BtcHeader {
        version: 0x20000002,
        previous_header_hash: h256_rev(
            "000000000000000000eb9bc1f9557dc9e2cfe576f57a52f6be94720b338029e4",
        ),
        merkle_root_hash: h256_rev(
            "5b65144f6518bf4795abd428acd0c3fb2527e4e5c94b0f5a7366f4826001884a",
        ),
        time: 1501593374,
        bits: Compact::new(0x18014735),
        nonce: 0x7559dd16,
    }; //478558  bch forked from here

    let b2: BtcHeader = BtcHeader {
        version: 0x20000002,
        previous_header_hash: h256_rev(
            "0000000000000000011865af4122fe3b144e2cbeea86142e8ff2fb4107352d43",
        ),
        merkle_root_hash: h256_rev(
            "5fa62e1865455037450b7275d838d04f00230556129a4e86621a6bc4ad318c18",
        ),
        time: 1501593780,
        bits: Compact::new(0x18014735),
        nonce: 0xb78dbdba,
    }; // 478559

    let b3: BtcHeader = BtcHeader {
        version: 0x20000002,
        previous_header_hash: h256_rev(
            "00000000000000000019f112ec0a9982926f1258cdcc558dd7c3b7e5dc7fa148",
        ),
        merkle_root_hash: h256_rev(
            "8bd5e10005d8e01aa60278def2025d39b5a441261d934a24bd39e7423866787c",
        ),
        time: 1501594184,
        bits: Compact::new(0x18014735),
        nonce: 0x43628196,
    }; // 478560

    let b4: BtcHeader = BtcHeader {
        version: 0x20000002,
        previous_header_hash: h256_rev(
            "000000000000000000e512213f7303f72c5f7446e6e295f73c28cb024dd79e34",
        ),
        merkle_root_hash: h256_rev(
            "aaa533386910909ed6e6319a3ed2bb86774a8d1d9b373f975d53daad6b12170e",
        ),
        time: 1501594485,
        bits: Compact::new(0x18014735),
        nonce: 0xdabcc394,
    }; // 478561

    let b5: BtcHeader = BtcHeader {
        version: 0x20000002,
        previous_header_hash: h256_rev(
            "0000000000000000008876768068eea31f8f34e2f029765cd2ac998bdc3a2b2d",
        ),
        merkle_root_hash: h256_rev(
            "a51effefcc9eaac767ea211c661e5393d38bf3577b5b7e2d54471098b0ac4e35",
        ),
        time: 1501594711,
        bits: Compact::new(0x18014735),
        nonce: 0xa07f1745,
    }; // 478562

    let b2_fork: BtcHeader = BtcHeader {
        version: 0x20000000,
        previous_header_hash: h256_rev(
            "0000000000000000011865af4122fe3b144e2cbeea86142e8ff2fb4107352d43",
        ),
        merkle_root_hash: h256_rev(
            "c896c91a0be4d3eed5568bab4c3084945e5e06669be38ec06b1c8ca4d84baaab",
        ),
        time: 1501611161,
        bits: Compact::new(0x18014735),
        nonce: 0xe84aca22,
    }; // 478559

    let b3_fork: BtcHeader = BtcHeader {
        version: 0x20000000,
        previous_header_hash: h256_rev(
            "000000000000000000651ef99cb9fcbe0dadde1d424bd9f15ff20136191a5eec",
        ),
        merkle_root_hash: h256_rev(
            "088a7d29c4c6b95a74e362d64a801f492e748369a4fec1ca4e1ab47eefc8af82",
        ),
        time: 1501612386,
        bits: Compact::new(0x18014735),
        nonce: 0xcb72a740,
    }; // 478560
    let b4_fork: BtcHeader = BtcHeader {
        version: 0x20000002,
        previous_header_hash: h256_rev(
            "000000000000000000b15ad892af8f6aca4462d46d0b6e5884cadc033c8f257b",
        ),
        merkle_root_hash: h256_rev(
            "f64de8adf8dac328fb8f1dcb4ba19b6e94de7abc8c4eeaae83df8f62504e8758",
        ),
        time: 1501612639,
        bits: Compact::new(0x18014735),
        nonce: 0x0310f5e2,
    }; // 478561
    let b5_fork: BtcHeader = BtcHeader {
        version: 0x20000000,
        previous_header_hash: h256_rev(
            "00000000000000000013ee8874665f73862a3a0b6a30f895fe34f4c94d3e8a15",
        ),
        merkle_root_hash: h256_rev(
            "a464516af1dab6eadb963b62c5df0e503c8908af503dfff7a169b9d3f9851b11",
        ),
        time: 1501613578,
        bits: Compact::new(0x18014735),
        nonce: 0x0a24f4c4,
    }; // 478562
    let b6_fork: BtcHeader = BtcHeader {
        version: 0x20000000,
        previous_header_hash: h256_rev(
            "0000000000000000005c6e82aa704d326a3a2d6a4aa09f1725f532da8bb8de4d",
        ),
        merkle_root_hash: h256_rev(
            "a27fac4ab26df6e12a33b2bb853140d7e231326ddbc9a1d6611b553b0645a040",
        ),
        time: 1501616264,
        bits: Compact::new(0x18014735),
        nonce: 0x6bd75df1,
    }; // 478563

    (
        478557,
        vec![b0.clone(), b1, b2, b3, b4, b5],
        vec![b0, b1, b2_fork, b3_fork, b4_fork, b5_fork, b6_fork],
    )
}
