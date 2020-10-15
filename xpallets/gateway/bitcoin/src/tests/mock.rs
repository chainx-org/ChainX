// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

#![cfg(test)]

use super::common::*;

use core::time::Duration;
use std::cell::RefCell;

// Substrate
use frame_support::{impl_outer_origin, parameter_types, sp_io, weights::Weight};
use frame_system::EnsureSignedBy;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
    AccountId32, Perbill,
};

// light-bitcoin
use light_bitcoin::primitives::Compact;

use chainx_primitives::AssetId;
use xpallet_assets::AssetRestrictions;
use xpallet_assets_registrar::AssetInfo;
use xpallet_gateway_common::types::TrusteeInfoConfig;

pub use xpallet_protocol::X_BTC;
pub use xpallet_protocol::X_ETH;

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
}

impl frame_system::Trait for Test {
    type BaseCallFilter = ();
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
    type MaximumBlockWeight = MaximumBlockWeight;
    type DbWeight = ();
    type BlockExecutionWeight = ();
    type ExtrinsicBaseWeight = ();
    type MaximumExtrinsicWeight = MaximumBlockWeight;
    type MaximumBlockLength = MaximumBlockLength;
    type AvailableBlockRatio = AvailableBlockRatio;
    type Version = ();
    type PalletInfo = ();
    type AccountData = pallet_balances::AccountData<Balance>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
}

