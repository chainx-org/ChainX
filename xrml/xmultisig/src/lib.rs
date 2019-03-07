// Copyright 2018 Chainpool.

//! this module is for multisig, but now this is just for genesis multisig addr, not open for public.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
use serde_derive::{Deserialize, Serialize};

use parity_codec::{Decode, Encode};

use sr_std::marker::PhantomData;
use sr_std::prelude::*;
use sr_std::result::Result as StdResult;

use sr_primitives::traits::Hash;
use srml_support::dispatch::Result;
use srml_support::{decl_event, decl_module, decl_storage, Dispatchable, Parameter, StorageMap};

use srml_balances as balances;
use srml_system as system;
use system::ensure_signed;

use xrml_xsupport::error;

#[cfg(test)]
mod tests;

pub trait MultiSigFor<AccountId: Sized, Hash: Sized> {
    /// generate multisig addr for a accountid
    fn multi_sig_addr_for(who: &AccountId) -> AccountId;

    fn multi_sig_id_for(who: &AccountId, addr: &AccountId, data: &[u8]) -> Hash;
}

pub trait GenesisMultiSig<AccountId> {
    fn gen_genesis_multisig(accounts: Vec<AccountId>) -> (AccountId, AccountId);
}

pub trait Trait: balances::Trait {
    type MultiSig: MultiSigFor<Self::AccountId, Self::Hash>;
    type GenesisMultiSig: GenesisMultiSig<Self::AccountId>;
    type Proposal: Parameter + Dispatchable<Origin = Self::Origin>;
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

decl_event!(
    pub enum Event<T> where
        <T as system::Trait>::AccountId,
        <T as system::Trait>::Hash,
        <T as balances::Trait>::Balance,
        <T as Trait>::Proposal
    {
        /// deploy a multisig and get address, who deploy, deploy addr, owners num, required num
        DeployMultiSig(AccountId, AccountId, u32, u32),
        /// exec. who, addr, multisigid, type
        ExecMultiSig(AccountId, AccountId, Hash, Box<Proposal>),
        /// confirm. addr, multisigid, yet_needed, owners_done
        Confirm(AccountId, Hash, u32, u32),

        /// remove multisig id for a multisig addr
        RemoveMultiSigIdFor(AccountId, Hash),

        /// set deploy fee, by Root
        SetDeployFee(Balance),
        /// set exec fee, by Root
        SetExecFee(Balance),
        /// set confirm fee, by Root
        SetConfirmFee(Balance),
    }
);

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event<T>() = default;

        fn deploy(origin, owners: Vec<(T::AccountId, bool)>, required_num: u32) -> Result {
            let from = ensure_signed(origin)?;
            let multisig_addr: T::AccountId = T::MultiSig::multi_sig_addr_for(&from);
            Self::deploy_impl(false, &multisig_addr, &from, owners, required_num)
        }

        fn execute(origin, multi_sig_addr: T::AccountId, proposal: Box<T::Proposal>) -> Result {
            let from: T::AccountId = ensure_signed(origin)?;
            Self::execute_impl(&from, &multi_sig_addr, proposal)
        }
        fn confirm(origin, multi_sig_addr: T::AccountId, multi_sig_id: T::Hash) -> Result {
            let from = ensure_signed(origin)?;
            Self::confirm_impl(&from, &multi_sig_addr, multi_sig_id)
        }
        fn is_owner_for(origin, multi_sig_addr: T::AccountId) -> Result {
            let from = ensure_signed(origin)?;
            Self::is_owner(&from, &multi_sig_addr, false).map(|_| ())
        }
        // remove multisig addr
        fn remove_multi_sig_for(origin, multi_sig_addr: T::AccountId, multi_sig_id: T::Hash) -> Result {
            let from: T::AccountId = ensure_signed(origin)?;
            Self::only_owner(&from, &multi_sig_addr, true)?;

            Self::remove_multi_sig_id(&multi_sig_addr, multi_sig_id);
            Ok(())
        }
    }
}

const MAX_OWNERS: u32 = 32;
const MAX_PENDING: u32 = 5;

#[derive(PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub struct AddrInfo<AccountId> {
    is_root: bool,
    required_num: u32,
    owner_list: Vec<(AccountId, bool)>,
}

