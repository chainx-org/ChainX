// Copyright 2018-2019 Chainpool.

#![cfg(test)]

use super::mock::*;
use super::*;

use matches::matches;
use runtime_io::with_externalities;
use substrate_primitives::crypto::UncheckedInto;
use substrate_primitives::H256;
use support::{assert_err, assert_ok};

fn deploy(account: AccountId, owners: Vec<(AccountId, MultiSigPermission)>, required_num: u32) {
    deploy_impl(account, owners, required_num, AddrType::Normal)
}

fn deploy_trustee(
    account: AccountId,
    owners: Vec<(AccountId, MultiSigPermission)>,
    required_num: u32,
) {
    deploy_impl(account, owners, required_num, AddrType::Trustee)
}

fn deploy_impl(
    account: AccountId,
    owners: Vec<(AccountId, MultiSigPermission)>,
    required_num: u32,
    addr_type: AddrType,
) {
    let multisig_addr = <Test as Trait>::MultiSig::multi_sig_addr_for(&account);
    MultiSig::deploy_impl(addr_type, &multisig_addr, &account, owners, required_num).unwrap();
}

#[test]
fn test_genesis() {
    with_externalities(&mut new_test_ext(), || {
        let a: AccountId = H256::repeat_byte(0x1).unchecked_into();
        let b: AccountId = H256::repeat_byte(0x2).unchecked_into();
        let c: AccountId = H256::repeat_byte(0x3).unchecked_into();

        let target_addr: <Test as system::Trait>::AccountId =
            <Test as system::Trait>::Hashing::hash(&b"Team"[..]).unchecked_into();
        let target_addr2: <Test as system::Trait>::AccountId =
            <Test as system::Trait>::Hashing::hash(&b"Council"[..]).unchecked_into();

        let owners = vec![
            (a.clone(), MultiSigPermission::ConfirmAndPropose),
            (b.clone(), MultiSigPermission::ConfirmAndPropose),
            (c.clone(), MultiSigPermission::ConfirmOnly),
        ];

        // deploy
        MultiSig::deploy_in_genesis(owners.clone(), 2, owners.clone(), 2).unwrap();

        let len = MultiSig::multi_sig_list_len_for(&a);
        assert_eq!(len, 2);
        let addr = MultiSig::multi_sig_list_item_for(&(a.clone(), 0));
        assert_eq!(target_addr, addr);
        let addr = MultiSig::multi_sig_list_item_for(&(a.clone(), 1));
        assert_eq!(target_addr2, addr);

        let addrinfo = MultiSig::multisig_addr_info(&target_addr).unwrap();
        assert!(matches!(addrinfo.addr_type, AddrType::Normal));
        assert_eq!(addrinfo.required_num, 2);
        assert_eq!(addrinfo.owner_list, owners);

        let addrinfo = MultiSig::multisig_addr_info(&target_addr2).unwrap();
        assert!(matches!(addrinfo.addr_type, AddrType::Root));
        assert_eq!(addrinfo.required_num, 2);
        assert_eq!(addrinfo.owner_list, owners);
    })
}

