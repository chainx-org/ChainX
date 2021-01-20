use std::{cell::RefCell, collections::BTreeMap, ops::AddAssign, time::Duration};

use frame_support::{impl_outer_origin, parameter_types, sp_io, traits::GenesisBuild};
use sp_core::H256;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
};

use super::vault::pallet as vault;

#[derive(Clone, PartialEq, Eq)]
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
    type AccountId = u64;
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

impl vault::Config for Test {
    type PCX = Balances;
    type Event = ();
}

pub(crate) type System = frame_system::Module<Test>;
pub(crate) type Balances = pallet_balances::Module<Test>;

pub struct ExtBuilder;
impl ExtBuilder {
    pub fn build(minimium_vault_collateral: u32) -> sp_io::TestExternalities {
        let mut storage = frame_system::GenesisConfig::default()
            .build_storage::<Test>()
            .unwrap();
        let _ = pallet_balances::GenesisConfig::<Test> {
            balances: vec![(1, 1000), (2, 2000)],
        }
        .assimilate_storage(&mut storage);

        let _ = GenesisBuild::<Test>::assimilate_storage(
            &vault::GenesisConfig {
                minimium_vault_collateral,
            },
            &mut storage,
        );
        sp_io::TestExternalities::from(storage)
    }
}