// struct for the status of a pending operation.
#[derive(PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub struct PendingState<Proposal> {
    yet_needed: u32,
    owners_done: u32,
    proposal: Box<Proposal>,
}

decl_storage! {
    trait Store for Module<T: Trait> as XMultiSig {
        pub RootAddrList get(root_addr_list): Vec<T::AccountId>;

        pub MultiSigAddrInfo get(multisig_addr_info): map T::AccountId => Option<AddrInfo<T::AccountId>>;

        pub PendingListFor get(pending_list_for): map T::AccountId => Vec<T::Hash>;
        pub PendingStateFor get(pending_state_for): map (T::AccountId, T::Hash) => Option<PendingState<T::Proposal>>;

        // for deployer
        pub MultiSigListItemFor get(multi_sig_list_item_for): map (T::AccountId, u32) => T::AccountId;
        pub MultiSigListLenFor get(multi_sig_list_len_for): map T::AccountId => u32;
    }
}

//impl trait
/// Simple MultiSigIdFor struct
pub struct SimpleMultiSigIdFor<T: Trait>(PhantomData<T>);

impl<T: Trait> MultiSigFor<T::AccountId, T::Hash> for SimpleMultiSigIdFor<T>
where
    T::AccountId: From<T::Hash>,
{
    fn multi_sig_addr_for(who: &T::AccountId) -> T::AccountId {
        let mut buf = Vec::<u8>::new();
        buf.extend_from_slice(&who.encode());
        buf.extend_from_slice(&<system::Module<T>>::account_nonce(who).encode());
        buf.extend_from_slice(&<Module<T>>::multi_sig_list_len_for(who).encode()); // in case same nonce in genesis
        T::Hashing::hash(&buf[..]).into()
    }

    fn multi_sig_id_for(who: &T::AccountId, addr: &T::AccountId, data: &[u8]) -> T::Hash {
        let mut buf = Vec::<u8>::new();
        buf.extend_from_slice(&who.encode());
        buf.extend_from_slice(&addr.encode());
        buf.extend_from_slice(&<system::Module<T>>::account_nonce(who).encode());
        buf.extend_from_slice(data);
        T::Hashing::hash(&buf[..])
    }
}

pub struct ChainXGenesisMultisig<T: Trait>(PhantomData<T>);
impl<T: Trait> GenesisMultiSig<T::AccountId> for ChainXGenesisMultisig<T>
where
    T::AccountId: From<T::Hash> + AsRef<[u8]>,
{
    fn gen_genesis_multisig(accounts: Vec<T::AccountId>) -> (T::AccountId, T::AccountId) {
        let mut buf = Vec::<u8>::new();
        for a in accounts.iter() {
            buf.extend_from_slice(a.as_ref());
        }
        let team_multisig_addr: T::AccountId = T::Hashing::hash(&buf[..]).into();
        let council_multisig_addr: T::AccountId = T::Hashing::hash(&b"Council"[..]).into();
        (team_multisig_addr, council_multisig_addr)
    }
}

impl<T: Trait> Module<T> {
    //    fn remove_multi_sig_addr(multi_sig_addr: &T::AccountId) {
    //    }

    fn remove_multi_sig_id(multi_sig_addr: &T::AccountId, multi_sig_id: T::Hash) {
        Self::remove_pending_for(multi_sig_addr, multi_sig_id);
        PendingListFor::<T>::mutate(multi_sig_addr, |v| {
            v.retain(|x| x != &multi_sig_id);
        });
        // event
        Self::deposit_event(RawEvent::RemoveMultiSigIdFor(
            multi_sig_addr.clone(),
            multi_sig_id,
        ));
    }

    fn remove_pending_for(multi_sig_addr: &T::AccountId, multi_sig_id: T::Hash) {
        PendingStateFor::<T>::remove((multi_sig_addr.clone(), multi_sig_id))
    }