#[test]
fn test_multisig() {
    with_externalities(&mut new_test_ext(), || {
        let a: AccountId = H256::repeat_byte(0x1).unchecked_into();
        let b: AccountId = H256::repeat_byte(0x2).unchecked_into();
        let c: AccountId = H256::repeat_byte(0x3).unchecked_into();
        let d: AccountId = H256::repeat_byte(0x4).unchecked_into();
        let e: AccountId = H256::repeat_byte(0x5).unchecked_into();

        let owners = vec![
            (a.clone(), MultiSigPermission::ConfirmAndPropose),
            (b.clone(), MultiSigPermission::ConfirmAndPropose),
            (c.clone(), MultiSigPermission::ConfirmAndPropose),
            (d.clone(), MultiSigPermission::ConfirmOnly),
            (e.clone(), MultiSigPermission::ConfirmOnly),
        ];
        // deploy
        MultiSig::deploy_in_genesis(owners.clone(), 3, owners, 3).unwrap();

        // 1 is root
        let addr = MultiSig::multi_sig_list_item_for(&(a.clone(), 1));
        let origin = system::RawOrigin::Signed(a.clone()).into();
        // transfer
        let proposal = Box::new(TestCall(true));
        assert_ok!(MultiSig::execute(origin, addr.clone(), proposal.clone()));

        assert_eq!(MultiSig::pending_list_for(&addr).len(), 1);
        let multi_sig_id = MultiSig::pending_list_for(&addr).get(0).unwrap().clone();
        assert_eq!(
            MultiSig::pending_state_for(&(addr.clone(), multi_sig_id)),
            Some(PendingState::<<Test as Trait>::Proposal> {
                yet_needed: 2,
                owners_done: 1,
                proposal
            })
        );

        //b
        let origin = system::RawOrigin::Signed(b.clone()).into();
        assert_ok!(MultiSig::confirm(origin, addr.clone(), multi_sig_id));
        //a
        let origin = system::RawOrigin::Signed(a.clone()).into();
        assert_err!(
            MultiSig::confirm(origin, addr.clone(), multi_sig_id),
            "this account has confirmed for this multi sig addr and id"
        );
        //e
        let origin = system::RawOrigin::Signed(e.clone()).into();
        assert_err!(
            MultiSig::confirm(origin, addr.clone(), multi_sig_id),
            "call success"
        );

        // has delete
        assert_eq!(
            MultiSig::pending_state_for(&(addr.clone(), multi_sig_id)),
            None
        );
        assert_eq!(MultiSig::pending_list_for(&addr), vec![]);

        //d
        let origin = system::RawOrigin::Signed(d.clone()).into();
        assert_err!(
            MultiSig::confirm(origin, addr.clone(), multi_sig_id),
            "pending state not exist"
        );
    })
}

#[test]
fn test_not_required_owner() {
    with_externalities(&mut new_test_ext(), || {
        let a: AccountId = H256::repeat_byte(0x1).unchecked_into();
        let b: AccountId = H256::repeat_byte(0x2).unchecked_into();
        let c: AccountId = H256::repeat_byte(0x3).unchecked_into();
        let d: AccountId = H256::repeat_byte(0x4).unchecked_into();
        let owners = vec![
            (a.clone(), MultiSigPermission::ConfirmAndPropose),
            (b.clone(), MultiSigPermission::ConfirmOnly),
            (c.clone(), MultiSigPermission::ConfirmOnly),
        ];

        // deploy
        MultiSig::deploy_in_genesis(owners.clone(), 2, owners, 2).unwrap();

        // 1 is root
        let addr = MultiSig::multi_sig_list_item_for(&(a.clone(), 1));
        // let origin = system::RawOrigin::Signed(c.clone()).into();
        assert!(MultiSig::is_owner(&c, &addr.clone(), false).is_ok());
        assert_err!(
            MultiSig::is_owner(&c, &addr.clone(), true),
            "it's the owner but not required owner"
        );

        let origin = system::RawOrigin::Signed(c.clone()).into();
        let proposal = Box::new(TestCall(true));
        assert_err!(
            MultiSig::execute(origin, addr.clone(), proposal.clone()),
            "it's the owner but not required owner"
        );

        let origin = system::RawOrigin::Signed(d.clone()).into();
        assert_err!(
            MultiSig::execute(origin, addr.clone(), proposal.clone()),
            "it's not the owner"
        );

        // exec success
        let origin = system::RawOrigin::Signed(a.clone()).into();
        assert_ok!(MultiSig::execute(origin, addr.clone(), proposal.clone()));
        // confirm success
        let multi_sig_id = MultiSig::pending_list_for(&addr).get(0).unwrap().clone();

        let origin = system::RawOrigin::Signed(c.clone()).into();
        assert_err!(
            MultiSig::confirm(origin, addr.clone(), multi_sig_id),
            "call success"
        );
    })
}

