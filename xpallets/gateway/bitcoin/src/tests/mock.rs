// Copyright 2018-2019 Chainpool.

#![cfg(test)]

use crate::*;

use core::time::Duration;
use std::cell::RefCell;

// Substrate
use frame_support::{impl_outer_origin, parameter_types, sp_io, weights::Weight};
use frame_system::EnsureSignedBy;
use sp_core::crypto::UncheckedInto;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
    AccountId32, Perbill,
};
use sp_std::collections::btree_map::BTreeMap;

// light-bitcoin
use light_bitcoin::primitives::Compact;
use light_bitcoin::serialization;

// use xbridge_common::traits::IntoVecu8;
use chainx_primitives::AssetId;
use xpallet_assets::{AssetRestriction, AssetRestrictions};
use xpallet_assets_registrar::AssetInfo;
use xpallet_gateway_common::types::TrusteeInfoConfig;

use crate::tests::as_h256;

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
    type ModuleToIndex = ();
    type AccountData = pallet_balances::AccountData<Balance>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
}

parameter_types! {
    pub const ExistentialDeposit: u64 = 0;
}
impl pallet_balances::Trait for Test {
    type Balance = Balance;
    type DustRemoval = ();
    type Event = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
}

// parameter_types! {
//     pub const DepositBase: u64 = 1;
//     pub const DepositFactor: u64 = 1;
//     pub const MaxSignatories: u16 = 3;
// }
// impl pallet_multisig::Trait for Test {
//     type Event = ();
//     type Call = Call;
//     type Currency = Balances;
//     type DepositBase = DepositBase;
//     type DepositFactor = DepositFactor;
//     type MaxSignatories = MaxSignatories;
//     type WeightInfo = ();
// }

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
}

impl xpallet_gateway_common::Trait for Test {
    type Event = ();
    type Validator = ();
    type DetermineMultisigAddress = ();
    type Bitcoin = XGatewayBitcoin;
    type BitcoinTrustee = XGatewayBitcoin;
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
}

pub type System = frame_system::Module<Test>;
pub type Balances = pallet_balances::Module<Test>;
pub type XAssets = xpallet_assets::Module<Test>;
// pub type XGatewayRecords = xpallet_gateway_records::Module<Test>;
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
        AssetRestriction::DestroyUsable.into(),
    )
}

