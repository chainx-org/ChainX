// Copyright 2018 Chainpool.

use substrate_primitives::{Blake2Hasher, H256};

use runtime_io;
use runtime_io::with_externalities;
use runtime_primitives::testing::{Digest, DigestItem, Header};
use runtime_primitives::traits::BlakeTwo256;
use runtime_primitives::BuildStorage;

use super::transaction::*;
use super::*;

impl_outer_origin! {
    pub enum Origin for Test {}
}

pub type AccountId = H256;

#[derive(Clone, Eq, PartialEq)]
pub struct Test;

impl system::Trait for Test {
    type Origin = Origin;
    type Index = u64;
    type BlockNumber = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type Digest = Digest;
    type AccountId = AccountId;
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
    type OnCalcFee = cxsupport::Module<Test>;
    type Event = ();
}

impl cxsupport::Trait for Test {}

impl Trait for Test {
    type MultiSig = SimpleMultiSigIdFor<Test>;
    type Event = ();
}

type Balances = balances::Module<Test>;
type MultiSig = Module<Test>;

pub fn new_test_ext() -> runtime_io::TestExternalities<Blake2Hasher> {
    let mut r = system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap();

    let b_config = balances::GenesisConfig::<Test> {
        balances: vec![
            (1.into(), 10000),
            (2.into(), 510),
            (3.into(), 1000),
            (4.into(), 500),
            (5.into(), 100),
            (6.into(), 100),
        ],
        transaction_base_fee: 0,
        transaction_byte_fee: 0,
        existential_deposit: 0, // better set 0
        transfer_fee: 10,
        creation_fee: 10,
        reclaim_rebate: 0,
    };
    let b_config_copy = BalancesConfigCopy::create_from_src(&b_config).src();

    // balance
    r.extend(b_config.build_storage().unwrap());
    // financialrecords
    r.extend(
        GenesisConfig::<Test> {
            genesis_multi_sig: vec![
                (
                    1.into(),
                    vec![
                        (1.into(), true),
                        (2.into(), true),
                        (3.into(), true),
                        (4.into(), false),
                        (5.into(), false),
                    ],
                    3,
                    1000,
                ),
                (1.into(), vec![(2.into(), true), (3.into(), false)], 3, 300),
                (2.into(), vec![(1.into(), true), (3.into(), false)], 2, 100),
                (3.into(), vec![(1.into(), true), (3.into(), false)], 2, 100),
                (
                    4.into(),
                    vec![
                        (1.into(), true),
                        (2.into(), true),
                        (3.into(), false),
                        (100.into(), false),
                    ],
                    1,
                    100,
                ),
            ],
            deploy_fee: 10,
            exec_fee: 10,
            confirm_fee: 10,
            balances_config: b_config_copy,
        }
        .build_storage()
        .unwrap(),
    );
    r.into()
}

pub fn err_test_ext() -> runtime_io::TestExternalities<Blake2Hasher> {
    let mut r = system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap();

    let b_config = balances::GenesisConfig::<Test> {
        balances: vec![(1.into(), 100)],
        transaction_base_fee: 0,
        transaction_byte_fee: 0,
        existential_deposit: 0, // better set 0
        transfer_fee: 10,
        creation_fee: 10,
        reclaim_rebate: 0,
    };
    let b_config_copy = BalancesConfigCopy::create_from_src(&b_config).src();

    // balance
    r.extend(b_config.build_storage().unwrap());
    // financialrecords
    r.extend(
        GenesisConfig::<Test> {
            genesis_multi_sig: vec![(
                1.into(),
                vec![
                    (1.into(), true),
                    (2.into(), true),
                    (3.into(), true),
                    (4.into(), false),
                    (5.into(), false),
                ],
                3,
                1000,
            )],
            deploy_fee: 10,
            exec_fee: 10,
            confirm_fee: 10,
            balances_config: b_config_copy,
        }
        .build_storage()
        .unwrap(),
    );
    r.into()
}

