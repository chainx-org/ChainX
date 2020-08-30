use core::time::Duration;
use std::cell::RefCell;

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

use super::mock_impls;
use crate::trustees;
use crate::types::TrusteeInfoConfig;

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

impl crate::Trait for Test {
    type Event = ();
    type Validator = ();
    type DetermineMultisigAddress = MultisigAddr;
    type Bitcoin = mock_impls::MockBitcoin<Test>;
    type BitcoinTrustee = mock_impls::MockBitcoin<Test>;
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
    type AccountExtractor = ();
    type TrusteeSessionProvider = ();
    type TrusteeOrigin = EnsureSignedBy<trustees::bitcoin::BtcTrusteeMultisig<Test>, AccountId>;
    type Channel = (); // mock_impls::MockCommon<Test>;
    type AddrBinding = (); //mock_impls::MockCommon<Test>;
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

        let _ = crate::GenesisConfig::<Test> {
            trustees: trustees(),
        }
        .assimilate_storage(&mut storage);

        let ext = sp_io::TestExternalities::new(storage);
        ext
    }
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