    fn is_owner(
        who: &T::AccountId,
        addr: &T::AccountId,
        required: bool,
    ) -> StdResult<u32, &'static str> {
        if let Some(addr_info) = Self::multisig_addr_info(addr) {
            for (index, (id, req)) in addr_info.owner_list.iter().enumerate() {
                if id == who {
                    if required && (*req == false) {
                        return Err("it's the owner but not required owner");
                    } else {
                        return Ok(index as u32);
                    }
                }
            }
        } else {
            return Err("the multi sig addr not exist");
        }
        Err("it's not the owner")
    }

    fn confirm_and_check(
        who: &T::AccountId,
        multi_sig_addr: &T::AccountId,
        multi_sig_id: T::Hash,
    ) -> StdResult<bool, &'static str> {
        let index = Self::is_owner(who, multi_sig_addr, false)?;

        let mut pending = if let Some(pending) =
            Self::pending_state_for(&(multi_sig_addr.clone(), multi_sig_id))
        {
            pending
        } else {
            return Err("pending state not exist");
        };

        let ret: bool;

        let index_bit = 1 << index; // not longer then index_bit's type
        if pending.owners_done & index_bit == 0 {
            if pending.yet_needed <= 1 {
                // enough confirmations
                ret = true;
            } else {
                pending.yet_needed -= 1;
                pending.owners_done |= index_bit;
                // update pending state
                PendingStateFor::<T>::insert(&(multi_sig_addr.clone(), multi_sig_id), pending);
                ret = false;
            }
        } else {
            return Err("this account has confirmed for this multi sig addr and id");
        }
        Ok(ret)
    }

    // func alias
    fn only_owner(
        who: &T::AccountId,
        addr: &T::AccountId,
        required: bool,
    ) -> StdResult<u32, &'static str> {
        Self::is_owner(who, addr, required)
    }
    fn only_many_owner(
        who: &T::AccountId,
        multi_sig_addr: &T::AccountId,
        multi_sig_id: T::Hash,
    ) -> StdResult<bool, &'static str> {
        Self::confirm_and_check(who, multi_sig_addr, multi_sig_id)
    }
}

impl<T: Trait> Module<T> {
    fn deploy_impl(
        root: bool,
        multi_addr: &T::AccountId,
        deployer: &T::AccountId,
        owners: Vec<(T::AccountId, bool)>,
        required_num: u32,
    ) -> Result {
        let mut owner_list = Vec::new();
        owner_list.push((deployer.clone(), true));
        owner_list.extend(owners.into_iter().filter(|info| {
            if info.0 == *deployer {
                false
            } else {
                true
            }
        }));

        let owners_len = owner_list.len() as u32;
        if owners_len > MAX_OWNERS {
            return Err("total owners can't more than `MAX_OWNERS`");
        }

        if owners_len < required_num {
            return Err("owners count can't less than required num");
        }

        // 1
        let len = Self::multi_sig_list_len_for(deployer);
        <MultiSigListItemFor<T>>::insert(&(deployer.clone(), len), multi_addr.clone());
        <MultiSigListLenFor<T>>::insert(deployer.clone(), len + 1); // length inc

        let addr_info = AddrInfo::<T::AccountId> {
            is_root: root,
            required_num,
            owner_list: owner_list,
        };
        // 2
        MultiSigAddrInfo::<T>::insert(multi_addr, addr_info);
        // event
        Self::deposit_event(RawEvent::DeployMultiSig(
            deployer.clone(),
            multi_addr.clone(),
            owners_len,
            required_num,
        ));
        Ok(())
    }

    #[cfg(feature = "std")]
    pub fn deploy_in_genesis(
        owners: Vec<(T::AccountId, bool)>,
        required_num: u32,
    ) -> (T::AccountId, T::AccountId) {
        use srml_support::StorageValue;
        if owners.len() < 1 {
            panic!("the owners count can't be zero");
        }
        let deployer = owners.get(0).unwrap().clone().0;

        let (team_multisig_addr, council_multisig_addr) = T::GenesisMultiSig::gen_genesis_multisig(
            owners.iter().map(|(a, _)| a.clone()).collect(),
        );

        let _ = Self::deploy_impl(
            true,
            &team_multisig_addr,
            &deployer,
            owners.clone(),
            required_num,
        );
        let _ = Self::deploy_impl(
            true,
            &council_multisig_addr,
            &deployer,
            owners.clone(),
            required_num,
        );

        RootAddrList::<T>::put(vec![
            team_multisig_addr.clone(),
            council_multisig_addr.clone(),
        ]);
        return (team_multisig_addr, council_multisig_addr);
    }