pub fn err_test_ext2() -> runtime_io::TestExternalities<Blake2Hasher> {
    let mut r = system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap();

    let b_config = balances::GenesisConfig::<Test> {
        balances: vec![],
        transaction_base_fee: 0,
        transaction_byte_fee: 0,
        existential_deposit: 0, // better set 0
        transfer_fee: 10,
        creation_fee: 10,
        reclaim_rebate: 0,
    };
    let b_config_copy = BalancesConfigCopy::create_from_src(&b_config).src();

    // balance
    r.extend(b_config.build_storage().unwrap());
    // financialrecords
    r.extend(
        GenesisConfig::<Test> {
            genesis_multi_sig: vec![(
                1.into(),
                vec![
                    (1.into(), true),
                    (2.into(), true),
                    (3.into(), true),
                    (4.into(), false),
                    (5.into(), false),
                ],
                3,
                1000,
            )],
            deploy_fee: 10,
            exec_fee: 10,
            confirm_fee: 10,
            balances_config: b_config_copy,
        }
        .build_storage()
        .unwrap(),
    );
    r.into()
}

#[test]
fn test_genesis() {
    with_externalities(&mut new_test_ext(), || {
        let a: H256 = 1.into();
        let b: H256 = 2.into();
        let c: H256 = 3.into();

        // 10000 - 1000(first value) - 10(deploy fee) - 10(transfer fee) - 300(second value) - 10(deploy fee) - 10(transfer fee)
        assert_eq!(Balances::total_balance(&a), 8660);
        // 510 - 100 - 10 - 10
        assert_eq!(Balances::total_balance(&b), 390);

        // for 1
        for i in 0..MultiSig::multi_sig_list_len_for(&1.into()) {
            let addr = MultiSig::multi_sig_list_item_for((a.clone(), i)).unwrap();
            println!("{:?} {:} {:?}", a, i, addr);
            assert_eq!(
                Balances::total_balance(&addr),
                if i == 0 { 1000 } else { 300 }
            );
            assert_eq!(
                MultiSig::num_owner_for(&addr).unwrap(),
                if i == 0 { 5 } else { 3 }
            );
            assert_eq!(MultiSig::required_num_for(&addr).unwrap(), 3);
        }
        // for 2
        let addr = MultiSig::multi_sig_list_item_for((b.clone(), 0)).unwrap();
        println!("{:?} {:} {:?}", b, 0, addr);
        assert_eq!(Balances::total_balance(&addr), 100);
        assert_eq!(MultiSig::num_owner_for(&addr).unwrap(), 3);
        assert_eq!(MultiSig::required_num_for(&addr).unwrap(), 2);

        // for 3
        let addr = MultiSig::multi_sig_list_item_for((c.clone(), 0)).unwrap();
        println!("{:?} {:} {:?}", c, 0, addr);
        assert_eq!(Balances::total_balance(&addr), 100);
        assert_eq!(MultiSig::num_owner_for(&addr).unwrap(), 2);
        assert_eq!(MultiSig::required_num_for(&addr).unwrap(), 2);
        let origin = system::RawOrigin::Signed(c.clone()).into();
        assert_ok!(MultiSig::is_owner_for(origin, addr));
        assert_err!(
            MultiSig::is_owner(&c, &addr, true),
            "it's the owner but not required owner"
        );
    })
}

#[test]
#[should_panic]
fn test_err_genesis() {
    with_externalities(&mut err_test_ext(), || {})
}

#[test]
#[should_panic]
fn test_err_genesis2() {
    with_externalities(&mut err_test_ext2(), || {})
}