parameter_types! {
    pub const ExistentialDeposit: u64 = 0;
}
impl pallet_balances::Trait for Test {
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

impl xpallet_assets_registrar::Trait for Test {
    type Event = ();
    type NativeAssetId = ChainXAssetId;
    type RegistrarHandler = ();
    type WeightInfo = ();
}

impl xpallet_assets::Trait for Test {
    type Event = ();
    type Currency = Balances;
    type Amount = Amount;
    type TreasuryAccount = ();
    type OnCreatedAccount = frame_system::CallOnCreatedAccount<Test>;
    type OnAssetChanged = ();
    type WeightInfo = ();
}

impl xpallet_gateway_records::Trait for Test {
    type Event = ();
    type WeightInfo = ();
}

impl xpallet_gateway_common::Trait for Test {
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

impl Trait for Test {
    type Event = ();
    type UnixTime = Timestamp;
    type AccountExtractor = xpallet_gateway_common::extractor::Extractor;
    type TrusteeSessionProvider =
        xpallet_gateway_common::trustees::bitcoin::BtcTrusteeSessionManager<Test>;
    type TrusteeOrigin = EnsureSignedBy<
        xpallet_gateway_common::trustees::bitcoin::BtcTrusteeMultisig<Test>,
        AccountId,
    >;
    type Channel = XGatewayCommon;
    type AddrBinding = XGatewayCommon;
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
            reserved_block: 2100,
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
            reserved_block: 2100,
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

pub fn load_mainnet_btc_genesis_header_info() -> ((BtcHeader, u32), H256, BtcNetwork) {
    (
        (
            BtcHeader {
                version: 536870912,
                previous_header_hash: as_h256(
                    "0000000000000000000a4adf6c5192128535d4dcb56cfb5753755f8d392b26bf",
                ),
                merkle_root_hash: as_h256(
                    "1d21e60acb0b12e5cfd3f775edb647f982a2d666f9886b2f61ea5e72577b0f5e",
                ),
                time: 1558168296,
                bits: Compact::new(388627269),
                nonce: 1439505020,
            },
            576576,
        ),
        as_h256("0000000000000000001721f58deb88b0710295a02551f0dde1e2e231a15f1882"),
        BtcNetwork::Mainnet,
    )
}

fn trustees_info() -> Vec<(
    Chain,
    TrusteeInfoConfig,
    Vec<(AccountId, Vec<u8>, Vec<u8>, Vec<u8>)>,
)> {
    let btc_trustees = trustees::<Test>();
    let btc_config = TrusteeInfoConfig {
        min_trustee_count: 3,
        max_trustee_count: 15,
    };
    vec![(Chain::Bitcoin, btc_config, btc_trustees)]
}

pub fn generate_mock_blocks() -> (u32, Vec<BtcHeader>, Vec<BtcHeader>) {
    let b0 = BtcHeader {
        version: 0x20000002,
        previous_header_hash: as_h256(
            "0000000000000000004801aaa0db00c30a6c8d89d16fd30a2115dda5a9fc3469",
        ),
        merkle_root_hash: as_h256(
            "b2f6c37fb65308f2ff12cfc84e3b4c8d49b02534b86794d7f1dd6d6457327200",
        ),
        time: 1501593084,
        bits: Compact::new(0x18014735),
        nonce: 0x7a511539,
    }; // 478557  btc/bch common use

    let b1: BtcHeader = BtcHeader {
        version: 0x20000002,
        previous_header_hash: as_h256(
            "000000000000000000eb9bc1f9557dc9e2cfe576f57a52f6be94720b338029e4",
        ),
        merkle_root_hash: as_h256(
            "5b65144f6518bf4795abd428acd0c3fb2527e4e5c94b0f5a7366f4826001884a",
        ),
        time: 1501593374,
        bits: Compact::new(0x18014735),
        nonce: 0x7559dd16,
    }; //478558  bch forked from here

    let b2: BtcHeader = BtcHeader {
        version: 0x20000002,
        previous_header_hash: as_h256(
            "0000000000000000011865af4122fe3b144e2cbeea86142e8ff2fb4107352d43",
        ),
        merkle_root_hash: as_h256(
            "5fa62e1865455037450b7275d838d04f00230556129a4e86621a6bc4ad318c18",
        ),
        time: 1501593780,
        bits: Compact::new(0x18014735),
        nonce: 0xb78dbdba,
    }; // 478559

    let b3: BtcHeader = BtcHeader {
        version: 0x20000002,
        previous_header_hash: as_h256(
            "00000000000000000019f112ec0a9982926f1258cdcc558dd7c3b7e5dc7fa148",
        ),
        merkle_root_hash: as_h256(
            "8bd5e10005d8e01aa60278def2025d39b5a441261d934a24bd39e7423866787c",
        ),
        time: 1501594184,
        bits: Compact::new(0x18014735),
        nonce: 0x43628196,
    }; // 478560

    let b4: BtcHeader = BtcHeader {
        version: 0x20000002,
        previous_header_hash: as_h256(
            "000000000000000000e512213f7303f72c5f7446e6e295f73c28cb024dd79e34",
        ),
        merkle_root_hash: as_h256(
            "aaa533386910909ed6e6319a3ed2bb86774a8d1d9b373f975d53daad6b12170e",
        ),
        time: 1501594485,
        bits: Compact::new(0x18014735),
        nonce: 0xdabcc394,
    }; // 478561

    let b5: BtcHeader = BtcHeader {
        version: 0x20000002,
        previous_header_hash: as_h256(
            "0000000000000000008876768068eea31f8f34e2f029765cd2ac998bdc3a2b2d",
        ),
        merkle_root_hash: as_h256(
            "a51effefcc9eaac767ea211c661e5393d38bf3577b5b7e2d54471098b0ac4e35",
        ),
        time: 1501594711,
        bits: Compact::new(0x18014735),
        nonce: 0xa07f1745,
    }; // 478562

    let b2_fork: BtcHeader = BtcHeader {
        version: 0x20000000,
        previous_header_hash: as_h256(
            "0000000000000000011865af4122fe3b144e2cbeea86142e8ff2fb4107352d43",
        ),
        merkle_root_hash: as_h256(
            "c896c91a0be4d3eed5568bab4c3084945e5e06669be38ec06b1c8ca4d84baaab",
        ),
        time: 1501611161,
        bits: Compact::new(0x18014735),
        nonce: 0xe84aca22,
    }; // 478559

    let b3_fork: BtcHeader = BtcHeader {
        version: 0x20000000,
        previous_header_hash: as_h256(
            "000000000000000000651ef99cb9fcbe0dadde1d424bd9f15ff20136191a5eec",
        ),
        merkle_root_hash: as_h256(
            "088a7d29c4c6b95a74e362d64a801f492e748369a4fec1ca4e1ab47eefc8af82",
        ),
        time: 1501612386,
        bits: Compact::new(0x18014735),
        nonce: 0xcb72a740,
    }; // 478560
    let b4_fork: BtcHeader = BtcHeader {
        version: 0x20000002,
        previous_header_hash: as_h256(
            "000000000000000000b15ad892af8f6aca4462d46d0b6e5884cadc033c8f257b",
        ),
        merkle_root_hash: as_h256(
            "f64de8adf8dac328fb8f1dcb4ba19b6e94de7abc8c4eeaae83df8f62504e8758",
        ),
        time: 1501612639,
        bits: Compact::new(0x18014735),
        nonce: 0x0310f5e2,
    }; // 478561
    let b5_fork: BtcHeader = BtcHeader {
        version: 0x20000000,
        previous_header_hash: as_h256(
            "00000000000000000013ee8874665f73862a3a0b6a30f895fe34f4c94d3e8a15",
        ),
        merkle_root_hash: as_h256(
            "a464516af1dab6eadb963b62c5df0e503c8908af503dfff7a169b9d3f9851b11",
        ),
        time: 1501613578,
        bits: Compact::new(0x18014735),
        nonce: 0x0a24f4c4,
    }; // 478562
    let b6_fork: BtcHeader = BtcHeader {
        version: 0x20000000,
        previous_header_hash: as_h256(
            "0000000000000000005c6e82aa704d326a3a2d6a4aa09f1725f532da8bb8de4d",
        ),
        merkle_root_hash: as_h256(
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
