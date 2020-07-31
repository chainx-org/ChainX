use crate::*;
use crate::{Module, Trait};
use chainx_primitives::AssetId;
use frame_support::{impl_outer_event, impl_outer_origin, parameter_types, sp_io, weights::Weight};
use sp_core::H256;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
    Perbill,
};
use std::collections::BTreeMap;

/// The AccountId alias in this test module.
pub(crate) type AccountId = u64;
pub(crate) type BlockNumber = u64;
pub(crate) type Balance = u128;

impl_outer_origin! {
    pub enum Origin for Test {}
}

use frame_system as system;
mod assets {
    // Re-export needed for `impl_outer_event!`.
    pub use super::super::*;
}

impl_outer_event! {
    pub enum MetaEvent for Test {
        system<T>,
        pallet_balances<T>,
        assets<T>,
    }
}

// For testing the pallet, we construct most of a mock runtime. This means
// first constructing a configuration type (`Test`) which `impl`s each of the
// configuration traits of pallets we want to use.
#[derive(Clone, Eq, PartialEq, Debug)]
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
    type BlockNumber = BlockNumber;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = u64;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type Event = MetaEvent;
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
}
parameter_types! {
    pub const ExistentialDeposit: u64 = 1;
}
impl pallet_balances::Trait for Test {
    type Balance = Balance;
    type DustRemoval = ();
    type Event = MetaEvent;
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
}

impl Trait for Test {
    type Event = MetaEvent;
    type Currency = Balances;
    type OnAssetChanged = ();
    type OnAssetRegisterOrRevoke = ();
}

pub struct ExtBuilder;
impl Default for ExtBuilder {
    fn default() -> Self {
        Self
    }
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
        AssetRestriction::DestroyFree.into(),
    )
}

impl ExtBuilder {
    pub fn build(
        self,
        assets: Vec<(AssetId, AssetInfo, AssetRestrictions, bool, bool)>,
        endowed: BTreeMap<AssetId, Vec<(AccountId, Balance)>>,
    ) -> sp_io::TestExternalities {
        let _ = env_logger::try_init();
        let mut storage = frame_system::GenesisConfig::default()
            .build_storage::<Test>()
            .unwrap();

        let _ = GenesisConfig::<Test> {
            assets,
            endowed,
            memo_len: 128,
        }
        .assimilate_storage(&mut storage);

        let ext = sp_io::TestExternalities::new(storage);
        ext
    }
    pub fn build_and_execute(self, test: impl FnOnce() -> ()) {
        let btc_assets = btc();
        let assets = vec![(btc_assets.0, btc_assets.1, btc_assets.2, true, true)];
        let mut endowed = BTreeMap::new();
        let endowed_info = vec![(1, 100), (2, 200), (3, 300), (4, 400)];
        endowed.insert(btc_assets.0, endowed_info);

        let mut ext = self.build(assets, endowed);
        ext.execute_with(|| System::set_block_number(1));
        ext.execute_with(test);
    }

    pub fn build_no_endowed_and_execute(self, test: impl FnOnce() -> ()) {
        let btc_assets = btc();
        let assets = vec![(btc_assets.0, btc_assets.1, btc_assets.2, true, true)];
        let mut ext = self.build(assets, Default::default());
        ext.execute_with(|| System::set_block_number(1));
        ext.execute_with(test);
    }
}

pub type System = frame_system::Module<Test>;
pub type Balances = pallet_balances::Module<Test>;
pub type XAssets = Module<Test>;
pub type XAssetsErr = Error<Test>;