#[test]
fn test_multisig() {
    with_externalities(&mut new_test_ext(), || {
        let a: H256 = 1.into();
        let b: H256 = 2.into();
        // let c: H256 = 3.into();
        let d: H256 = 4.into();
        let e: H256 = 5.into();

        let addr = MultiSig::multi_sig_list_item_for((a.clone(), 0)).unwrap();

        let origin = system::RawOrigin::Signed(a.clone()).into();
        // transfer
        let t = TransferT::<Test> { to: a, value: 100 };
        assert_ok!(MultiSig::execute(
            origin,
            addr,
            TransactionType::TransferChainX,
            t.encode()
        ));

        assert_eq!(MultiSig::pending_list_len_for(addr), 1);
        let multi_sig_id = MultiSig::pending_list_item_for((addr.clone(), 0)).unwrap();
<<<<<<< HEAD
        assert_eq!(MultiSig::pending_state_for((addr.clone(), multi_sig_id)), PendingState { yet_needed: 2, owners_done: 1, index: 0 });
=======
        assert_eq!(
            MultiSig::pending_state_for((addr.clone(), multi_sig_id)),
            PendingState {
                yet_needed: 2,
                owners_done: 1,
                index: 0
            }
        );
>>>>>>> develop

        //b
        let origin = system::RawOrigin::Signed(b.clone()).into();
        assert_ok!(MultiSig::confirm(origin, addr, multi_sig_id));
        //a
        let origin = system::RawOrigin::Signed(a.clone()).into();
        assert_err!(
            MultiSig::confirm(origin, addr, multi_sig_id),
            "this account has confirmed for this multi sig addr and id"
        );
        //e
        let origin = system::RawOrigin::Signed(e.clone()).into();
        assert_ok!(MultiSig::confirm(origin, addr, multi_sig_id));

        assert_eq!(Balances::total_balance(&addr), 1000 - 100 - 10);
        // a: 8660 - 10(execute) + 100(transfer to)
        assert_eq!(Balances::total_balance(&a), 8660 - 10 + 100);

        // has delete
<<<<<<< HEAD
        assert_eq!(MultiSig::transaction_for((addr.clone(), multi_sig_id)), None);
        assert_eq!(MultiSig::pending_list_item_for((addr.clone(), 0)), None);

        assert_eq!(MultiSig::pending_state_for((addr.clone(), multi_sig_id)), PendingState { yet_needed: 0, owners_done: 0, index: 0 });
=======
        assert_eq!(
            MultiSig::transaction_for((addr.clone(), multi_sig_id)),
            None
        );
        assert_eq!(MultiSig::pending_list_item_for((addr.clone(), 0)), None);

        assert_eq!(
            MultiSig::pending_state_for((addr.clone(), multi_sig_id)),
            PendingState {
                yet_needed: 0,
                owners_done: 0,
                index: 0
            }
        );
>>>>>>> develop

        //d
        let origin = system::RawOrigin::Signed(d.clone()).into();
        assert_err!(
            MultiSig::confirm(origin, addr, multi_sig_id),
            "no pending tx for this addr and id or it has finished"
        );
    })
}

#[test]
fn test_not_required_owner() {
    with_externalities(&mut new_test_ext(), || {
        let a: H256 = 1.into();
        let b: H256 = 2.into();
        let c: H256 = 3.into();

        let addr = MultiSig::multi_sig_list_item_for((c.clone(), 0)).unwrap();
        let origin = system::RawOrigin::Signed(c.clone()).into();
        assert_ok!(MultiSig::is_owner_for(origin, addr));
        assert_err!(
            MultiSig::is_owner(&c, &addr, true),
            "it's the owner but not required owner"
        );

        let origin = system::RawOrigin::Signed(c.clone()).into();
        let t = TransferT::<Test> { to: a, value: 10 };
        assert_err!(
            MultiSig::execute(origin, addr, TransactionType::TransferChainX, t.encode()),
            "it's the owner but not required owner"
        );

        let origin = system::RawOrigin::Signed(b.clone()).into();
        assert_err!(
            MultiSig::execute(origin, addr, TransactionType::TransferChainX, t.encode()),
            "it's not the owner"
        );

        // exec success
        let origin = system::RawOrigin::Signed(a.clone()).into();
        assert_ok!(MultiSig::execute(
            origin,
            addr,
            TransactionType::TransferChainX,
            t.encode()
        ));
        // confirm success
        let multi_sig_id = MultiSig::pending_list_item_for((addr.clone(), 0)).unwrap();

        let origin = system::RawOrigin::Signed(c.clone()).into();
        assert_ok!(MultiSig::confirm(origin, addr, multi_sig_id));
    })
}

