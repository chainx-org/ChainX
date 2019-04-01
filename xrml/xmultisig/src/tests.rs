// Copyright 2018-2019 Chainpool.

#![cfg(test)]

use super::mock::*;
use super::*;

use runtime_io::with_externalities;
use substrate_primitives::H256;
use support::{assert_err, assert_ok};

#[test]
fn test_genesis() {
    with_externalities(&mut new_test_ext(), || {
        let a: H256 = H256::repeat_byte(0x1);
        let b: H256 = H256::repeat_byte(0x2);
        let c: H256 = H256::repeat_byte(0x3);

        let mut buf = Vec::<u8>::new();
        let h: <Test as system::Trait>::Hash = a.clone().into();
        buf.extend_from_slice(h.as_ref());
        let h: <Test as system::Trait>::Hash = b.clone().into();
        buf.extend_from_slice(h.as_ref());
        let h: <Test as system::Trait>::Hash = c.clone().into();
        buf.extend_from_slice(h.as_ref());
        let target_addr: <Test as system::Trait>::AccountId =
            <Test as system::Trait>::Hashing::hash(&buf[..]).into();
        let target_addr2: <Test as system::Trait>::AccountId =
            <Test as system::Trait>::Hashing::hash(&b"Council"[..]).into();

        let owners = vec![(a.clone(), true), (b.clone(), true), (c.clone(), false)];

        // deploy
        MultiSig::deploy_in_genesis(owners.clone(), 2);

        let len = MultiSig::multi_sig_list_len_for(a);
        assert_eq!(len, 2);
        let addr = MultiSig::multi_sig_list_item_for(&(a, 0));
        assert_eq!(target_addr, addr);
        let addr = MultiSig::multi_sig_list_item_for(&(a, 1));
        assert_eq!(target_addr2, addr);

        let addrinfo = MultiSig::multisig_addr_info(&target_addr).unwrap();
        assert_eq!(addrinfo.is_root, true);
        assert_eq!(addrinfo.required_num, 2);
        assert_eq!(addrinfo.owner_list, owners);

        let addrinfo = MultiSig::multisig_addr_info(&target_addr2).unwrap();
        assert_eq!(addrinfo.is_root, true);
        assert_eq!(addrinfo.required_num, 2);
        assert_eq!(addrinfo.owner_list, owners);
    })
}