#[test]
fn test_not_exist() {
    with_externalities(&mut new_test_ext(), || {
        let a: AccountId = H256::repeat_byte(0x1).unchecked_into();
        let b: AccountId = H256::repeat_byte(0x2).unchecked_into();
        let c: AccountId = H256::repeat_byte(0x3).unchecked_into();
        let fake: AccountId = H256::repeat_byte(0xf).unchecked_into();

        let owners = vec![
            (a.clone(), MultiSigPermission::ConfirmAndPropose),
            (b.clone(), MultiSigPermission::ConfirmOnly),
            (c.clone(), MultiSigPermission::ConfirmOnly),
        ];
        // deploy
        MultiSig::deploy_in_genesis(owners.clone(), 2, owners, 2).unwrap();

        let origin = system::RawOrigin::Signed(a.clone()).into();
        let proposal = Box::new(TestCall(true));
        assert_err!(
            MultiSig::execute(origin, fake.clone(), proposal),
            "the multi sig addr not exist"
        );

        let origin = system::RawOrigin::Signed(a.clone()).into();
        assert_err!(
            MultiSig::confirm(origin, fake.clone(), H256::default()),
            "the multi sig addr not exist"
        );

        let addr = MultiSig::multi_sig_list_item_for(&(a.clone(), 0));
        let origin = system::RawOrigin::Signed(a.clone()).into();
        // transfer
        let proposal = Box::new(TestCall(true));
        assert_ok!(MultiSig::execute(origin, addr.clone(), proposal));
        let origin = system::RawOrigin::Signed(a.clone()).into();
        assert_err!(
            MultiSig::confirm(origin, addr.clone(), H256::default()),
            "pending state not exist"
        );
    })
}

#[test]
fn test_remove() {
    with_externalities(&mut new_test_ext(), || {
        let a: AccountId = H256::repeat_byte(0x1).unchecked_into();
        let b: AccountId = H256::repeat_byte(0x2).unchecked_into();
        let c: AccountId = H256::repeat_byte(0x3).unchecked_into();
        let d: AccountId = H256::repeat_byte(0x4).unchecked_into();

        let owners = vec![
            (a.clone(), MultiSigPermission::ConfirmAndPropose),
            (b.clone(), MultiSigPermission::ConfirmAndPropose),
            (c.clone(), MultiSigPermission::ConfirmOnly),
            (d.clone(), MultiSigPermission::ConfirmOnly),
        ];
        // deploy
        MultiSig::deploy_in_genesis(owners.clone(), 3, owners, 3).unwrap();

        let addr = MultiSig::multi_sig_list_item_for(&(a.clone(), 0));
        let origin = system::RawOrigin::Signed(a.clone()).into();
        // transfer
        // a
        let proposal = Box::new(TestCall(true));
        assert_ok!(MultiSig::execute(origin, addr.clone(), proposal.clone()));

        let multi_sig_id = MultiSig::pending_list_for(&addr).get(0).unwrap().clone();
        // b
        let origin = system::RawOrigin::Signed(b.clone()).into();
        assert_ok!(MultiSig::confirm(origin, addr.clone(), multi_sig_id));

        // yet need 1
        assert_eq!(
            MultiSig::pending_state_for(&(addr.clone(), multi_sig_id)),
            Some(PendingState::<<Test as Trait>::Proposal> {
                yet_needed: 1,
                owners_done: 3,
                proposal: proposal.clone()
            })
        );
        // remove the pending
        // d can't remove
        let origin = system::RawOrigin::Signed(d.clone()).into();
        assert_err!(
            MultiSig::remove_multi_sig_for(origin, addr.clone(), multi_sig_id),
            "it's the owner but not required owner"
        );
        assert_eq!(
            MultiSig::pending_state_for(&(addr.clone(), multi_sig_id)),
            Some(PendingState::<<Test as Trait>::Proposal> {
                yet_needed: 1,
                owners_done: 3,
                proposal: proposal.clone()
            })
        );

        // b remove
        let origin = system::RawOrigin::Signed(b.clone()).into();
        assert_ok!(MultiSig::remove_multi_sig_for(
            origin,
            addr.clone(),
            multi_sig_id
        ));

        // has del
        assert_eq!(
            MultiSig::pending_state_for(&(addr.clone(), multi_sig_id)),
            None
        );

        let origin = system::RawOrigin::Signed(b.clone()).into();
        assert_err!(
            MultiSig::confirm(origin, addr.clone(), multi_sig_id),
            "pending state not exist"
        );
    })
}

