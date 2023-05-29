// Copyright 2019-2023 ChainX Project Authors. Licensed under GPL-3.0.

use frame_support::{
    construct_runtime, parameter_types, traits::{ConstU32, ConstU64}, PalletId
};
use frame_system::EnsureRoot;
use sp_core::H256;
use sp_runtime::{
    testing::Header, traits::{BlakeTwo256, IdentityLookup}, AccountId32
};

/// The AccountId alias in this test module.
pub(crate) type AccountId = AccountId32;
pub(crate) type BlockNumber = u64;
pub(crate) type Balance = u128;
pub(crate) use crate as btc_ledger;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

construct_runtime!(
    pub enum Test where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
        BtcLedger: btc_ledger::{Pallet, Call, Storage, Config<T>, Event<T>}
    }
);

parameter_types! {
    pub BlockWeights: frame_system::limits::BlockWeights =
        frame_system::limits::BlockWeights::simple_max(1024);
    pub const BtcLedgerPalletId: PalletId = PalletId(*b"pcx/trsy");
}
impl frame_system::Config for Test {
    type BaseCallFilter = frame_support::traits::Everything;
    type BlockWeights = BlockWeights;
    type BlockLength = ();
    type DbWeight = ();
    type Origin = Origin;
    type Index = u64;
    type BlockNumber = BlockNumber;
    type Call = Call;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type Event = Event;
    type BlockHashCount = ConstU64<250>;
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = ();
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = ();
    type OnSetCode = ();
    type MaxConsumers = ConstU32<16>;
}

impl crate::Config for Test {
    type Balance = Balance;
    type Event = Event;
    type CouncilOrigin = EnsureRoot<AccountId>;
    type PalletId = BtcLedgerPalletId;
}

pub const ALICE: [u8; 32] = [1u8; 32];
pub const BOB: [u8; 32] = [2u8; 32];
pub const CHARLIE: [u8; 32] = [3u8; 32];

pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = frame_system::GenesisConfig::default()
        .build_storage::<Test>()
        .unwrap();

    btc_ledger::GenesisConfig::<Test> {
        balances: vec![(ALICE.into(), 10), (BOB.into(), 20)]
    }
        .assimilate_storage(&mut t)
        .unwrap();

    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| {
        System::inc_providers(&ALICE.into());
        System::inc_providers(&BOB.into());
        System::set_block_number(1)
    });

    ext
}