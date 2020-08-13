use crate::*;
use crate::{Module, Trait};
use chainx_primitives::AssetId;
use frame_support::{
    assert_noop, assert_ok, impl_outer_event, impl_outer_origin, parameter_types, sp_io,
    weights::Weight,
};
use sp_core::H256;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
    Perbill,
};

use xpallet_protocol::X_BTC;

/// The AccountId alias in this test module.
pub(crate) type BlockNumber = u64;

impl_outer_origin! {
    pub enum Origin for Test {}
}

use frame_system as system;
mod xpallet_assets_metadata {
    // Re-export needed for `impl_outer_event!`.
    pub use super::super::*;
}

impl_outer_event! {
    pub enum MetaEvent for Test {
        system<T>,
        xpallet_assets_metadata<T>,
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
    type AccountData = ();
    type OnNewAccount = ();
    type OnKilledAccount = ();
}

parameter_types! {
    pub const ChainXAssetId: AssetId = 0;
}

impl Trait for Test {
    type Event = MetaEvent;
    type NativeAssetId = ChainXAssetId;
    type OnAssetRegisterOrRevoke = ();
}

pub struct ExtBuilder;
impl Default for ExtBuilder {
    fn default() -> Self {
        Self
    }
}

pub(crate) fn btc() -> (AssetId, AssetInfo) {
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
    )
}

impl ExtBuilder {
    pub fn build(self, assets: Vec<(AssetId, AssetInfo, bool, bool)>) -> sp_io::TestExternalities {
        let mut storage = frame_system::GenesisConfig::default()
            .build_storage::<Test>()
            .unwrap();

        let _ = GenesisConfig { assets }.assimilate_storage::<Test>(&mut storage);

        let ext = sp_io::TestExternalities::new(storage);
        ext
    }
    pub fn build_and_execute(self, test: impl FnOnce() -> ()) {
        let btc_assets = btc();
        let assets = vec![(btc_assets.0, btc_assets.1, true, true)];
        let mut ext = self.build(assets);
        ext.execute_with(|| System::set_block_number(1));
        ext.execute_with(test);
    }
}

pub type System = frame_system::Module<Test>;
pub type XAssetsMetadata = Module<Test>;
pub type Err = Error<Test>;

#[test]
fn test_register() {
    ExtBuilder::default().build_and_execute(|| {
        let abc_id = 100;
        let abc_assets = (
            abc_id,
            AssetInfo::new::<Test>(
                b"ABC".to_vec(),
                b"ABC".to_vec(),
                Chain::Bitcoin,
                8,
                b"abc".to_vec(),
            )
            .unwrap(),
        );
        assert_ok!(XAssetsMetadata::register_asset(
            Origin::root(),
            abc_assets.0,
            abc_assets.1.clone(),
            false,
            false
        ));
        assert_noop!(
            XAssetsMetadata::register_asset(
                Origin::root(),
                abc_assets.0,
                abc_assets.1,
                false,
                false
            ),
            Err::AlreadyExistentToken
        );

        assert_noop!(XAssetsMetadata::get_asset(&abc_id), Err::InvalidAsset);

        assert_ok!(XAssetsMetadata::recover_asset(Origin::root(), abc_id, true));
        assert!(XAssetsMetadata::get_asset(&abc_id).is_ok());

        assert_noop!(
            XAssetsMetadata::revoke_asset(Origin::root(), 10000),
            Err::InvalidAsset
        );
        assert_noop!(
            XAssetsMetadata::recover_asset(Origin::root(), X_BTC, true),
            Err::InvalidAsset
        );

        assert_ok!(XAssetsMetadata::revoke_asset(Origin::root(), X_BTC));
        assert_noop!(XAssetsMetadata::get_asset(&X_BTC), Err::InvalidAsset);
    })
}