#[test]
fn test_single() {
    with_externalities(&mut new_test_ext(), || {
        let a: AccountId = H256::repeat_byte(0x1).unchecked_into();
        let b: AccountId = H256::repeat_byte(0x2).unchecked_into();
        let c: AccountId = H256::repeat_byte(0x3).unchecked_into();
        let d: AccountId = H256::repeat_byte(0x4).unchecked_into();
        let e: AccountId = H256::repeat_byte(0x5).unchecked_into();

        let owners = vec![
            (a.clone(), MultiSigPermission::ConfirmAndPropose),
            (b.clone(), MultiSigPermission::ConfirmAndPropose),
            (c.clone(), MultiSigPermission::ConfirmOnly),
            (d.clone(), MultiSigPermission::ConfirmOnly),
        ];

        // user deploy, not root
        // let origin = system::RawOrigin::Signed(a.clone()).into();
        // assert_ok!(MultiSig::deploy(origin, owners, 1));
        deploy(a.clone(), owners, 1);
        // a
        let addr = MultiSig::multi_sig_list_item_for(&(a.clone(), 0));

        let proposal = Box::new(TestCall(false));
        let origin = system::RawOrigin::Signed(a.clone()).into();
        assert_err!(
            MultiSig::execute(origin, addr.clone(), proposal.clone()),
            "call success"
        );

        // c, not required owner
        let origin = system::RawOrigin::Signed(c.clone()).into();
        assert_err!(
            MultiSig::execute(origin, addr.clone(), proposal.clone()),
            "it\'s the owner but not required owner"
        );

        // e, not owner
        let origin = system::RawOrigin::Signed(e.clone()).into();
        assert_err!(
            MultiSig::execute(origin, addr.clone(), proposal.clone()),
            "it\'s not the owner"
        );
    })
}

