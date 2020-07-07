use crate::*;

use std::{
    cell::RefCell,
    collections::{BTreeMap, HashSet},
};

use frame_support::{impl_outer_origin, parameter_types, weights::Weight};
use frame_system as system;
use sp_core::H256;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
    Perbill,
};

use chainx_primitives::AssetId;
use xpallet_assets::{AssetInfo, AssetRestriction, AssetRestrictions, Chain};

/// The AccountId alias in this test module.
pub(crate) type AccountId = u64;
pub(crate) type AccountIndex = u64;
pub(crate) type BlockNumber = u64;
pub(crate) type Balance = u128;
pub(crate) type Price = u64;

pub(crate) type SessionIndex = u64;

impl_outer_origin! {
    pub enum Origin for Test {}
}

// For testing the pallet, we construct most of a mock runtime. This means
// first constructing a configuration type (`Test`) which `impl`s each of the
// configuration traits of pallets we want to use.
#[derive(Clone, Eq, PartialEq)]
pub struct Test;

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const MaximumBlockWeight: Weight = 1024;
    pub const MaximumBlockLength: u32 = 2 * 1024;
    pub const AvailableBlockRatio: Perbill = Perbill::from_percent(75);
}

impl system::Trait for Test {
    type BaseCallFilter = ();
    type Origin = Origin;
    type Call = ();
    type Index = u64;
    type BlockNumber = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = u64;
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
    type AccountData = ();
    type OnNewAccount = ();
    type OnKilledAccount = ();
}

impl Trait for Test {
    type Event = ();
    type Price = Price;
}

impl xpallet_assets::Trait for Test {
    type Balance = Balance;
    type Event = ();
    type OnAssetChanged = ();
    type OnAssetRegisterOrRevoke = XSpot;
    type DetermineTokenJackpotAccountId = ();
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

const PCX_PRECISION: u8 = 8;

fn pcx() -> (AssetId, AssetInfo, AssetRestrictions) {
    (
        xpallet_protocol::PCX,
        AssetInfo::new::<Test>(
            b"PCX".to_vec(),
            b"Polkadot ChainX".to_vec(),
            Chain::ChainX,
            PCX_PRECISION,
            b"ChainX's crypto currency in Polkadot ecology".to_vec(),
        )
        .unwrap(),
        AssetRestriction::Deposit
            | AssetRestriction::Withdraw
            | AssetRestriction::DestroyWithdrawal
            | AssetRestriction::DestroyFree,
    )
}

pub(crate) fn btc() -> (AssetId, AssetInfo, AssetRestrictions) {
    (
        xpallet_protocol::X_BTC,
        AssetInfo::new::<Test>(
            b"X-BTC".to_vec(),
            b"X-BTC".to_vec(),
            Chain::Bitcoin,
            8,
            b"ChainX's cross-chain Bitcoin".to_vec(),
        )
        .unwrap(),
        AssetRestriction::Deposit
            | AssetRestriction::Withdraw
            | AssetRestriction::DestroyWithdrawal
            | AssetRestriction::DestroyFree,
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

        let mut endowed = BTreeMap::new();
        let pcx_id = pcx().0;
        let endowed_info = vec![];
        endowed.insert(pcx_id, endowed_info);
        let _ = xpallet_assets::GenesisConfig::<Test> {
            assets,
            endowed,
            memo_len: 128,
        }
        .assimilate_storage(&mut storage);

        let trading_pairs = vec![
            (
                xpallet_protocol::PCX,
                xpallet_protocol::X_BTC,
                9,
                2,
                100000,
                true,
            ),
            (
                xpallet_protocol::X_DOT,
                xpallet_protocol::PCX,
                4,
                2,
                100000,
                true,
            ),
        ];

        let _ = GenesisConfig::<Test> {
            trading_pairs,
            ..Default::default()
        }
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
    pub fn build_and_execute(self, test: impl FnOnce() -> ()) {
        let mut ext = self.build();
        ext.execute_with(test);
    }
}

pub type System = frame_system::Module<Test>;
pub type XAssets = xpallet_assets::Module<Test>;
pub type XSpot = Module<Test>;
