use substrate_primitives::{H256, Blake2Hasher};

use primitives::BuildStorage;
use primitives::traits::BlakeTwo256;
use primitives::testing::{Digest, DigestItem, Header};
use runtime_io;
use runtime_io::with_externalities;

use super::*;


impl_outer_origin! {
        pub enum Origin for Test {}
    }

#[derive(Clone, Eq, PartialEq)]
pub struct Test;

impl system::Trait for Test {
    type Origin = Origin;
    type Index = u64;
    type BlockNumber = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type Digest = Digest;
    type AccountId = u64;
    type Header = Header;
    type Event = ();
    type Log = DigestItem;
}

impl balances::Trait for Test {
    type Balance = u64;
    type AccountIndex = u64;
    type OnFreeBalanceZero = ();
    type EnsureAccountLiquid = ();
    type Event = ();
}

impl cxsystem::Trait for Test {}

impl associations::Trait for Test {
    type OnCalcFee = CXSupport;
    type Event = ();
}

impl Trait for Test {}

type Balances = balances::Module<Test>;
type CXSystem = cxsystem::Module<Test>;
type CXSupport = Module<Test>;

pub fn new_test_ext() -> runtime_io::TestExternalities<Blake2Hasher> {
    let mut r = system::GenesisConfig::<Test>::default().build_storage().unwrap();
    // balances
    r.extend(balances::GenesisConfig::<Test> {
        balances: vec![(1, 1000), (2, 510), (3, 1000)],
        transaction_base_fee: 0,
        transaction_byte_fee: 0,
        existential_deposit: 0,
        transfer_fee: 0,
        creation_fee: 0,
        reclaim_rebate: 0,
    }.build_storage().unwrap());
    // cxsystem
    r.extend(cxsystem::GenesisConfig::<Test> {
        death_account: 100,
    }.build_storage().unwrap());

    r.into()
}

#[test]
fn test_no_relation_no_producer() {
    with_externalities(&mut new_test_ext(), || {
        assert_ok!(CXSupport::handle_fee_before(&1, 100, true, || Ok(())));

        assert_eq!(Balances::free_balance(CXSystem::death_account()), 100);
    })
}

#[test]
fn test_no_relation_with_producer() {
    with_externalities(&mut new_test_ext(), || {
        let origin = system::RawOrigin::Inherent.into();
        CXSystem::set_block_producer(origin, 5).unwrap();

        assert_ok!(CXSupport::handle_fee_before(&1, 99, true, || Ok(())));
        assert_eq!(Balances::free_balance(5), 99);
    })
}

#[test]
fn test_with_relation_with_producer() {
    with_externalities(&mut new_test_ext(), || {
        use runtime_support::StorageMap;

        let origin = system::RawOrigin::Inherent.into();
        CXSystem::set_block_producer(origin, 5).unwrap();

        let origin = system::RawOrigin::Signed(1).into();
        assert_ok!(associations::Module::<Test>::init_account(origin, 10, 100));
        assert_eq!(balances::FreeBalance::<Test>::exists(10), true);

        assert_ok!(CXSupport::handle_fee_before(&10, 99, true, || Ok(())));

        assert_eq!(Balances::free_balance(1), 1000 - 100 + 49);
        assert_eq!(Balances::free_balance(5), 50); // block producer
        assert_eq!(Balances::free_balance(10), 100 - 99);
    })
}