#[test]
fn test_proposal_too_many() {
    with_externalities(&mut new_test_ext(), || {
        let a: AccountId = H256::repeat_byte(0x1).unchecked_into();
        let b: AccountId = H256::repeat_byte(0x2).unchecked_into();
        let c: AccountId = H256::repeat_byte(0x3).unchecked_into();
        let d: AccountId = H256::repeat_byte(0x4).unchecked_into();
        //        let e: H256 = H256::repeat_byte(0x5);

        let owners = vec![
            (a.clone(), MultiSigPermission::ConfirmAndPropose),
            (b.clone(), MultiSigPermission::ConfirmAndPropose),
            (c.clone(), MultiSigPermission::ConfirmOnly),
            (d.clone(), MultiSigPermission::ConfirmOnly),
        ];
        // let origin = system::RawOrigin::Signed(a.clone()).into();
        // assert_ok!(MultiSig::deploy(origin, owners, 3));
        deploy(a.clone(), owners, 3);

        let addr = MultiSig::multi_sig_list_item_for(&(a.clone(), 0));

        let proposal = Box::new(TestCall(false));
        let origin = system::RawOrigin::Signed(a.clone()).into();
        assert_ok!(MultiSig::execute(origin, addr.clone(), proposal.clone()));
        <system::Module<Test>>::inc_account_nonce(&a);

        let origin = system::RawOrigin::Signed(b.clone()).into();
        assert_ok!(MultiSig::execute(origin, addr.clone(), proposal.clone()));
        <system::Module<Test>>::inc_account_nonce(&b);

        let origin = system::RawOrigin::Signed(a.clone()).into();
        assert_ok!(MultiSig::execute(origin, addr.clone(), proposal.clone()));
        <system::Module<Test>>::inc_account_nonce(&a);

        let origin = system::RawOrigin::Signed(b.clone()).into();
        assert_ok!(MultiSig::execute(origin, addr.clone(), proposal.clone()));
        <system::Module<Test>>::inc_account_nonce(&b);

        let origin = system::RawOrigin::Signed(a.clone()).into();
        assert_ok!(MultiSig::execute(origin, addr.clone(), proposal.clone()));
        <system::Module<Test>>::inc_account_nonce(&a);

        assert_eq!(MultiSig::pending_list_for(&addr).len(), 5);
        // already 5 proposal
        let origin = system::RawOrigin::Signed(b.clone()).into();
        assert_err!(
            MultiSig::execute(origin, addr.clone(), proposal.clone()),
            "pending list can't be larger than MAX_PENDING"
        );
        <system::Module<Test>>::inc_account_nonce(&b);

        // confirm 1
        let multi_sig_id = MultiSig::pending_list_for(&addr).get(0).unwrap().clone();
        let origin = system::RawOrigin::Signed(b.clone()).into();
        assert_ok!(MultiSig::confirm(origin, addr.clone(), multi_sig_id));
        <system::Module<Test>>::inc_account_nonce(&b);

        let origin = system::RawOrigin::Signed(c.clone()).into();
        assert_err!(
            MultiSig::confirm(origin, addr.clone(), multi_sig_id),
            "call success"
        );
        <system::Module<Test>>::inc_account_nonce(&c);

        assert_eq!(MultiSig::pending_list_for(&addr).len(), 4);
        // insert 6
        let origin = system::RawOrigin::Signed(b.clone()).into();
        assert_ok!(MultiSig::execute(origin, addr.clone(), proposal.clone()));
        <system::Module<Test>>::inc_account_nonce(&b);

        assert_eq!(MultiSig::pending_list_for(&addr).len(), 5);
        let origin = system::RawOrigin::Signed(a.clone()).into();
        assert_err!(
            MultiSig::execute(origin, addr.clone(), proposal.clone()),
            "pending list can't be larger than MAX_PENDING"
        );
        <system::Module<Test>>::inc_account_nonce(&a);

        // remove 2
        let multi_sig_id = MultiSig::pending_list_for(&addr).get(0).unwrap().clone();
        let origin = system::RawOrigin::Signed(b.clone()).into();
        assert_ok!(MultiSig::remove_multi_sig_for(
            origin,
            addr.clone(),
            multi_sig_id
        ));
        <system::Module<Test>>::inc_account_nonce(&b);
        assert_eq!(MultiSig::pending_list_for(&addr).len(), 4);

        // insert 7
        let origin = system::RawOrigin::Signed(a.clone()).into();
        assert_ok!(MultiSig::execute(origin, addr.clone(), proposal.clone()));
        <system::Module<Test>>::inc_account_nonce(&a);
    })
}

