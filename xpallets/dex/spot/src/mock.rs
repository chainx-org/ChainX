// Copyright 2019-2022 ChainX Project Authors. Licensed under GPL-3.0.

use std::{
    cell::RefCell,
    collections::{BTreeMap, HashSet},
};

use frame_support::{
    parameter_types,
    traits::{GenesisBuild, Get},
    weights::Weight,
};
use sp_core::H256;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
    Perbill,
};

use chainx_primitives::{AssetId, BlockNumber};
use xp_protocol::{BTC_DECIMALS, PCX, PCX_DECIMALS, X_BTC, X_DOT};
use xpallet_assets::{AssetInfo, AssetRestrictions, Chain};

use crate::{self as xpallet_dex_spot, *};

/// The AccountId alias in this test module.
pub(crate) type AccountId = u64;
pub(crate) type AccountIndex = u64;
pub(crate) type Balance = u128;
pub(crate) type Price = u128;

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
        XAssetsRegistrar: xpallet_assets_registrar::{Pallet, Call, Config, Storage, Event<T>},
        XAssets: xpallet_assets::{Pallet, Call, Storage, Event<T>, Config<T>},
        XSpot: xpallet_dex_spot::{Pallet, Call, Storage, Event<T>, Config<T>},
    }
);

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const MaximumBlockWeight: Weight = 1024;
    pub const MaximumBlockLength: u32 = 2 * 1024;
    pub const AvailableBlockRatio: Perbill = Perbill::from_percent(75);
    pub const SS58Prefix: u8 = 42;
}

impl frame_system::Config for Test {
    type BaseCallFilter = frame_support::traits::Everything;
    type BlockWeights = ();
    type BlockLength = ();
    type Origin = Origin;
    type Call = Call;
    type Index = AccountIndex;
    type BlockNumber = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = AccountId;
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

pub struct ExistentialDeposit;
impl Get<Balance> for ExistentialDeposit {
    fn get() -> Balance {
        EXISTENTIAL_DEPOSIT.with(|v| *v.borrow())
    }
}

parameter_types! {
    pub const MaxReserves: u32 = 50;
}
impl pallet_balances::Config for Test {
    type MaxLocks = ();
    type Balance = Balance;
    type Event = ();
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
    type ReserveIdentifier = [u8; 8];
    type MaxReserves = MaxReserves;
}

impl Config for Test {
    type Event = ();
    type Price = Price;
    type WeightInfo = ();
}

parameter_types! {
    pub const ChainXAssetId: AssetId = 0;
}

impl xpallet_assets_registrar::Config for Test {
    type Event = ();
    type NativeAssetId = ChainXAssetId;
    type RegistrarHandler = XSpot;
    type WeightInfo = ();
}

impl xpallet_assets::Config for Test {
    type Event = ();
    type Currency = Balances;
    type TreasuryAccount = ();
    type OnCreatedAccount = frame_system::Provider<Test>;
    type OnAssetChanged = ();
    type WeightInfo = ();
}

thread_local! {
    static SESSION: RefCell<(Vec<AccountId>, HashSet<AccountId>)> = RefCell::new(Default::default());
    static EXISTENTIAL_DEPOSIT: RefCell<Balance> = RefCell::new(0);
    static ELECTION_LOOKAHEAD: RefCell<BlockNumber> = RefCell::new(0);
    static PERIOD: RefCell<BlockNumber> = RefCell::new(1);
    static MAX_ITERATIONS: RefCell<u32> = RefCell::new(0);
}

#[derive(Default)]
pub struct ExtBuilder;

fn pcx() -> (AssetId, AssetInfo, AssetRestrictions) {
    (
        PCX,
        AssetInfo::new::<Test>(
            b"PCX".to_vec(),
            b"Polkadot ChainX".to_vec(),
            Chain::ChainX,
            PCX_DECIMALS,
            b"ChainX's crypto currency in Polkadot ecology".to_vec(),
        )
        .unwrap(),
        AssetRestrictions::DEPOSIT
            | AssetRestrictions::WITHDRAW
            | AssetRestrictions::DESTROY_WITHDRAWAL
            | AssetRestrictions::DESTROY_USABLE,
    )
}

pub(crate) fn btc() -> (AssetId, AssetInfo, AssetRestrictions) {
    (
        X_BTC,
        AssetInfo::new::<Test>(
            b"X-BTC".to_vec(),
            b"X-BTC".to_vec(),
            Chain::Bitcoin,
            BTC_DECIMALS,
            b"ChainX's cross-chain Bitcoin".to_vec(),
        )
        .unwrap(),
        AssetRestrictions::DEPOSIT
            | AssetRestrictions::WITHDRAW
            | AssetRestrictions::DESTROY_WITHDRAWAL
            | AssetRestrictions::DESTROY_USABLE,
    )
}

impl ExtBuilder {
    pub fn build(self) -> sp_io::TestExternalities {
        let _ = env_logger::try_init();
        let mut storage = frame_system::GenesisConfig::default()
            .build_storage::<Test>()
            .unwrap();

        let pcx_asset = pcx();
        let btc_asset = btc();
        let assets = vec![
            (pcx_asset.0, pcx_asset.1, pcx_asset.2, true, false),
            (btc_asset.0, btc_asset.1, pcx_asset.2, true, true),
        ];

        let mut init_assets = vec![];
        let mut assets_restrictions = vec![];
        for (a, b, c, d, e) in assets {
            init_assets.push((a, b, d, e));
            assets_restrictions.push((a, c))
        }

        GenesisBuild::<Test>::assimilate_storage(
            &xpallet_assets_registrar::GenesisConfig {
                assets: init_assets,
            },
            &mut storage,
        )
        .unwrap();

        let endowed = BTreeMap::new();
        let _ = xpallet_assets::GenesisConfig::<Test> {
            assets_restrictions,
            endowed,
        }
        .assimilate_storage(&mut storage);

        let trading_pairs = vec![
            (PCX, X_BTC, 9, 2, 100000, true),
            (X_DOT, PCX, 4, 2, 100000, true),
        ];

        let _ = xpallet_dex_spot::GenesisConfig::<Test> { trading_pairs }
            .assimilate_storage(&mut storage);

        let mut ext = sp_io::TestExternalities::from(storage);

        // We consider all test to start after timestamp is initialized
        // This must be ensured by having `timestamp::on_initialize` called before
        // `staking::on_initialize`
        ext.execute_with(|| {
            System::set_block_number(1);
        });

        ext
    }
    pub fn build_and_execute(self, test: impl FnOnce()) {
        let mut ext = self.build();
        ext.execute_with(test);
    }
}
