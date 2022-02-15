// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use frame_support::{parameter_types, sp_io, traits::GenesisBuild};
use frame_system::EnsureRoot;
use sp_core::H256;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
};

use chainx_primitives::AssetId;

pub use xp_protocol::{X_BTC, X_ETH};

use crate::{self as xpallet_gateway_records, *};

/// The AccountId alias in this test module.
pub(crate) type AccountId = u64;
pub(crate) type BlockNumber = u64;
pub(crate) type Balance = u128;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
    pub enum Test where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
        Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
        Assets: xpallet_assets::{Pallet, Call, Storage, Event<T>},
        XGatewayRecords: xpallet_gateway_records::{Pallet, Call, Storage, Event<T>},
    }
);

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const SS58Prefix: u8 = 42;
}

impl frame_system::Config for Test {
    type BaseCallFilter = frame_support::traits::Everything;
    type BlockWeights = ();
    type BlockLength = ();
    type Origin = Origin;
    type Call = Call;
    type Index = u64;
    type BlockNumber = BlockNumber;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = u64;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type Event = ();
    type BlockHashCount = BlockHashCount;
    type DbWeight = ();
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<Balance>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = SS58Prefix;
    type OnSetCode = ();
}

parameter_types! {
    pub const ExistentialDeposit: u64 = 0;
    pub const MaxReserves: u32 = 50;
}
impl pallet_balances::Config for Test {
    type MaxLocks = ();
    type Balance = Balance;
    type DustRemoval = ();
    type Event = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
    type ReserveIdentifier = [u8; 8];
    type MaxReserves = MaxReserves;
}

parameter_types! {
    pub const AssetDeposit: Balance = 1;
    pub const ApprovalDeposit: Balance = 1;
    pub const StringLimit: u32 = 50;
    pub const MetadataDepositBase: Balance = 1;
    pub const MetadataDepositPerByte: Balance = 1;
}

impl xpallet_assets::Config for Test {
    type Event = ();
    type Balance = Balance;
    type AssetId = AssetId;
    type Currency = Balances;
    type ForceOrigin = EnsureRoot<AccountId>;
    type AssetDeposit = AssetDeposit;
    type MetadataDepositBase = MetadataDepositBase;
    type MetadataDepositPerByte = MetadataDepositPerByte;
    type ApprovalDeposit = ApprovalDeposit;
    type StringLimit = StringLimit;
    type Freezer = XGatewayRecords;
    type Extra = ();
    type WeightInfo = xpallet_assets::weights::SubstrateWeight<Test>;
}

// assets
parameter_types! {
    pub const BtcAssetId: AssetId = 1;
}

impl Config for Test {
    type Event = ();
    type BtcAssetId = BtcAssetId;
    type Currency = Balances;
    type WeightInfo = xpallet_gateway_records::weights::SubstrateWeight<Test>;
}

pub type XRecordsErr = Error<Test>;

pub const ALICE: AccountId = 1;

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

        let _ = crate::GenesisConfig::<Test> {
            initial_asset_chain: vec![(X_BTC, Chain::Bitcoin), (X_ETH, Chain::Ethereum)],
        }
        .assimilate_storage(&mut storage);
        let _ = xpallet_assets::GenesisConfig::<Test> {
            assets: vec![(X_BTC, ALICE, true, 1), (X_ETH, ALICE, true, 1)],
            metadata: vec![
                (
                    X_BTC,
                    "XBTC".to_string().into_bytes(),
                    "XBTC".to_string().into_bytes(),
                    8,
                ),
                (
                    X_ETH,
                    "XETH".to_string().into_bytes(),
                    "XETH".to_string().into_bytes(),
                    18,
                ),
            ],
            accounts: vec![],
        }
        .assimilate_storage(&mut storage);
        sp_io::TestExternalities::new(storage)
    }
    pub fn build_and_execute(self, test: impl FnOnce()) {
        let mut ext = self.build();
        ext.execute_with(|| System::set_block_number(1));
        ext.execute_with(test);
    }
}
