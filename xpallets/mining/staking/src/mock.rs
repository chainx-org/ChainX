use crate::*;
use crate::{Module, Trait};
use chainx_primitives::AssetId;
use frame_support::{impl_outer_event, impl_outer_origin, parameter_types, weights::Weight};
use sp_core::H256;
use sp_runtime::{
    testing::{Header, UintAuthorityId},
    traits::{BlakeTwo256, IdentityLookup},
    Perbill,
};
use std::{
    cell::RefCell,
    collections::{BTreeMap, HashSet},
};
use xp_mining_staking::SessionIndex;
use xpallet_assets::{AssetInfo, AssetRestriction, AssetRestrictions, Chain};

pub const INIT_TIMESTAMP: u64 = 30_000;

/// The AccountId alias in this test module.
pub(crate) type AccountId = u64;
pub(crate) type AccountIndex = u64;
pub(crate) type BlockNumber = u64;
pub(crate) type Balance = u128;

impl_outer_origin! {
    pub enum Origin for Test {}
}

mod staking {
    // Re-export needed for `impl_outer_event!`.
    pub use super::super::*;
}

use frame_system as system;
use pallet_session as session;
use xpallet_assets as assets;

impl_outer_event! {
    pub enum MetaEvent for Test {
        system<T>,
        assets<T>,
        session,
        staking<T>,
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
    type ModuleToIndex = ();
    type AccountData = ();
    type OnNewAccount = ();
    type OnKilledAccount = ();
}

parameter_types! {
    pub const MinimumPeriod: u64 = 5;
}

impl pallet_timestamp::Trait for Test {
    type Moment = u64;
    type OnTimestampSet = ();
    type MinimumPeriod = MinimumPeriod;
}

impl xpallet_assets::Trait for Test {
    type Balance = Balance;
    type Event = MetaEvent;
    type OnAssetChanged = ();
    type OnAssetRegisterOrRevoke = ();
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
}

pub struct DummyTreasuryAccount;

impl TreasuryAccount<AccountId> for DummyTreasuryAccount {
    fn treasury_account() -> AccountId {
        100_000
    }
}

impl Trait for Test {
    type Event = MetaEvent;
    type AssetMining = ();
    type TreasuryAccount = DummyTreasuryAccount;
    type DetermineRewardPotAccount = ();
}

// This function basically just builds a genesis storage key/value store according to
// our desired mockup.
pub fn new_test_ext() -> sp_io::TestExternalities {
    system::GenesisConfig::default()
        .build_storage::<Test>()
        .unwrap()
        .into()
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
    validator_count: u32,
    minimum_validator_count: u32,
}

impl Default for ExtBuilder {
    fn default() -> Self {
        Self {
            session_length: 1,
            election_lookahead: 0,
            session_per_era: 3,
            validator_count: 2,
            minimum_validator_count: 0,
        }
    }
}

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

        let pcx_asset = pcx();
        let assets = vec![(pcx_asset.0, pcx_asset.1, pcx_asset.2, true, true)];

        let mut endowed = BTreeMap::new();
        let pcx_id = pcx().0;
        let endowed_info = vec![(1, 100), (2, 200), (3, 300), (4, 400)];
        endowed.insert(pcx_id, endowed_info.clone());
        let _ = xpallet_assets::GenesisConfig::<Test> {
            assets,
            endowed,
            memo_len: 128,
        }
        .assimilate_storage(&mut storage);

        let validators = vec![1, 2, 3, 4];

        let _ = GenesisConfig::<Test> {
            validators: endowed_info,
            validator_count: 6,
            sessions_per_era: 3,
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
pub type XAssets = xpallet_assets::Module<Test>;
pub type Session = pallet_session::Module<Test>;
pub type Timestamp = pallet_timestamp::Module<Test>;
pub type XStaking = Module<Test>;
