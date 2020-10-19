// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use std::{
    cell::RefCell,
    collections::{BTreeMap, HashSet},
};

use frame_support::{
    impl_outer_event, impl_outer_origin, parameter_types, traits::Get, weights::Weight,
};
use sp_core::H256;
use sp_runtime::{
    testing::{Header, UintAuthorityId},
    traits::{BlakeTwo256, IdentityLookup},
    Perbill,
};

use chainx_primitives::AssetId;
use xp_mining_staking::SessionIndex;
use xpallet_assets::{AssetInfo, AssetRestrictions, Chain};

use crate::*;
use crate::{Module, Trait};

pub const INIT_TIMESTAMP: u64 = 30_000;

/// The AccountId alias in this test module.
pub(crate) type AccountId = u64;
pub(crate) type BlockNumber = u64;
pub(crate) type Balance = u128;
pub(crate) type Amount = i128;

impl_outer_origin! {
    pub enum Origin for Test {}
}

mod mining_asset {
    // Re-export needed for `impl_outer_event!`.
    pub use super::super::*;
}

use frame_system as system;
use pallet_balances as balances;
use pallet_session as session;
use xpallet_assets as assets;
use xpallet_assets_registrar as assets_registrar;
use xpallet_mining_staking as staking;

impl_outer_event! {
    pub enum MetaEvent for Test {
        system<T>,
        balances<T>,
        session,
        assets_registrar,
        assets<T>,
        staking<T>,
        mining_asset<T>,
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

impl frame_system::Trait for Test {
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
    type PalletInfo = ();
    type AccountData = pallet_balances::AccountData<Balance>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
}

parameter_types! {
    pub const MinimumPeriod: u64 = 5;
}

impl pallet_timestamp::Trait for Test {
    type Moment = u64;
    type OnTimestampSet = ();
    type MinimumPeriod = MinimumPeriod;
    type WeightInfo = ();
}

pub struct ExistentialDeposit;
impl Get<Balance> for ExistentialDeposit {
    fn get() -> Balance {
        EXISTENTIAL_DEPOSIT.with(|v| *v.borrow())
    }
}

impl pallet_balances::Trait for Test {
    type MaxLocks = ();
    type Balance = Balance;
    type Event = MetaEvent;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
}

parameter_types! {
    pub const ChainXAssetId: AssetId = 0;
}
impl xpallet_assets_registrar::Trait for Test {
    type Event = MetaEvent;
    type NativeAssetId = ChainXAssetId;
    type RegistrarHandler = XMiningAsset;
    type WeightInfo = ();
}

impl xpallet_assets::Trait for Test {
    type Event = MetaEvent;
    type Currency = Balances;
    type Amount = Amount;
    type TreasuryAccount = ();
    type OnCreatedAccount = frame_system::CallOnCreatedAccount<Test>;
    type OnAssetChanged = XMiningAsset;
    type WeightInfo = ();
}

/// Another session handler struct to test on_disabled.
pub struct OtherSessionHandler;
impl pallet_session::OneSessionHandler<AccountId> for OtherSessionHandler {
    type Key = UintAuthorityId;

    fn on_genesis_session<'a, I: 'a>(_: I)
    where
        I: Iterator<Item = (&'a AccountId, Self::Key)>,
        AccountId: 'a,
    {
    }

    fn on_new_session<'a, I: 'a>(_: bool, validators: I, _: I)
    where
        I: Iterator<Item = (&'a AccountId, Self::Key)>,
        AccountId: 'a,
    {
        SESSION.with(|x| {
            *x.borrow_mut() = (validators.map(|x| x.0.clone()).collect(), HashSet::new())
        });
    }

    fn on_disabled(validator_index: usize) {
        SESSION.with(|d| {
            let mut d = d.borrow_mut();
            let value = d.0[validator_index];
            d.1.insert(value);
        })
    }
}

pub(crate) fn btc() -> (AssetId, AssetInfo, AssetRestrictions) {
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
        AssetRestrictions::empty(),
    )
}

impl sp_runtime::BoundToRuntimeAppPublic for OtherSessionHandler {
    type Public = UintAuthorityId;
}

pub struct Period;
impl Get<BlockNumber> for Period {
    fn get() -> BlockNumber {
        PERIOD.with(|v| *v.borrow())
    }
}

parameter_types! {
    pub const Offset: BlockNumber = 0;
    pub const UncleGenerations: u64 = 0;
    pub const DisabledValidatorsThreshold: Perbill = Perbill::from_percent(25);
}

sp_runtime::impl_opaque_keys! {
    pub struct SessionKeys {
        pub other: OtherSessionHandler,
    }
}

impl pallet_session::Trait for Test {
    type SessionManager = XStaking;
    type Keys = SessionKeys;
    type ShouldEndSession = pallet_session::PeriodicSessions<Period, Offset>;
    type SessionHandler = (OtherSessionHandler,);
    type Event = MetaEvent;
    type ValidatorId = AccountId;
    type ValidatorIdOf = ();
    type DisabledValidatorsThreshold = DisabledValidatorsThreshold;
    type NextSessionRotation = pallet_session::PeriodicSessions<Period, Offset>;
    type WeightInfo = ();
}

pub struct DummyTreasuryAccount;

pub(crate) const VESTING_ACCOUNT: AccountId = 10_000;
pub(crate) const TREASURY_ACCOUNT: AccountId = 100_000;

impl xpallet_support::traits::TreasuryAccount<AccountId> for DummyTreasuryAccount {
    fn treasury_account() -> AccountId {
        TREASURY_ACCOUNT
    }
}

parameter_types! {
    pub const SessionDuration: BlockNumber = 50;
    pub const MigrationSessionOffset: u32 = 500;
    pub const MinimumReferralId: u32 = 2;
    pub const MaximumReferralId: u32 = 12;
}

pub struct DummyStakingRewardPotAccountDeterminer;

impl xp_mining_common::RewardPotAccountFor<AccountId, AccountId>
    for DummyStakingRewardPotAccountDeterminer
{
    fn reward_pot_account_for(validator: &AccountId) -> AccountId {
        10_000_000 + u64::from(*validator)
    }
}

impl xpallet_mining_staking::Trait for Test {
    type Currency = Balances;
    type Event = MetaEvent;
    type AssetMining = XMiningAsset;
    type MigrationSessionOffset = MigrationSessionOffset;
    type SessionDuration = SessionDuration;
    type MinimumReferralId = MinimumReferralId;
    type MaximumReferralId = MaximumReferralId;
    type SessionInterface = Self;
    type TreasuryAccount = DummyTreasuryAccount;
    type DetermineRewardPotAccount = DummyStakingRewardPotAccountDeterminer;
    type WeightInfo = ();
}

pub struct DummyAssetRewardPotAccountDeterminer;

impl xp_mining_common::RewardPotAccountFor<AccountId, AssetId>
    for DummyAssetRewardPotAccountDeterminer
{
    fn reward_pot_account_for(asset_id: &AssetId) -> AccountId {
        1_000_000 + u64::from(*asset_id)
    }
}

pub struct DummyGatewayReferralGetter;

impl GatewayInterface<AccountId> for DummyGatewayReferralGetter {
    fn referral_of(who: &AccountId, _: AssetId) -> Option<AccountId> {
        Some(10_000_000_000 + *who)
    }
}

impl Trait for Test {
    type StakingInterface = Self;
    type GatewayInterface = DummyGatewayReferralGetter;
    type Event = MetaEvent;
    type TreasuryAccount = ();
    type DetermineRewardPotAccount = DummyAssetRewardPotAccountDeterminer;
    type WeightInfo = ();
}

thread_local! {
    static SESSION: RefCell<(Vec<AccountId>, HashSet<AccountId>)> = RefCell::new(Default::default());
    static SESSION_PER_ERA: RefCell<SessionIndex> = RefCell::new(3);
    static EXISTENTIAL_DEPOSIT: RefCell<Balance> = RefCell::new(0);
    static ELECTION_LOOKAHEAD: RefCell<BlockNumber> = RefCell::new(0);
    static PERIOD: RefCell<BlockNumber> = RefCell::new(1);
    static MAX_ITERATIONS: RefCell<u32> = RefCell::new(0);
}

pub struct ExtBuilder {
    session_length: BlockNumber,
    election_lookahead: BlockNumber,
    session_per_era: SessionIndex,
}

impl Default for ExtBuilder {
    fn default() -> Self {
        Self {
            session_length: 1,
            election_lookahead: 0,
            session_per_era: 3,
        }
    }
}

impl ExtBuilder {
    pub fn set_associated_constants(&self) {
        SESSION_PER_ERA.with(|v| *v.borrow_mut() = self.session_per_era);
        ELECTION_LOOKAHEAD.with(|v| *v.borrow_mut() = self.election_lookahead);
        PERIOD.with(|v| *v.borrow_mut() = self.session_length);
    }
    pub fn build(self) -> sp_io::TestExternalities {
        let _ = env_logger::try_init();
        self.set_associated_constants();
        let mut storage = frame_system::GenesisConfig::default()
            .build_storage::<Test>()
            .unwrap();

        let _ = pallet_balances::GenesisConfig::<Test> {
            balances: vec![(1, 100), (2, 200), (3, 300), (4, 400)],
        }
        .assimilate_storage(&mut storage);

        let _ = xpallet_assets_registrar::GenesisConfig { assets: vec![] }
            .assimilate_storage::<Test>(&mut storage);
        let _ = xpallet_assets::GenesisConfig::<Test> {
            assets_restrictions: vec![],
            endowed: BTreeMap::new(),
        }
        .assimilate_storage(&mut storage);

        let validators = vec![1, 2, 3, 4];

        let _ = xpallet_mining_staking::GenesisConfig::<Test> {
            validators: vec![
                (1, b"1 ".to_vec(), 10),
                (2, b"2 ".to_vec(), 20),
                (3, b"3 ".to_vec(), 30),
                (4, b"4 ".to_vec(), 40),
            ],
            validator_count: 6,
            sessions_per_era: 3,
            vesting_account: VESTING_ACCOUNT,
            glob_dist_ratio: (12, 88),
            mining_ratio: (10, 90),
            offence_severity: 2,
            ..Default::default()
        }
        .assimilate_storage(&mut storage);

        let _ = pallet_session::GenesisConfig::<Test> {
            keys: validators
                .iter()
                .map(|x| {
                    (
                        *x,
                        *x,
                        SessionKeys {
                            other: UintAuthorityId(*x as u64),
                        },
                    )
                })
                .collect(),
        }
        .assimilate_storage(&mut storage);

        let _ = GenesisConfig::<Test> {
            claim_restrictions: vec![(xp_protocol::X_BTC, (7, 3))],
            mining_power_map: vec![(xp_protocol::X_BTC, 400)],
        }
        .assimilate_storage(&mut storage);

        let mut ext = sp_io::TestExternalities::from(storage);
        ext.execute_with(|| {
            let validators = Session::validators();
            SESSION.with(|x| *x.borrow_mut() = (validators.clone(), HashSet::new()));
        });

        // We consider all test to start after timestamp is initialized
        // This must be ensured by having `timestamp::on_initialize` called before
        // `staking::on_initialize`
        ext.execute_with(|| {
            System::set_block_number(1);
            Timestamp::set_timestamp(INIT_TIMESTAMP);
        });

        ext
    }
    pub fn build_and_execute(self, test: impl FnOnce() -> ()) {
        let mut ext = self.build();
        ext.execute_with(test);
        // ext.execute_with(post_conditions);
    }
}

pub type System = frame_system::Module<Test>;
pub type Balances = pallet_balances::Module<Test>;
pub type XAssetsRegistrar = xpallet_assets_registrar::Module<Test>;
pub type XAssets = xpallet_assets::Module<Test>;
pub type Session = pallet_session::Module<Test>;
pub type Timestamp = pallet_timestamp::Module<Test>;
pub type XStaking = xpallet_mining_staking::Module<Test>;
pub type XMiningAsset = Module<Test>;