lazy_static::lazy_static! {
    pub static ref ALICE: AccountId = H256::repeat_byte(1).unchecked_into();
    pub static ref BOB: AccountId = H256::repeat_byte(2).unchecked_into();
    pub static ref CHARLIE: AccountId = H256::repeat_byte(3).unchecked_into();
    pub static ref DAVE: AccountId = H256::repeat_byte(4).unchecked_into();
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
        let _ = GenesisConfig {
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
        .assimilate_storage::<Test>(&mut storage);

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

        let _ = xpallet_gateway_common::GenesisConfig::<Test> {
            trustees: trustees(),
        }
        .assimilate_storage(&mut storage);

        let (genesis_info, genesis_hash, network_id) = load_mainnet_btc_genesis_header_info();

        let _ = GenesisConfig {
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
        .assimilate_storage::<Test>(&mut storage);

        let ext = sp_io::TestExternalities::new(storage);
        ext
    }
    pub fn build_and_execute(self, test: impl FnOnce() -> ()) {
        let mut ext = self.build();
        ext.execute_with(|| System::set_block_number(1));
        ext.execute_with(test);
    }
}

// pub fn load_mock_btc_genesis_header_info() -> ((BtcHeader, u32), H256, BtcNetwork) {
//     (
//         (
//             BtcHeader {
//                 version: 0x20000002,
//                 previous_header_hash: as_h256(
//                     "000000000000000000eb9bc1f9557dc9e2cfe576f57a52f6be94720b338029e4",
//                 ),
//                 merkle_root_hash: as_h256(
//                     "5b65144f6518bf4795abd428acd0c3fb2527e4e5c94b0f5a7366f4826001884a",
//                 ),
//                 time: 1501593374,
//                 bits: Compact::new(0x18014735),
//                 nonce: 0x7559dd16,
//             },
//             478558,
//         ),
//         as_h256("0000000000000000011865af4122fe3b144e2cbeea86142e8ff2fb4107352d43"),
//         BtcNetwork::Mainnet,
//     )
// }

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

fn trustees() -> Vec<(
    Chain,
    TrusteeInfoConfig,
    Vec<(AccountId, Vec<u8>, Vec<u8>, Vec<u8>)>,
)> {
    let btc_trustees = vec![
        (
            ALICE.clone(),
            b"".to_vec(),
            hex::decode("02df92e88c4380778c9c48268460a124a8f4e7da883f80477deaa644ced486efc6")
                .expect("hex decode failed")
                .into(),
            hex::decode("0386b58f51da9b37e59c40262153173bdb59d7e4e45b73994b99eec4d964ee7e88")
                .expect("hex decode failed")
                .into(),
        ),
        (
            BOB.clone(),
            b"".to_vec(),
            hex::decode("0244d81efeb4171b1a8a433b87dd202117f94e44c909c49e42e77b69b5a6ce7d0d")
                .expect("hex decode failed")
                .into(),
            hex::decode("02e4631e46255571122d6e11cda75d5d601d5eb2585e65e4e87fe9f68c7838a278")
                .expect("hex decode failed")
                .into(),
        ),
        (
            CHARLIE.clone(),
            b"".to_vec(),
            hex::decode("03a36339f413da869df12b1ab0def91749413a0dee87f0bfa85ba7196e6cdad102")
                .expect("hex decode failed")
                .into(),
            hex::decode("0263d46c760d3e04883d4b433c9ce2bc32130acd9faad0192a2b375dbba9f865c3")
                .expect("hex decode failed")
                .into(),
        ),
    ];
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

pub fn generate_blocks() -> BTreeMap<u32, BtcHeader> {
    let bytes = include_bytes!("./res/headers-576576-578692.json");
    let headers: Vec<(u32, String)> = serde_json::from_slice(&bytes[..]).expect("should not fail");
    headers
        .into_iter()
        .map(|(height, h)| {
            let hex = hex::decode(h).expect("should be valid hex");
            let header =
                serialization::deserialize(Reader::new(&hex)).expect("should be valid header");
            (height, header)
        })
        .collect()
}

/*
pub struct DummyTrusteeSession;
impl xbridge_common::traits::TrusteeSession<AccountId, TrusteeAddrInfo> for DummyTrusteeSession {
    fn trustee_session(
        _: u32,
    ) -> result::Result<TrusteeSessionInfo<AccountId, TrusteeAddrInfo>, &'static str> {
        Ok(TrusteeSessionInfo {
            trustee_list: [
                AccountId::from_slice(&[0]),
                AccountId::from_slice(&[1]),
                AccountId::from_slice(&[2]),
            ]
            .to_vec(),
            hot_address: TrusteeAddrInfo::from_vecu8(&[0]).unwrap(),
            cold_address: TrusteeAddrInfo::from_vecu8(&[1]).unwrap(),
        })
    }

    fn current_trustee_session(
    ) -> std::result::Result<TrusteeSessionInfo<AccountId, TrusteeAddrInfo>, &'static str> {
        Ok(TrusteeSessionInfo {
            trustee_list: [
                AccountId::from_slice(&[0]),
                AccountId::from_slice(&[1]),
                AccountId::from_slice(&[2]),
            ]
            .to_vec(),
            hot_address: TrusteeAddrInfo::from_vecu8(&[0]).unwrap(),
            cold_address: TrusteeAddrInfo::from_vecu8(&[1]).unwrap(),
        })
    }

    fn last_trustee_session(
    ) -> std::result::Result<TrusteeSessionInfo<AccountId, TrusteeAddrInfo>, &'static str> {
        Ok(TrusteeSessionInfo {
            trustee_list: [
                AccountId::from_slice(&[0]),
                AccountId::from_slice(&[1]),
                AccountId::from_slice(&[2]),
            ]
            .to_vec(),
            hot_address: TrusteeAddrInfo::from_vecu8(&[0]).unwrap(),
            cold_address: TrusteeAddrInfo::from_vecu8(&[1]).unwrap(),
        })
    }
}

pub struct DummyCrossChain;
impl xbridge_common::traits::CrossChainBinding<AccountId, BTCAddress> for DummyCrossChain {
    fn update_binding(_: &AccountId, _: btc_keys::Address, _: Option<Vec<u8>>) {}

    fn get_binding_info(_: &btc_keys::Address) -> Option<(AccountId, Option<AccountId>)> {
        Some((
            AccountId::from_slice(&[0]),
            Some(AccountId::from_slice(&[1])),
        ))
    }
}

pub struct DummyBitcoinTrusteeMultiSig;
impl xbridge_common::traits::TrusteeMultiSig<AccountId> for DummyBitcoinTrusteeMultiSig {
    fn multisig_for_trustees() -> AccountId {
        AccountId::from_slice(&[9])
    }
}

pub type XAssets = xpallet_assets::Module<Test>;
pub type XBridgeOfBTC = Module<Test>;
pub type XBridgeOfBTCLockup = lockup::Module<Test>;

pub fn new_test_ext() -> runtime_io::TestExternalities<Blake2Hasher> {
    let mut r = frame_system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap()
        .0;

    // bridge btc
    r.extend(
        GenesisConfig::<Test> {
            // start genesis block: (genesis, blocknumber)
            genesis: (
                BtcHeader {
                    version: 980090880,
                    previous_header_hash: as_h256(
                        "00000000000000ab706b663326210d03780fea6ecfe0cc59c78f0c7dddba9cc2",
                    ),
                    merkle_root_hash: as_h256(
                        "91ee572484dabc6edf5a8da44a4fb55b5040facf66624b2a37c4f633070c60c8",
                    ),
                    time: 1550454022,
                    bits: Compact::new(436283074),
                    nonce: 47463732,
                },
                1457525,
            ),
            genesis_hash: as_h256(
                "0000000000000059227e29b86313c99ac908a1d71db97632b402f13a569b4709",
            ),
            params_info: BTCParams::new(
                520159231,            // max_bits
                2 * 60 * 60,          // block_max_future
                2 * 7 * 24 * 60 * 60, // target_timespan_seconds
                10 * 60,              // target_spacing_seconds
                4,
            ), // retargeting_factor
            network_id: 0,
            confirmation_number: 3,
            reserved_block: 2100,
            btc_withdrawal_fee: 1000,
            max_withdrawal_count: 100,
            _genesis_phantom_data: Default::default(),
        }
        .build_storage()
        .unwrap()
        .0,
    );
    r.into()
}

pub fn new_test_mainnet() -> runtime_io::TestExternalities<Blake2Hasher> {
    let mut r = frame_system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap()
        .0;

    r.extend(
        xsystem::GenesisConfig::<Test> {
            network_props: (xsystem::NetworkType::Mainnet, 44),
            _genesis_phantom_data: Default::default(),
        }
        .build_storage()
        .unwrap()
        .0,
    );

    // bridge btc
    r.extend(
        GenesisConfig::<Test> {
            // start genesis block: (genesis, blocknumber)
            genesis: (
                BtcHeader {
                    version: 545259520,
                    previous_header_hash: as_h256(
                        "00000000000000000001b2505c11119fcf29be733ec379f686518bf1090a522a",
                    ),
                    merkle_root_hash: as_h256(
                        "cc09d95fd8ccc985826b9eb46bf73f8449116f18535423129f0574500985cf90",
                    ),
                    time: 1556958733,
                    bits: Compact::new(388628280),
                    nonce: 2897942742,
                },
                574560,
            ),
            genesis_hash: as_h256(
                "00000000000000000008c8427670a65dec4360e88bf6c8381541ef26b30bd8fc",
            ),
            params_info: BTCParams::new(
                486604799,            // max_bits
                2 * 60 * 60,          // block_max_future
                2 * 7 * 24 * 60 * 60, // target_timespan_seconds
                10 * 60,              // target_spacing_seconds
                4,                    // retargeting_factor
            ), // retargeting_factor
            network_id: 0,
            confirmation_number: 4,
            reserved_block: 2100,
            btc_withdrawal_fee: 40000,
            max_withdrawal_count: 100,
            _genesis_phantom_data: Default::default(),
        }
        .build_storage()
        .unwrap()
        .0,
    );
    r.into()
}




*/
