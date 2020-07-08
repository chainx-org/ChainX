use crate::*;
use crate::{Module, Trait};
use frame_support::{impl_outer_origin, parameter_types, weights::Weight};
use frame_system as system;
use sp_core::H256;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
    Perbill,
};
use std::{
    cell::RefCell,
    collections::{BTreeMap, HashSet},
};
use xp_mining_staking::SessionIndex;

/// The AccountId alias in this test module.
pub(crate) type AccountId = u64;
pub(crate) type AccountIndex = u64;
pub(crate) type BlockNumber = u64;
pub(crate) type Balance = u128;

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
}

impl xpallet_assets::Trait for Test {
    type Balance = Balance;
    type Event = ();
    type OnAssetChanged = XMiningAsset;
    type OnAssetRegisterOrRevoke = XMiningAsset;
}

impl xpallet_mining_staking::Trait for Test {
    type Event = ();
    type OnMinting = ();
    type CollectAssetMiningInfo = ();
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
    existential_deposit: Balance,
    validator_pool: bool,
    nominate: bool,
    validator_count: u32,
    minimum_validator_count: u32,
    fair: bool,
    num_validators: Option<u32>,
    has_stakers: bool,
    max_offchain_iterations: u32,
}

impl Default for ExtBuilder {
    fn default() -> Self {
        Self {
            session_length: 1,
            election_lookahead: 0,
            session_per_era: 3,
            existential_deposit: 1,
            validator_pool: false,
            nominate: true,
            validator_count: 2,
            minimum_validator_count: 0,
            fair: true,
            num_validators: None,
            has_stakers: true,
            max_offchain_iterations: 0,
        }
    }
}

use chainx_primitives::AssetId;
use xpallet_assets::{AssetInfo, AssetRestriction, AssetRestrictions, Chain};

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
        AssetRestrictions::none(),
    )
}

impl ExtBuilder {
    pub fn build(self) -> sp_io::TestExternalities {
        let _ = env_logger::try_init();
        let mut storage = frame_system::GenesisConfig::default()
            .build_storage::<Test>()
            .unwrap();
        let balance_factor = if self.existential_deposit > 1 { 256 } else { 1 };

        let pcx_asset = pcx();
        let assets = vec![(pcx_asset.0, pcx_asset.1, pcx_asset.2, true, false)];

        let mut endowed = BTreeMap::new();
        let pcx_id = pcx().0;
        let endowed_info = vec![(1, 100), (2, 200), (3, 300), (4, 400)];
        endowed.insert(pcx_id, endowed_info);
        let _ = xpallet_assets::GenesisConfig::<Test> {
            assets,
            endowed,
            memo_len: 128,
        }
        .assimilate_storage(&mut storage);

        let mut ext = sp_io::TestExternalities::from(storage);
        // ext.execute_with(|| {
        // let validators = Session::validators();
        // SESSION.with(|x| *x.borrow_mut() = (validators.clone(), HashSet::new()));
        // });

        // We consider all test to start after timestamp is initialized
        // This must be ensured by having `timestamp::on_initialize` called before
        // `staking::on_initialize`
        ext.execute_with(|| {
            System::set_block_number(1);
            // Timestamp::set_timestamp(INIT_TIMESTAMP);
            XStaking::register(Origin::signed(1)).unwrap();
            XStaking::register(Origin::signed(2)).unwrap();
            XStaking::register(Origin::signed(3)).unwrap();
            XStaking::register(Origin::signed(4)).unwrap();
            XStaking::bond(Origin::signed(1), 1, 10, b"memo".to_vec().into()).unwrap();
            XStaking::bond(Origin::signed(2), 2, 20, b"memo".to_vec().into()).unwrap();
            XStaking::bond(Origin::signed(3), 3, 30, b"memo".to_vec().into()).unwrap();
            XStaking::bond(Origin::signed(4), 4, 40, b"memo".to_vec().into()).unwrap();
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
pub type XStaking = xpallet_mining_staking::Module<Test>;
// pub type Session = pallet_session::Module<Test>;
// pub type Timestamp = pallet_timestamp::Module<Test>;
pub type XMiningAsset = Module<Test>;