#[test]
fn test_try_to_call_root() {
    with_externalities(&mut new_test_ext(), || {
        let a: AccountId = H256::repeat_byte(0x1).unchecked_into();
        let b: AccountId = H256::repeat_byte(0x2).unchecked_into();
        let c: AccountId = H256::repeat_byte(0x3).unchecked_into();
        let d: AccountId = H256::repeat_byte(0x4).unchecked_into();
        let e: AccountId = H256::repeat_byte(0x5).unchecked_into();
        // root multisig addr
        let owners = vec![
            (a.clone(), MultiSigPermission::ConfirmAndPropose),
            (b.clone(), MultiSigPermission::ConfirmAndPropose),
            (c.clone(), MultiSigPermission::ConfirmOnly),
        ];
        MultiSig::deploy_in_genesis(owners.clone(), 2, owners.clone(), 2).unwrap();
        // no multisig addr
        let owners = vec![
            (a.clone(), MultiSigPermission::ConfirmAndPropose),
            (b.clone(), MultiSigPermission::ConfirmAndPropose),
            (c.clone(), MultiSigPermission::ConfirmOnly),
        ];
        // addr1
        // let origin = system::RawOrigin::Signed(a.clone()).into();
        // assert_ok!(MultiSig::deploy(origin, owners, 2));
        deploy(a.clone(), owners, 2);

        let owners = vec![
            (c.clone(), MultiSigPermission::ConfirmAndPropose),
            (d.clone(), MultiSigPermission::ConfirmAndPropose),
            (e.clone(), MultiSigPermission::ConfirmOnly),
        ];
        // addr2
        // let origin = system::RawOrigin::Signed(a.clone()).into();
        // assert_ok!(MultiSig::deploy(origin, owners, 2));
        deploy(a.clone(), owners, 2);

        // 1 is root
        let addr_genesis1 = MultiSig::multi_sig_list_item_for(&(a.clone(), 1));
        //        let addr_genesis2 = MultiSig::multi_sig_list_item_for(&(a.clone(), 1));
        let addr_a1 = MultiSig::multi_sig_list_item_for(&(a.clone(), 2));
        let addr_c1 = MultiSig::multi_sig_list_item_for(&(a.clone(), 0));

        assert_eq!(MultiSig::multisig_addr_info(&addr_genesis1).is_some(), true);
        //        assert_eq!(MultiSig::multisig_addr_info(&addr_genesis2).is_some(), true);
        assert_eq!(MultiSig::multisig_addr_info(&addr_a1).is_some(), true);
        assert_eq!(MultiSig::multisig_addr_info(&addr_c1).is_some(), true);
        // =================
        let proposal = Box::new(TestCall(true));
        let proposal2 = Box::new(TestCall(false));
        let origin = system::RawOrigin::Signed(a.clone()).into();
        assert_ok!(MultiSig::execute(
            origin,
            addr_genesis1.clone(),
            proposal.clone()
        ));
        <system::Module<Test>>::inc_account_nonce(&a);
        let origin = system::RawOrigin::Signed(b.clone()).into();
        assert_ok!(MultiSig::execute(
            origin,
            addr_genesis1.clone(),
            proposal2.clone()
        ));
        <system::Module<Test>>::inc_account_nonce(&b);
        let addr_genesis1_id = MultiSig::pending_list_for(&addr_genesis1)
            .get(0)
            .unwrap()
            .clone();
        let addr_genesis1_id2 = MultiSig::pending_list_for(&addr_genesis1)
            .get(1)
            .unwrap()
            .clone();

        let origin = system::RawOrigin::Signed(d.clone()).into();
        assert_err!(
            MultiSig::execute(origin, addr_genesis1.clone(), proposal.clone()),
            "it\'s not the owner"
        );
        let origin = system::RawOrigin::Signed(d.clone()).into();
        assert_err!(
            MultiSig::confirm(origin, addr_genesis1.clone(), addr_genesis1_id),
            "it\'s not the owner"
        );

        let origin = system::RawOrigin::Signed(b.clone()).into();
        assert_err!(
            MultiSig::confirm(origin, addr_genesis1.clone(), addr_genesis1_id),
            "call success"
        );
        let origin = system::RawOrigin::Signed(c.clone()).into();
        assert_err!(
            MultiSig::confirm(origin, addr_genesis1.clone(), addr_genesis1_id2),
            "call success"
        );
        // ========= genesis addr test finish

        let origin = system::RawOrigin::Signed(a.clone()).into();
        assert_ok!(MultiSig::execute(origin, addr_a1.clone(), proposal.clone()));
        <system::Module<Test>>::inc_account_nonce(&a);
        let origin = system::RawOrigin::Signed(b.clone()).into();
        assert_ok!(MultiSig::execute(
            origin,
            addr_a1.clone(),
            proposal2.clone()
        ));
        <system::Module<Test>>::inc_account_nonce(&b);
        let addr_a1_id = MultiSig::pending_list_for(&addr_a1).get(0).unwrap().clone();
        let addr_a1_id2 = MultiSig::pending_list_for(&addr_a1).get(1).unwrap().clone();

        let origin = system::RawOrigin::Signed(d.clone()).into();
        assert_err!(
            MultiSig::execute(origin, addr_a1.clone(), proposal.clone()),
            "it\'s not the owner"
        );
        let origin = system::RawOrigin::Signed(d.clone()).into();
        assert_err!(
            MultiSig::confirm(origin, addr_a1.clone(), addr_a1_id),
            "it\'s not the owner"
        );

        let origin = system::RawOrigin::Signed(b.clone()).into();
        assert_err!(
            MultiSig::confirm(origin, addr_a1.clone(), addr_a1_id),
            "bad origin: expected to be a root origin"
        );
        let origin = system::RawOrigin::Signed(c.clone()).into();
        assert_err!(
            MultiSig::confirm(origin, addr_a1.clone(), addr_a1_id2),
            "call success"
        );
    })
}

