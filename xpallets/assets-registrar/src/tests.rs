// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use frame_support::{
    assert_noop, assert_ok, impl_outer_event, impl_outer_origin, parameter_types, sp_io,
};
use sp_core::H256;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
};

use chainx_primitives::AssetId;
use xp_protocol::X_BTC;

use crate::*;

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
        xpallet_assets_metadata,
    }
}

// For testing the pallet, we construct most of a mock runtime. This means
// first constructing a configuration type (`Test`) which `impl`s each of the
// configuration traits of pallets we want to use.
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct Test;

parameter_types! {
    pub const BlockHashCount: u64 = 250;
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
    type AccountId = u64;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type Event = MetaEvent;
    type BlockHashCount = BlockHashCount;
    type DbWeight = ();
    type Version = ();
    type PalletInfo = ();
    type AccountData = ();
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = SS58Prefix;
}

parameter_types! {
    pub const ChainXAssetId: AssetId = 0;
}

impl Config for Test {
    type Event = MetaEvent;
    type NativeAssetId = ChainXAssetId;
    type RegistrarHandler = ();
    type WeightInfo = ();
}

pub struct ExtBuilder;
impl Default for ExtBuilder {
    fn default() -> Self {
        Self
    }
}

pub(crate) fn btc() -> (AssetId, AssetInfo) {
    (
        xp_protocol::X_BTC,
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
pub type XAssetsRegistrar = Module<Test>;
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
        assert_ok!(XAssetsRegistrar::register(
            Origin::root(),
            abc_assets.0,
            abc_assets.1.clone(),
            false,
            false
        ));
        assert_noop!(
            XAssetsRegistrar::register(Origin::root(), abc_assets.0, abc_assets.1, false, false),
            Err::AssetAlreadyExists
        );

        assert_noop!(
            XAssetsRegistrar::get_asset_info(&abc_id),
            Err::AssetIsInvalid
        );

        assert_ok!(XAssetsRegistrar::recover(Origin::root(), abc_id, true));
        assert!(XAssetsRegistrar::get_asset_info(&abc_id).is_ok());

        assert_noop!(
            XAssetsRegistrar::deregister(Origin::root(), 10000),
            Err::AssetIsInvalid
        );
        assert_noop!(
            XAssetsRegistrar::recover(Origin::root(), X_BTC, true),
            Err::AssetAlreadyValid
        );

        assert_ok!(XAssetsRegistrar::deregister(Origin::root(), X_BTC));
        assert_noop!(
            XAssetsRegistrar::get_asset_info(&X_BTC),
            Err::AssetIsInvalid
        );
    })
}
