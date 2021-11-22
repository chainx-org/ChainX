// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use std::{cell::RefCell, collections::HashSet};

use frame_support::{parameter_types, traits::GenesisBuild};
use sp_core::H256;
use sp_runtime::{
    testing::{Header, UintAuthorityId},
    traits::{BlakeTwo256, IdentityLookup},
    Perbill,
};
use xp_mining_staking::SessionIndex;

use crate::Config;
use crate::{self as xpallet_mining_staking, *};

pub const INIT_TIMESTAMP: u64 = 30_000;

pub(crate) const TREASURY_ACCOUNT: AccountId = 100_000;

/// The AccountId alias in this test module.
pub(crate) type AccountId = u64;
pub(crate) type AccountIndex = u64;
pub(crate) type BlockNumber = u64;
pub(crate) type Balance = u128;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
    pub enum Test where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
        Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent},
        Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
        Session: pallet_session::{Pallet, Call, Storage, Event, Config<T>},
        XStaking: xpallet_mining_staking::{Pallet, Call, Storage, Event<T>, Config<T>},
    }
);

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const SS58Prefix: u8 = 42;
}

impl frame_system::Config for Test {
    type BaseCallFilter = frame_support::traits::Everything;
    type BlockWeights = ();
    type BlockLength = ();
    type Origin = Origin;
    type Call = Call;
    type Index = AccountIndex;
    type BlockNumber = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type Event = Event;
    type BlockHashCount = BlockHashCount;
    type DbWeight = ();
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<Balance>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = SS58Prefix;
    type OnSetCode = ();
}

pub struct ExistentialDeposit;
impl Get<Balance> for ExistentialDeposit {
    fn get() -> Balance {
        EXISTENTIAL_DEPOSIT.with(|v| *v.borrow())
    }
}

parameter_types! {
    pub const MaxReserves: u32 = 50;
}

impl pallet_balances::Config for Test {
    type MaxLocks = ();
    type Balance = Balance;
    type Event = Event;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
    type ReserveIdentifier = [u8; 8];
    type MaxReserves = MaxReserves;
}

parameter_types! {
    pub const MinimumPeriod: u64 = 5;
}

impl pallet_timestamp::Config for Test {
    type Moment = u64;
    type OnTimestampSet = ();
    type MinimumPeriod = MinimumPeriod;
    type WeightInfo = ();
}

/// Another session handler struct to test on_disabled.
pub struct OtherSessionHandler;
impl frame_support::traits::OneSessionHandler<AccountId> for OtherSessionHandler {
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

impl pallet_session::Config for Test {
    type SessionManager = XStaking;
    type Keys = SessionKeys;
    type ShouldEndSession = pallet_session::PeriodicSessions<Period, Offset>;
    type SessionHandler = (OtherSessionHandler,);
    type Event = Event;
    type ValidatorId = AccountId;
    type ValidatorIdOf = ();
    type DisabledValidatorsThreshold = DisabledValidatorsThreshold;
    type NextSessionRotation = pallet_session::PeriodicSessions<Period, Offset>;
    type WeightInfo = ();
}

pub struct DummyTreasuryAccount;

impl TreasuryAccount<AccountId> for DummyTreasuryAccount {
    fn treasury_account() -> AccountId {
        TREASURY_ACCOUNT
    }
}

pub struct DummyStakingRewardPotAccountDeterminer;

impl xp_mining_common::RewardPotAccountFor<AccountId, AccountId>
    for DummyStakingRewardPotAccountDeterminer
{
    fn reward_pot_account_for(validator: &AccountId) -> AccountId {
        10_000_000 + u64::from(*validator)
    }
}

parameter_types! {
    pub const SessionDuration: BlockNumber = 50;
    pub const MinimumReferralId: u32 = 2;
    pub const MaximumReferralId: u32 = 12;
}

impl Config for Test {
    type Currency = Balances;
    type Event = Event;
    type AssetMining = ();
    type SessionDuration = SessionDuration;
    type MinimumReferralId = MinimumReferralId;
    type MaximumReferralId = MaximumReferralId;
    type SessionInterface = Self;
    type TreasuryAccount = DummyTreasuryAccount;
    type DetermineRewardPotAccount = DummyStakingRewardPotAccountDeterminer;
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
            glob_dist_ratio: (12, 88),
            mining_ratio: (10, 90),
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
            let _ = t_register(1, 10);
            let _ = t_register(2, 20);
            let _ = t_register(3, 30);
            let _ = t_register(4, 40);
            let validators = Session::validators();
            SESSION.with(|x| *x.borrow_mut() = (validators.clone(), HashSet::new()));
        });

        // We consider all test to start after timestamp is initialized
        // This must be ensured by having `timestamp::on_initialize` called before
        // `staking::on_initialize`
        ext.execute_with(|| {
            System::set_block_number(1);
            Timestamp::set_timestamp(INIT_TIMESTAMP);
            // Just ignore the immortals for tests.
            XStaking::set_immortals(Origin::root(), vec![]).unwrap();
        });

        ext
    }
    pub fn build_and_execute(self, test: impl FnOnce() -> ()) {
        let mut ext = self.build();
        ext.execute_with(test);
    }
}

pub fn t_register(who: AccountId, initial_bond: Balance) -> DispatchResult {
    let mut referral_id = who.to_string().as_bytes().to_vec();

    if referral_id.len() < 2 {
        referral_id.extend_from_slice(&[0, 0, 0, who as u8]);
    }

    XStaking::register(Origin::signed(who), referral_id, initial_bond)
}