#[test]
fn test_deploy() {
    with_externalities(&mut new_test_ext(), || {
        let a: AccountId = H256::repeat_byte(0x1).unchecked_into();
        let b: AccountId = H256::repeat_byte(0x2).unchecked_into();
        let c: AccountId = H256::repeat_byte(0x3).unchecked_into();
        let d: AccountId = H256::repeat_byte(0x4).unchecked_into();
        let e: AccountId = H256::repeat_byte(0x5).unchecked_into();
        // root multisig addr
        let owners = vec![
            (a.clone(), MultiSigPermission::ConfirmAndPropose),
            (b.clone(), MultiSigPermission::ConfirmAndPropose),
            (c.clone(), MultiSigPermission::ConfirmOnly),
        ];
        MultiSig::deploy_in_genesis(owners.clone(), 2, owners.clone(), 2).unwrap();
        // no multisig addr
        let _owners = vec![
            (a.clone(), MultiSigPermission::ConfirmAndPropose),
            (b.clone(), MultiSigPermission::ConfirmAndPropose),
            (c.clone(), MultiSigPermission::ConfirmOnly),
            (d.clone(), MultiSigPermission::ConfirmOnly),
            (e.clone(), MultiSigPermission::ConfirmOnly),
        ];
        // let _origin = system::RawOrigin::Signed(a.clone()).into();
        // current deploy has been removed.
    })
}

#[test]
fn test_limited_call() {
    with_externalities(&mut new_test_ext2(), || {
        let a: AccountId = H256::repeat_byte(0x1).unchecked_into();
        let b: AccountId = H256::repeat_byte(0x2).unchecked_into();
        let c: AccountId = H256::repeat_byte(0x3).unchecked_into();
        // root multisig addr
        let owners = vec![
            (a.clone(), MultiSigPermission::ConfirmAndPropose),
            (b.clone(), MultiSigPermission::ConfirmAndPropose),
            (c.clone(), MultiSigPermission::ConfirmOnly),
        ];
        deploy_trustee(a.clone(), owners, 2);

        let addr = MultiSig::multi_sig_list_item_for(&(a.clone(), 0));
        let origin = system::RawOrigin::Signed(a.clone()).into();
        // root call
        let proposal = Box::new(TestCall(true));
        assert_err!(
            MultiSig::execute(origin, addr.clone(), proposal.clone()),
            "do not allow trustee multisig addr to call this proposal"
        );

        // not allow call
        let mycall = MyCall::normal_call;
        let proposal = Box::new(mycall);
        let origin = system::RawOrigin::Signed(a.clone()).into();
        assert_err!(
            MultiSig2::execute(origin, addr.clone(), proposal),
            "do not allow trustee multisig addr to call this proposal"
        );

        // allow call
        let mycall = MyCall::normal_call2;
        let proposal = Box::new(mycall);
        let origin = system::RawOrigin::Signed(a.clone()).into();
        assert_ok!(MultiSig2::execute(origin, addr.clone(), proposal));

        // root call
        let mycall = MyCall::root_call;
        let proposal = Box::new(mycall);
        let origin = system::RawOrigin::Signed(a.clone()).into();
        assert_err!(
            MultiSig2::execute(origin, addr.clone(), proposal),
            "do not allow trustee multisig addr to call this proposal"
        );
    });
}

