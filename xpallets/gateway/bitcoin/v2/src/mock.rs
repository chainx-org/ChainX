use frame_support::{impl_outer_origin, parameter_types, sp_io, traits::GenesisBuild};
use sp_core::H256;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
};

use super::assets::pallet as assets;
use super::vault::pallet as vault;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Test;

/// The AccountId alias in this test module.
pub(crate) type AccountId = u64;
pub(crate) type BlockNumber = u64;
pub(crate) type Amount = i128;
pub(crate) type Balance = u128;

impl_outer_origin! {
    pub enum Origin for Test where system = frame_system {}
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
    type AccountId = AccountId;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type Event = ();
    type BlockHashCount = ();
    type DbWeight = ();
    type Version = ();
    type PalletInfo = ();
    type AccountData = pallet_balances::AccountData<Balance>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = ();
}
parameter_types! {
    pub const ExistentialDeposit: u64 = 0;
    pub const ChainXAssetId: u32 = 0;
}

impl pallet_balances::Config for Test {
    type MaxLocks = ();
    type Balance = Balance;
    type DustRemoval = ();
    type Event = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
}

impl xpallet_assets_registrar::Config for Test {
    type Event = ();
    type NativeAssetId = ChainXAssetId;
    type RegistrarHandler = ();
    type WeightInfo = ();
}

impl xpallet_assets::Config for Test {
    type Event = ();
    type Currency = Balances;
    type Amount = Amount;
    type TreasuryAccount = ();
    type OnCreatedAccount = frame_system::CallOnCreatedAccount<Test>;
    type OnAssetChanged = ();
    type WeightInfo = ();
}

impl assets::Config for Test {}

impl vault::Config for Test {
    type Event = ();
}

pub(crate) type System = frame_system::Pallet<Test>;
pub(crate) type Balances = pallet_balances::Module<Test>;

pub struct ExtBuilder;
impl ExtBuilder {
    pub fn build(minimium_vault_collateral: u32) -> sp_io::TestExternalities {
        use super::assets::types::ExchangeRate;
        let mut storage = frame_system::GenesisConfig::default()
            .build_storage::<Test>()
            .unwrap();

        let _ = GenesisBuild::<Test>::assimilate_storage(
            &assets::GenesisConfig {
                exchange_rate: ExchangeRate {
                    price: 123123123,
                    decimal: 6,
                },
            },
            &mut storage,
        );

        let _ = pallet_balances::GenesisConfig::<Test> {
            balances: vec![(1, 1000), (2, 2000)],
        }
        .assimilate_storage(&mut storage);

        let _ = GenesisBuild::<Test>::assimilate_storage(
            &vault::GenesisConfig {
                minimium_vault_collateral,
                secure_threshold: 300,
                premium_threshold: 250,
                liquidation_threshold: 180,
            },
            &mut storage,
        );
        sp_io::TestExternalities::from(storage)
    }
}