#[test]
fn test_not_exist() {
    with_externalities(&mut new_test_ext(), || {
        let a: H256 = 1.into();

        let origin = system::RawOrigin::Signed(a.clone()).into();
        let t = TransferT::<Test> { to: a, value: 10 };
        assert_err!(
            MultiSig::execute(
                origin,
                0.into(),
                TransactionType::TransferChainX,
                t.encode()
            ),
            "the multi sig addr not exist"
        );

        let origin = system::RawOrigin::Signed(a.clone()).into();
        assert_err!(
            MultiSig::confirm(origin, 0.into(), 0.into()),
            "the multi sig addr not exist"
        );

<<<<<<< HEAD

=======
>>>>>>> develop
        let addr = MultiSig::multi_sig_list_item_for((a.clone(), 0)).unwrap();
        let origin = system::RawOrigin::Signed(a.clone()).into();
        // transfer
        let t = TransferT::<Test> { to: a, value: 100 };
<<<<<<< HEAD
        assert_ok!(MultiSig::execute(origin, addr, TransactionType::TransferChainX, t.encode()));
=======
        assert_ok!(MultiSig::execute(
            origin,
            addr,
            TransactionType::TransferChainX,
            t.encode()
        ));
>>>>>>> develop
        let origin = system::RawOrigin::Signed(a.clone()).into();
        assert_err!(
            MultiSig::confirm(origin, addr, 0.into()),
            "no pending tx for this addr and id or it has finished"
        );
    })
}

#[test]
fn test_remove() {
    with_externalities(&mut new_test_ext(), || {
        let a: H256 = 1.into();
        let b: H256 = 2.into();
        let c: H256 = 3.into();
        let e: H256 = 5.into();

        let addr = MultiSig::multi_sig_list_item_for((a.clone(), 0)).unwrap();
        let origin = system::RawOrigin::Signed(a.clone()).into();
        // transfer
        // a
        let t = TransferT::<Test> { to: a, value: 100 };
<<<<<<< HEAD
        assert_ok!(MultiSig::execute(origin, addr, TransactionType::TransferChainX, t.encode()));
=======
        assert_ok!(MultiSig::execute(
            origin,
            addr,
            TransactionType::TransferChainX,
            t.encode()
        ));
>>>>>>> develop

        let multi_sig_id = MultiSig::pending_list_item_for((addr.clone(), 0)).unwrap();
        // b
        let origin = system::RawOrigin::Signed(b.clone()).into();
        assert_ok!(MultiSig::confirm(origin, addr, multi_sig_id));

        // yet need 1
<<<<<<< HEAD
        assert_eq!(MultiSig::pending_state_for((addr.clone(), multi_sig_id)), PendingState { yet_needed: 1, owners_done: 3, index: 0 });
=======
        assert_eq!(
            MultiSig::pending_state_for((addr.clone(), multi_sig_id)),
            PendingState {
                yet_needed: 1,
                owners_done: 3,
                index: 0
            }
        );
>>>>>>> develop

        // remove the pending
        // e can't remove
        let origin = system::RawOrigin::Signed(e.clone()).into();
<<<<<<< HEAD
        assert_err!(MultiSig::remove_multi_sig_for(origin, addr, multi_sig_id), "it's the owner but not required owner");
        assert_eq!(MultiSig::pending_state_for((addr.clone(), multi_sig_id)), PendingState { yet_needed: 1, owners_done: 3, index: 0 });
=======
        assert_err!(
            MultiSig::remove_multi_sig_for(origin, addr, multi_sig_id),
            "it's the owner but not required owner"
        );
        assert_eq!(
            MultiSig::pending_state_for((addr.clone(), multi_sig_id)),
            PendingState {
                yet_needed: 1,
                owners_done: 3,
                index: 0
            }
        );
>>>>>>> develop

        // c remove
        let origin = system::RawOrigin::Signed(c.clone()).into();
        assert_ok!(MultiSig::remove_multi_sig_for(origin, addr, multi_sig_id));

        // has del
<<<<<<< HEAD
        assert_eq!(MultiSig::transaction_for((addr.clone(), multi_sig_id)), None);
        assert_eq!(MultiSig::pending_state_for((addr.clone(), multi_sig_id)), PendingState { yet_needed: 0, owners_done: 0, index: 0 });
=======
        assert_eq!(
            MultiSig::transaction_for((addr.clone(), multi_sig_id)),
            None
        );
        assert_eq!(
            MultiSig::pending_state_for((addr.clone(), multi_sig_id)),
            PendingState {
                yet_needed: 0,
                owners_done: 0,
                index: 0
            }
        );
>>>>>>> develop

        let origin = system::RawOrigin::Signed(b.clone()).into();
        assert_err!(
            MultiSig::confirm(origin, addr, multi_sig_id),
            "no pending tx for this addr and id or it has finished"
        );
    })
}

