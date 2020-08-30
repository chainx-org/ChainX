use core::time::Duration;
use std::cell::RefCell;
use std::str::FromStr;

use codec::{Decode, Encode};

// Substrate
use frame_support::traits::UnixTime;
use frame_support::{impl_outer_origin, parameter_types, sp_io, weights::Weight};
use frame_system::EnsureSignedBy;
use sp_core::{crypto::UncheckedInto, H256};
use sp_io::hashing::blake2_256;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
    AccountId32, Perbill,
};

use chainx_primitives::AssetId;
use xpallet_assets::{AssetRestriction, AssetRestrictions};
use xpallet_assets_registrar::{AssetInfo, Chain};
use xpallet_gateway_bitcoin::{BtcHeader, BtcNetwork, BtcParams, BtcTxVerifier, Compact};

use xpallet_gateway_common::trustees;
use xpallet_gateway_common::types::TrusteeInfoConfig;

pub use xpallet_protocol::X_BTC;
pub use xpallet_protocol::X_ETH;
use xpallet_support::traits::MultisigAddressFor;

pub(crate) type AccountId = AccountId32;
// pub type Signature = MultiSignature;
// pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;
// pub type AccountId = u64;
pub(crate) type BlockNumber = u64;
pub(crate) type Balance = u128;
pub(crate) type Amount = i128;

impl_outer_origin! {
    pub enum Origin for Test where system = frame_system {}
}

#[derive(Clone, Eq, PartialEq)]
pub struct Test;
pub type System = frame_system::Module<Test>;
pub type Balances = pallet_balances::Module<Test>;
pub type XAssets = xpallet_assets::Module<Test>;
pub type XGatewayRecords = xpallet_gateway_records::Module<Test>;
pub type XGatewayBitcoin = xpallet_gateway_bitcoin::Module<Test>;
pub type XGatewayCommon = xpallet_gateway_common::Module<Test>;
// pub type XGatewayBitcoinErr = xpallet_gateway_bitcoin::Error<Test>;
pub type XGatewayCommonErr = xpallet_gateway_common::Error<Test>;

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

pub struct MultisigAddr;
impl MultisigAddressFor<AccountId> for MultisigAddr {
    fn calc_multisig(who: &[AccountId], threshold: u16) -> AccountId {
        let entropy = (b"modlpy/utilisuba", who, threshold).using_encoded(blake2_256);
        AccountId::decode(&mut &entropy[..]).unwrap_or_default()
    }
}

impl xpallet_gateway_common::Trait for Test {
    type Event = ();
    type Validator = ();
    type DetermineMultisigAddress = MultisigAddr;
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

impl xpallet_gateway_bitcoin::Trait for Test {
    type Event = ();
    type UnixTime = Timestamp;
    type AccountExtractor = xpallet_gateway_common::extractor::Extractor;
    type TrusteeSessionProvider =
        xpallet_gateway_common::trustees::bitcoin::BtcTrusteeSessionManager<Test>;
    type TrusteeOrigin = EnsureSignedBy<trustees::bitcoin::BtcTrusteeMultisig<Test>, AccountId>;
    type Channel = XGatewayCommon;
    type AddrBinding = XGatewayCommon;
}

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

        let (genesis_info, genesis_hash, network_id) = load_mainnet_btc_genesis_header_info();

        let _ = xpallet_gateway_bitcoin::GenesisConfig {
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

        let _ = xpallet_gateway_common::GenesisConfig::<Test> {
            trustees: trustees(),
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

fn as_h256(s: &str) -> H256 {
    let h = H256::from_str(s).unwrap();
    fn reverse_h256(mut hash: H256) -> H256 {
        let bytes = hash.as_bytes_mut();
        bytes.reverse();
        H256::from_slice(bytes)
    }
    reverse_h256(h)
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