#[test]
fn test_transition() {
    with_externalities(&mut new_test_ext2(), || {
        let a: AccountId = H256::repeat_byte(0x1).unchecked_into();
        let b: AccountId = H256::repeat_byte(0x2).unchecked_into();
        let c: AccountId = H256::repeat_byte(0x3).unchecked_into();
        // root multisig addr
        let owners = vec![
            (a.clone(), MultiSigPermission::ConfirmAndPropose),
            (b.clone(), MultiSigPermission::ConfirmAndPropose),
            (c.clone(), MultiSigPermission::ConfirmOnly),
        ];
        // a normal multisig addr and a root multisig addr
        MultiSig::deploy_in_genesis(owners.clone(), 2, owners.clone(), 2).unwrap();
        // a trustee multisig addr
        deploy_trustee(a.clone(), owners.clone(), 2);
        let new_owners = vec![(b.clone(), false), (a.clone(), true)];

        // normal
        let addr1 = MultiSig2::multi_sig_list_item_for(&(a.clone(), 0));
        // root
        let addr2 = MultiSig2::multi_sig_list_item_for(&(a.clone(), 1));
        // trustee
        let addr3 = MultiSig2::multi_sig_list_item_for(&(a.clone(), 2));

        // user call directly, must fail
        let origin = system::RawOrigin::Signed(a.clone()).into();
        assert_err!(
            MultiSig2::transition(origin, new_owners.clone(), 2),
            "multisig address not exist."
        );

        // normal addr test
        let mycall = MyCall::transition(new_owners.clone(), 2);
        let proposal = Box::new(mycall);
        let origin = system::RawOrigin::Signed(a.clone()).into();
        assert_ok!(MultiSig2::execute(origin, addr1.clone(), proposal));
        let multi_sig_id = MultiSig::pending_list_for(&addr1)[0];
        let origin = system::RawOrigin::Signed(b.clone()).into();
        assert_ok!(MultiSig2::confirm(origin, addr1.clone(), multi_sig_id)); // exec success

        let new_addr = MultiSig2::multi_sig_list_item_for(&(b.clone(), 0));
        assert_eq!(new_addr, addr1); // transition do not modify address
        let addr_info = MultiSig2::multisig_addr_info(&new_addr).unwrap(); // new address must exist
        let expect = new_owners
            .clone()
            .into_iter()
            .map(|(a, _)| (a, MultiSigPermission::ConfirmAndPropose))
            .collect::<Vec<_>>();
        assert_eq!(addr_info.owner_list, expect);
        assert_eq!(addr_info.required_num, 2);

        let mycall = MyCall::transition(vec![], 0);
        let proposal = Box::new(mycall);
        let origin = system::RawOrigin::Signed(a.clone()).into();
        assert_ok!(MultiSig2::execute(origin, new_addr.clone(), proposal));
        let multi_sig_id = MultiSig::pending_list_for(&addr1)[0];
        let origin = system::RawOrigin::Signed(b.clone()).into();
        assert_err!(
            MultiSig2::confirm(origin, new_addr.clone(), multi_sig_id),
            "owners can't be empty."
        );

        // root addr test
        let mycall = MyCall::transition(new_owners.clone(), 2);
        let proposal = Box::new(mycall);
        let origin = system::RawOrigin::Signed(a.clone()).into();
        assert_ok!(MultiSig2::execute(origin, addr2.clone(), proposal));
        let multi_sig_id = MultiSig::pending_list_for(&addr2)[0];
        let origin = system::RawOrigin::Signed(b.clone()).into();
        assert_ok!(MultiSig2::confirm(origin, addr2.clone(), multi_sig_id)); // exec success
        let new_addr = MultiSig2::multi_sig_list_item_for(&(b.clone(), 1));
        let _ = MultiSig2::multisig_addr_info(&new_addr).unwrap(); // new address must exist

        // trustee can't transition from this call
        let mycall = MyCall::transition(new_owners.clone(), 2);
        let proposal = Box::new(mycall);
        let origin = system::RawOrigin::Signed(a.clone()).into();
        assert_err!(
            MultiSig2::execute(origin, addr3.clone(), proposal),
            "do not allow trustee multisig addr to call this proposal"
        );
    })
}