#[test]
fn test_conflict() {
    with_externalities(&mut new_test_ext(), || {
        let a: H256 = 1.into();
        let b: H256 = 2.into();
        let c: H256 = 3.into();

        let addr = MultiSig::multi_sig_list_item_for((a.clone(), 1)).unwrap();
        // transfer1
        // a
        let t = TransferT::<Test> { to: a, value: 250 };
        let origin = system::RawOrigin::Signed(a.clone()).into();
<<<<<<< HEAD
        assert_ok!(MultiSig::execute(origin, addr, TransactionType::TransferChainX, t.encode()));
=======
        assert_ok!(MultiSig::execute(
            origin,
            addr,
            TransactionType::TransferChainX,
            t.encode()
        ));
>>>>>>> develop
        let multi_sig_id = MultiSig::pending_list_item_for((addr.clone(), 0)).unwrap();

        // transfer2
        // a
        let t2 = TransferT::<Test> { to: b, value: 250 };
        let origin = system::RawOrigin::Signed(b.clone()).into();
<<<<<<< HEAD
        assert_ok!(MultiSig::execute(origin, addr, TransactionType::TransferChainX, t2.encode()));
=======
        assert_ok!(MultiSig::execute(
            origin,
            addr,
            TransactionType::TransferChainX,
            t2.encode()
        ));
>>>>>>> develop
        let multi_sig_id2 = MultiSig::pending_list_item_for((addr.clone(), 1)).unwrap();

        // confirm b sign for id1
        let origin = system::RawOrigin::Signed(b.clone()).into();
        assert_ok!(MultiSig::confirm(origin, addr, multi_sig_id));

        // confirm a sign for id2
        let origin = system::RawOrigin::Signed(a.clone()).into();
        assert_ok!(MultiSig::confirm(origin, addr, multi_sig_id2));

        // confirm c sign for id2
        let origin = system::RawOrigin::Signed(c.clone()).into();
        assert_ok!(MultiSig::confirm(origin, addr, multi_sig_id2));
        // has del
<<<<<<< HEAD
        assert_eq!(MultiSig::transaction_for((addr.clone(), multi_sig_id2)), None);
        assert_eq!(MultiSig::pending_state_for((addr.clone(), multi_sig_id2)), PendingState { yet_needed: 0, owners_done: 0, index: 0 });


        assert_eq!(Balances::total_balance(&addr), 300 - 250 - 10);  // 300 - 250 - 10(fee)
=======
        assert_eq!(
            MultiSig::transaction_for((addr.clone(), multi_sig_id2)),
            None
        );
        assert_eq!(
            MultiSig::pending_state_for((addr.clone(), multi_sig_id2)),
            PendingState {
                yet_needed: 0,
                owners_done: 0,
                index: 0
            }
        );

        assert_eq!(Balances::total_balance(&addr), 300 - 250 - 10); // 300 - 250 - 10(fee)
>>>>>>> develop

        // confirm c sign for id1
        let origin = system::RawOrigin::Signed(c.clone()).into();
        assert_err!(
            MultiSig::confirm(origin, addr, multi_sig_id),
            "balance too low to send value"
        );
        // has del
<<<<<<< HEAD
        assert_eq!(MultiSig::transaction_for((addr.clone(), multi_sig_id)), None);
        assert_eq!(MultiSig::pending_state_for((addr.clone(), multi_sig_id)), PendingState { yet_needed: 0, owners_done: 0, index: 0 });

        assert_eq!(Balances::total_balance(&addr), 300 - 250 - 10);  // 300 - 250 - 10(fee)
=======
        assert_eq!(
            MultiSig::transaction_for((addr.clone(), multi_sig_id)),
            None
        );
        assert_eq!(
            MultiSig::pending_state_for((addr.clone(), multi_sig_id)),
            PendingState {
                yet_needed: 0,
                owners_done: 0,
                index: 0
            }
        );

        assert_eq!(Balances::total_balance(&addr), 300 - 250 - 10); // 300 - 250 - 10(fee)
>>>>>>> develop
    })
}