#[test]
fn test_multisig() {
    with_externalities(&mut new_test_ext(), || {
        let a: H256 = H256::repeat_byte(0x1);
        let b: H256 = H256::repeat_byte(0x2);
        let c: H256 = H256::repeat_byte(0x3);
        let d: H256 = H256::repeat_byte(0x4);
        let e: H256 = H256::repeat_byte(0x5);

        let owners = vec![
            (a.clone(), true),
            (b.clone(), true),
            (c.clone(), true),
            (d.clone(), false),
            (e.clone(), false),
        ];
        // deploy
        MultiSig::deploy_in_genesis(owners, 3);

        let addr = MultiSig::multi_sig_list_item_for(&(a.clone(), 0));
        let origin = system::RawOrigin::Signed(a.clone()).into();
        // transfer
        let proposal = Box::new(TestCall(true));
        assert_ok!(MultiSig::execute(origin, addr, proposal.clone()));

        assert_eq!(MultiSig::pending_list_for(addr).len(), 1);
        let multi_sig_id = MultiSig::pending_list_for(&addr).get(0).unwrap().clone();
        assert_eq!(
            MultiSig::pending_state_for(&(addr, multi_sig_id)),
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
        let a: H256 = H256::repeat_byte(0x1);
        let b: H256 = H256::repeat_byte(0x2);
        let c: H256 = H256::repeat_byte(0x3);
        let d: H256 = H256::repeat_byte(0x4);
        let owners = vec![(a.clone(), true), (b.clone(), false), (c.clone(), false)];

        // deploy
        MultiSig::deploy_in_genesis(owners, 2);

        let addr = MultiSig::multi_sig_list_item_for(&(a.clone(), 0));
        let origin = system::RawOrigin::Signed(c.clone()).into();
        assert_ok!(MultiSig::is_owner_for(origin, addr.clone()));
        assert_err!(
            MultiSig::is_owner(&c, &addr, true),
            "it's the owner but not required owner"
        );

        let origin = system::RawOrigin::Signed(c.clone()).into();
        let proposal = Box::new(TestCall(true));
        assert_err!(
            MultiSig::execute(origin, addr, proposal.clone()),
            "it's the owner but not required owner"
        );

        let origin = system::RawOrigin::Signed(d.clone()).into();
        assert_err!(
            MultiSig::execute(origin, addr, proposal.clone()),
            "it's not the owner"
        );

        // exec success
        let origin = system::RawOrigin::Signed(a.clone()).into();
        assert_ok!(MultiSig::execute(origin, addr, proposal.clone()));
        // confirm success
        let multi_sig_id = MultiSig::pending_list_for(&addr).get(0).unwrap().clone();

        let origin = system::RawOrigin::Signed(c.clone()).into();
        assert_err!(
            MultiSig::confirm(origin, addr, multi_sig_id),
            "call success"
        );
    })
}

#[test]
fn test_not_exist() {
    with_externalities(&mut new_test_ext(), || {
        let a: H256 = H256::repeat_byte(0x1);
        let b: H256 = H256::repeat_byte(0x2);
        let c: H256 = H256::repeat_byte(0x3);
        let fake: H256 = H256::repeat_byte(0xf);

        let owners = vec![(a.clone(), true), (b.clone(), false), (c.clone(), false)];
        // deploy
        MultiSig::deploy_in_genesis(owners, 2);

        let origin = system::RawOrigin::Signed(a.clone()).into();
        let proposal = Box::new(TestCall(true));
        assert_err!(
            MultiSig::execute(origin, fake, proposal),
            "the multi sig addr not exist"
        );

        let origin = system::RawOrigin::Signed(a.clone()).into();
        assert_err!(
            MultiSig::confirm(origin, fake, fake),
            "the multi sig addr not exist"
        );

        let addr = MultiSig::multi_sig_list_item_for(&(a, 0));
        let origin = system::RawOrigin::Signed(a.clone()).into();
        // transfer
        let proposal = Box::new(TestCall(true));
        assert_ok!(MultiSig::execute(origin, addr, proposal));
        let origin = system::RawOrigin::Signed(a.clone()).into();
        assert_err!(
            MultiSig::confirm(origin, addr, fake),
            "pending state not exist"
        );
    })
}

#[test]
fn test_remove() {
    with_externalities(&mut new_test_ext(), || {
        let a: H256 = H256::repeat_byte(0x1);
        let b: H256 = H256::repeat_byte(0x2);
        let c: H256 = H256::repeat_byte(0x3);
        let d: H256 = H256::repeat_byte(0x4);

        let owners = vec![
            (a.clone(), true),
            (b.clone(), true),
            (c.clone(), false),
            (d.clone(), false),
        ];
        // deploy
        MultiSig::deploy_in_genesis(owners, 3);

        let addr = MultiSig::multi_sig_list_item_for(&(a, 0));
        let origin = system::RawOrigin::Signed(a.clone()).into();
        // transfer
        // a
        let proposal = Box::new(TestCall(true));
        assert_ok!(MultiSig::execute(origin, addr, proposal.clone()));

        let multi_sig_id = MultiSig::pending_list_for(&addr).get(0).unwrap().clone();
        // b
        let origin = system::RawOrigin::Signed(b.clone()).into();
        assert_ok!(MultiSig::confirm(origin, addr, multi_sig_id));

        // yet need 1
        assert_eq!(
            MultiSig::pending_state_for(&(addr, multi_sig_id)),
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
            MultiSig::remove_multi_sig_for(origin, addr, multi_sig_id),
            "it's the owner but not required owner"
        );
        assert_eq!(
            MultiSig::pending_state_for(&(addr, multi_sig_id)),
            Some(PendingState::<<Test as Trait>::Proposal> {
                yet_needed: 1,
                owners_done: 3,
                proposal: proposal.clone()
            })
        );

        // b remove
        let origin = system::RawOrigin::Signed(b.clone()).into();
        assert_ok!(MultiSig::remove_multi_sig_for(origin, addr, multi_sig_id));

        // has del
        assert_eq!(MultiSig::pending_state_for(&(addr, multi_sig_id)), None);

        let origin = system::RawOrigin::Signed(b.clone()).into();
        assert_err!(
            MultiSig::confirm(origin, addr, multi_sig_id),
            "pending state not exist"
        );
    })
}

#[test]
fn test_single() {
    with_externalities(&mut new_test_ext(), || {
        let a: H256 = H256::repeat_byte(0x1);
        let b: H256 = H256::repeat_byte(0x2);
        let c: H256 = H256::repeat_byte(0x3);
        let d: H256 = H256::repeat_byte(0x4);
        let e: H256 = H256::repeat_byte(0x5);

        let owners = vec![
            (a.clone(), true),
            (b.clone(), true),
            (c.clone(), false),
            (d.clone(), false),
        ];
        let origin = system::RawOrigin::Signed(a.clone()).into();
        // user deploy, not root
        assert_ok!(MultiSig::deploy(origin, owners, 1));
        // a
        let addr = MultiSig::multi_sig_list_item_for(&(a, 0));

        let proposal = Box::new(TestCall(false));
        let origin = system::RawOrigin::Signed(a.clone()).into();
        assert_err!(
            MultiSig::execute(origin, addr, proposal.clone()),
            "call success"
        );

        // c, not required owner
        let origin = system::RawOrigin::Signed(c.clone()).into();
        assert_err!(
            MultiSig::execute(origin, addr, proposal.clone()),
            "it\'s the owner but not required owner"
        );

        // e, not owner
        let origin = system::RawOrigin::Signed(e.clone()).into();
        assert_err!(
            MultiSig::execute(origin, addr, proposal.clone()),
            "it\'s not the owner"
        );
    })
}

#[test]
fn test_proposal_too_many() {
    with_externalities(&mut new_test_ext(), || {
        let a: H256 = H256::repeat_byte(0x1);
        let b: H256 = H256::repeat_byte(0x2);
        let c: H256 = H256::repeat_byte(0x3);
        let d: H256 = H256::repeat_byte(0x4);
        //        let e: H256 = H256::repeat_byte(0x5);

        let owners = vec![
            (a.clone(), true),
            (b.clone(), true),
            (c.clone(), false),
            (d.clone(), false),
        ];
        let origin = system::RawOrigin::Signed(a.clone()).into();
        assert_ok!(MultiSig::deploy(origin, owners, 3));

        let addr = MultiSig::multi_sig_list_item_for(&(a, 0));

        let proposal = Box::new(TestCall(false));
        let origin = system::RawOrigin::Signed(a.clone()).into();
        assert_ok!(MultiSig::execute(origin, addr, proposal.clone()));
        <system::Module<Test>>::inc_account_nonce(&a);

        let origin = system::RawOrigin::Signed(b.clone()).into();
        assert_ok!(MultiSig::execute(origin, addr, proposal.clone()));
        <system::Module<Test>>::inc_account_nonce(&b);

        let origin = system::RawOrigin::Signed(a.clone()).into();
        assert_ok!(MultiSig::execute(origin, addr, proposal.clone()));
        <system::Module<Test>>::inc_account_nonce(&a);

        let origin = system::RawOrigin::Signed(b.clone()).into();
        assert_ok!(MultiSig::execute(origin, addr, proposal.clone()));
        <system::Module<Test>>::inc_account_nonce(&b);

        let origin = system::RawOrigin::Signed(a.clone()).into();
        assert_ok!(MultiSig::execute(origin, addr, proposal.clone()));
        <system::Module<Test>>::inc_account_nonce(&a);

        assert_eq!(MultiSig::pending_list_for(&addr).len(), 5);
        // already 5 proposal
        let origin = system::RawOrigin::Signed(b.clone()).into();
        assert_err!(
            MultiSig::execute(origin, addr, proposal.clone()),
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
            MultiSig::confirm(origin, addr, multi_sig_id),
            "call success"
        );
        <system::Module<Test>>::inc_account_nonce(&c);

        assert_eq!(MultiSig::pending_list_for(&addr).len(), 4);
        // insert 6
        let origin = system::RawOrigin::Signed(b.clone()).into();
        assert_ok!(MultiSig::execute(origin, addr, proposal.clone()));
        <system::Module<Test>>::inc_account_nonce(&b);

        assert_eq!(MultiSig::pending_list_for(&addr).len(), 5);
        let origin = system::RawOrigin::Signed(a.clone()).into();
        assert_err!(
            MultiSig::execute(origin, addr, proposal.clone()),
            "pending list can't be larger than MAX_PENDING"
        );
        <system::Module<Test>>::inc_account_nonce(&a);

        // remove 2
        let multi_sig_id = MultiSig::pending_list_for(&addr).get(0).unwrap().clone();
        let origin = system::RawOrigin::Signed(b.clone()).into();
        assert_ok!(MultiSig::remove_multi_sig_for(origin, addr, multi_sig_id));
        <system::Module<Test>>::inc_account_nonce(&b);
        assert_eq!(MultiSig::pending_list_for(&addr).len(), 4);

        // insert 7
        let origin = system::RawOrigin::Signed(a.clone()).into();
        assert_ok!(MultiSig::execute(origin, addr, proposal.clone()));
        <system::Module<Test>>::inc_account_nonce(&a);
    })
}

#[test]
fn test_try_to_call_root() {
    with_externalities(&mut new_test_ext(), || {
        let a: H256 = H256::repeat_byte(0x1);
        let b: H256 = H256::repeat_byte(0x2);
        let c: H256 = H256::repeat_byte(0x3);
        let d: H256 = H256::repeat_byte(0x4);
        let e: H256 = H256::repeat_byte(0x5);
        // root multisig addr
        let owners = vec![(a.clone(), true), (b.clone(), true), (c.clone(), false)];
        MultiSig::deploy_in_genesis(owners.clone(), 2);
        // no multisig addr
        let owners = vec![(a.clone(), true), (b.clone(), true), (c.clone(), false)];
        let origin = system::RawOrigin::Signed(a.clone()).into();
        // addr1
        assert_ok!(MultiSig::deploy(origin, owners, 2));

        let owners = vec![(c.clone(), true), (d.clone(), true), (e.clone(), false)];
        let origin = system::RawOrigin::Signed(a.clone()).into();
        // addr2
        assert_ok!(MultiSig::deploy(origin, owners, 2));

        let addr_genesis1 = MultiSig::multi_sig_list_item_for(&(a, 0));
        //        let addr_genesis2 = MultiSig::multi_sig_list_item_for(&(a, 1));
        let addr_a1 = MultiSig::multi_sig_list_item_for(&(a, 2));
        let addr_c1 = MultiSig::multi_sig_list_item_for(&(a, 0));

        assert_eq!(MultiSig::multisig_addr_info(&addr_genesis1).is_some(), true);
        //        assert_eq!(MultiSig::multisig_addr_info(&addr_genesis2).is_some(), true);
        assert_eq!(MultiSig::multisig_addr_info(&addr_a1).is_some(), true);
        assert_eq!(MultiSig::multisig_addr_info(&addr_c1).is_some(), true);
        // =================
        let proposal = Box::new(TestCall(true));
        let proposal2 = Box::new(TestCall(false));
        let origin = system::RawOrigin::Signed(a.clone()).into();
        assert_ok!(MultiSig::execute(origin, addr_genesis1, proposal.clone()));
        <system::Module<Test>>::inc_account_nonce(&a);
        let origin = system::RawOrigin::Signed(b.clone()).into();
        assert_ok!(MultiSig::execute(origin, addr_genesis1, proposal2.clone()));
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
            MultiSig::execute(origin, addr_genesis1, proposal.clone()),
            "it\'s not the owner"
        );
        let origin = system::RawOrigin::Signed(d.clone()).into();
        assert_err!(
            MultiSig::confirm(origin, addr_genesis1, addr_genesis1_id),
            "it\'s not the owner"
        );

        let origin = system::RawOrigin::Signed(b.clone()).into();
        assert_err!(
            MultiSig::confirm(origin, addr_genesis1, addr_genesis1_id),
            "call success"
        );
        let origin = system::RawOrigin::Signed(c.clone()).into();
        assert_err!(
            MultiSig::confirm(origin, addr_genesis1, addr_genesis1_id2),
            "call success"
        );
        // ========= genesis addr test finish

        //        let origin = system::RawOrigin::Signed(a.clone()).into();
        //        assert_ok!(MultiSig::execute(origin, addr_a1, proposal.clone()));
        //        <system::Module<Test>>::inc_account_nonce(&a);
        //        let origin = system::RawOrigin::Signed(b.clone()).into();
        //        assert_ok!(MultiSig::execute(origin, addr_a1, proposal2.clone()));
        //        <system::Module<Test>>::inc_account_nonce(&b);
        //        let addr_a1_id = MultiSig::pending_list_for(&addr_a1).get(0).unwrap().clone();
        //        let addr_a1_id2 = MultiSig::pending_list_for(&addr_a1).get(1).unwrap().clone();
        //
        //        let origin = system::RawOrigin::Signed(d.clone()).into();
        //        assert_err!(MultiSig::execute(origin, addr_a1, proposal.clone()), "it\'s not the owner");
        //        let origin = system::RawOrigin::Signed(d.clone()).into();
        //        assert_err!(MultiSig::confirm(origin, addr_a1, addr_a1_id), "it\'s not the owner");
        //
        //        let origin = system::RawOrigin::Signed(b.clone()).into();
        //        assert_err!(MultiSig::confirm(origin, addr_a1, addr_a1_id), "bad origin: expected to be a root origin");
        //        let origin = system::RawOrigin::Signed(c.clone()).into();
        //        assert_err!(MultiSig::confirm(origin, addr_a1, addr_a1_id2), "call success");
    })
}

#[test]
fn test_deploy() {
    with_externalities(&mut new_test_ext(), || {
        //        let a: H256 = H256::repeat_byte(0x1);
        //        let b: H256 = H256::repeat_byte(0x2);
        //        let c: H256 = H256::repeat_byte(0x3);
        //        let d: H256 = H256::repeat_byte(0x4);
        //        let e: H256 = H256::repeat_byte(0x5);
        //        // root multisig addr
        //        let owners = vec![(a.clone(), true), (b.clone(), true), (c.clone(), false)];
        //        MultiSig::deploy_in_genesis(owners.clone(), 2);
        //        // no multisig addr
        //        let owners = vec![(a.clone(), true), (b.clone(), true), (c.clone(), false)];
        //        let origin = system::RawOrigin::Signed(a.clone()).into();

    })
}