    fn execute_impl(
        from: &T::AccountId,
        multi_sig_addr: &T::AccountId,
        proposal: Box<T::Proposal>,
    ) -> Result {
        Self::only_owner(&from, &multi_sig_addr, true)?;

        let mut pending_list = Self::pending_list_for(multi_sig_addr);
        if pending_list.len() as u32 >= MAX_PENDING {
            return Err("pending list can't be larger than MAX_PENDING");
        }

        if let Some(info) = Self::multisig_addr_info(multi_sig_addr) {
            let proposal_event = proposal.clone();
            let multi_sig_id: T::Hash;
            if info.required_num <= 1 {
                // real exec
                Self::exec(&multi_sig_addr, proposal)?;
                multi_sig_id = Default::default();
            } else {
                // determine multi sig id
                multi_sig_id =
                    T::MultiSig::multi_sig_id_for(&from, &multi_sig_addr, &proposal.encode());
                let pending = PendingState::<T::Proposal> {
                    yet_needed: info.required_num,
                    owners_done: 0,
                    proposal,
                };
                pending_list.push(multi_sig_id);

                PendingStateFor::<T>::insert(&(multi_sig_addr.clone(), multi_sig_id), pending);
                PendingListFor::<T>::insert(multi_sig_addr, pending_list);

                // confirm for self
                let origin = system::RawOrigin::Signed(from.clone()).into();
                Self::confirm(origin, multi_sig_addr.clone(), multi_sig_id)?;
            }
            Self::deposit_event(RawEvent::ExecMultiSig(
                from.clone(),
                multi_sig_addr.clone(),
                multi_sig_id,
                proposal_event,
            ));
        } else {
            return Err("the multi sig addr not exist");
        }

        Ok(())
    }

    fn confirm_impl(
        from: &T::AccountId,
        multi_sig_addr: &T::AccountId,
        multi_sig_id: T::Hash,
    ) -> Result {
        // TODO renew
        let ret = Self::only_many_owner(&from, &multi_sig_addr, multi_sig_id)?;

        let pending_state = if let Some(pending_state) =
            Self::pending_state_for(&(multi_sig_addr.clone(), multi_sig_id))
        {
            pending_state
        } else {
            return Err("no pending state for this addr and id or it has finished");
        };

        if ret == true {
            // remove log
            Self::remove_multi_sig_id(&multi_sig_addr, multi_sig_id);
            // real exec
            Self::exec(&multi_sig_addr, pending_state.proposal)?;
        } else {
            // log event
            Self::deposit_event(RawEvent::Confirm(
                multi_sig_addr.clone(),
                multi_sig_id,
                pending_state.yet_needed,
                pending_state.owners_done,
            ));
        }

        Ok(())
    }
}

impl<T: Trait> Module<T> {
    fn exec(addr: &T::AccountId, proposal: Box<T::Proposal>) -> Result {
        if let Some(info) = Self::multisig_addr_info(addr) {
            if info.is_root {
                Self::exec_tx_byroot(addr, proposal)
            } else {
                Self::exec_tx(addr, proposal)
            }
        } else {
            Err("addr info not exist")
        }
    }

    fn exec_tx(addr: &T::AccountId, proposal: Box<T::Proposal>) -> Result {
        let origin = system::RawOrigin::Signed(addr.clone()).into();
        proposal.dispatch(origin)
    }

    fn exec_tx_byroot(addr: &T::AccountId, proposal: Box<T::Proposal>) -> Result {
        // use signed to exec first, if failed, use root
        if let Err(e) = Self::exec_tx(addr, proposal.clone()) {
            if e == "bad origin: expected to be a root origin" {
                let white_list = Self::root_addr_list();
                if white_list.contains(addr) {
                    let origin = system::RawOrigin::Root.into();
                    return proposal.dispatch(origin);
                } else {
                    error!("this addr[{:}] not in white_list! can't exec root tx", addr);
                    return Err("this addr not in white_list! can't exec root tx");
                }
            }
            return Err(e);
        }

        Ok(())
    }
}