#[test]
fn test_parse_err() {
    with_externalities(&mut new_test_ext(), || {
        let a: H256 = 1.into();

        let addr = MultiSig::multi_sig_list_item_for((a.clone(), 1)).unwrap();
        // transfer1
        // a
        let t: Vec<u8> = vec![b't', b'e', b's', b't'];
        let origin = system::RawOrigin::Signed(a.clone()).into();
        assert_err!(
            MultiSig::execute(origin, addr, TransactionType::TransferChainX, t),
            "parse err for this tx data"
        );

        let mut t = (TransferT::<Test> { to: a, value: 250 }).encode();
        let e = t.len() - 1;
        t.remove(e);
        let origin = system::RawOrigin::Signed(a.clone()).into();
        assert_err!(
            MultiSig::execute(origin, addr, TransactionType::TransferChainX, t),
            "parse err for this tx data"
        );

        let mut t = (TransferT::<Test> { to: a, value: 250 }).encode();
        t.push(b'1');
        let origin = system::RawOrigin::Signed(a.clone()).into();
        assert_ok!(MultiSig::execute(
            origin,
            addr,
            TransactionType::TransferChainX,
            t
        ));
    })
}

#[test]
fn test_single() {
    with_externalities(&mut new_test_ext(), || {
        let a: H256 = 1.into();
        let b: H256 = 2.into();
        let c: H256 = 3.into();
        let d: H256 = 4.into();

        let addr = MultiSig::multi_sig_list_item_for((d.clone(), 0)).unwrap();

        // a
        let t = TransferT::<Test> { to: a, value: 5 };
        let origin = system::RawOrigin::Signed(a.clone()).into();
        assert_ok!(MultiSig::execute(
            origin,
            addr,
            TransactionType::TransferChainX,
            t.encode()
        ));

        assert_eq!(Balances::total_balance(&addr), 100 - 5 - 10);
        assert_eq!(MultiSig::pending_list_item_for((addr.clone(), 0)), None); // no pending state

        // b
        let t = TransferT::<Test> { to: a, value: 5 };
        let origin = system::RawOrigin::Signed(b.clone()).into();
        assert_ok!(MultiSig::execute(
            origin,
            addr,
            TransactionType::TransferChainX,
            t.encode()
        ));

        assert_eq!(Balances::total_balance(&addr), 100 - 5 - 10 - 5 - 10);
        assert_eq!(MultiSig::pending_list_item_for((addr.clone(), 0)), None); // no pending state

        // c can't
        let t = TransferT::<Test> { to: a, value: 5 };
        let origin = system::RawOrigin::Signed(c.clone()).into();
        assert_err!(
            MultiSig::execute(origin, addr, TransactionType::TransferChainX, t.encode()),
            "it's the owner but not required owner"
        );

        assert_eq!(Balances::total_balance(&addr), 100 - 5 - 10 - 5 - 10);
        assert_eq!(MultiSig::pending_list_item_for((addr.clone(), 0)), None); // no pending state
    })
}
