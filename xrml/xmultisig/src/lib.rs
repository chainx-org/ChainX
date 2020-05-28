// Copyright 2018-2019 Chainpool.

//! this module is for multisig, but now this is just for genesis multisig addr, not open for public.

#![allow(clippy::boxed_local)]
#![cfg_attr(not(feature = "std"), no_std)]

mod mock;
mod tests;
pub mod types;

use parity_codec::Encode;

// Substrate
use primitives::traits::Hash;
use rstd::{marker::PhantomData, prelude::*, result};
use substrate_primitives::crypto::UncheckedFrom;
use support::{
    decl_event, decl_module, decl_storage, dispatch::Result, Dispatchable, Parameter, StorageMap,
};
use system::ensure_signed;

use xsupport::{debug, error, info};

pub use self::types::{AddrInfo, AddrType, MultiSigPermission, PendingState};

// MAX_OWNERS equal PendingState.owners_done bits length
const MAX_OWNERS: u32 = 64;
const MAX_PENDING: u32 = 5;

pub trait MultiSigFor<AccountId: Sized, Hash: Sized> {
    /// generate multisig addr for a accountid
    fn multi_sig_addr_for(who: &AccountId) -> AccountId;

    fn multi_sig_id_for(who: &AccountId, addr: &AccountId, data: &[u8]) -> Hash;
}

pub trait GenesisMultiSig<AccountId> {
    fn gen_genesis_multisig() -> (AccountId, AccountId);
}

pub trait LimitedCall<AccountId> {
    fn allow(&self) -> bool;
    fn exec(&self, exerciser: &AccountId) -> Result;
}

pub trait Trait: xaccounts::Trait {
    type MultiSig: MultiSigFor<Self::AccountId, Self::Hash>;
    type GenesisMultiSig: GenesisMultiSig<Self::AccountId>;
    type Proposal: Parameter + Dispatchable<Origin = Self::Origin>;
    type TrusteeCall: LimitedCall<Self::AccountId> + From<Self::Proposal>;
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

//impl trait
/// Simple MultiSigIdFor struct
pub struct SimpleMultiSigIdFor<T: Trait>(PhantomData<T>);
impl<T: Trait> MultiSigFor<T::AccountId, T::Hash> for SimpleMultiSigIdFor<T>
where
    T::AccountId: UncheckedFrom<T::Hash> + AsRef<[u8]>,
{
    fn multi_sig_addr_for(who: &T::AccountId) -> T::AccountId {
        let mut buf = Vec::<u8>::new();
        buf.extend_from_slice(who.as_ref());
        buf.extend_from_slice(&<system::Module<T>>::account_nonce(who).encode());
        buf.extend_from_slice(&<Module<T>>::multi_sig_list_len_for(who).encode()); // in case same nonce in genesis
        UncheckedFrom::unchecked_from(T::Hashing::hash(&buf[..]))
    }

    fn multi_sig_id_for(who: &T::AccountId, addr: &T::AccountId, data: &[u8]) -> T::Hash {
        let mut buf = Vec::<u8>::new();
        buf.extend_from_slice(who.as_ref());
        buf.extend_from_slice(addr.as_ref());
        buf.extend_from_slice(&<system::Module<T>>::account_nonce(who).encode());
        buf.extend_from_slice(data);
        T::Hashing::hash(&buf[..])
    }
}

pub struct ChainXGenesisMultisig<T: Trait>(PhantomData<T>);
impl<T: Trait> GenesisMultiSig<T::AccountId> for ChainXGenesisMultisig<T>
where
    T::AccountId: UncheckedFrom<T::Hash> + AsRef<[u8]>,
{
    fn gen_genesis_multisig() -> (T::AccountId, T::AccountId) {
        let team_multisig_addr: T::AccountId =
            UncheckedFrom::unchecked_from(T::Hashing::hash(&b"Team"[..]));
        let council_multisig_addr: T::AccountId =
            UncheckedFrom::unchecked_from(T::Hashing::hash(&b"Council"[..]));
        (team_multisig_addr, council_multisig_addr)
    }
}

decl_event!(
    pub enum Event<T> where
        <T as system::Trait>::AccountId,
        <T as system::Trait>::Hash,
        <T as Trait>::Proposal
    {
        /// deploy a multisig and get address, who deploy, deploy addr, owners num, required num
        DeployMultiSig(AccountId, AccountId, u32, u32),
        /// exec. who, addr, multisigid, type
        ExecMultiSig(AccountId, AccountId, Hash, Box<Proposal>),
        /// confirm. addr, multisigid, yet_needed, owners_done
        Confirm(AccountId, Hash, u32, u64),

        /// remove multisig id for a multisig addr
        RemoveMultiSigIdFor(AccountId, Hash),
    }
);

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event<T>() = default;

        // not open for public
        // fn deploy(origin, owners: Vec<(T::AccountId, bool)>, required_num: u32) -> Result {
        //     let from = ensure_signed(origin)?;
        //     let multisig_addr: T::AccountId = T::MultiSig::multi_sig_addr_for(&from);
        //     debug!("[deploy]|deploy for new mutisig addr|who:{:?}|new multisig addr:{:?}|required:{:}|owners:{:?}", from, multisig_addr, required_num, owners);
        //     Self::deploy_impl(AddrType::Normal, &multisig_addr, &from, owners, required_num)
        // }

        fn execute(origin, multi_sig_addr: T::AccountId, proposal: Box<T::Proposal>) -> Result {
            let from: T::AccountId = ensure_signed(origin)?;
            debug!("[execute]|create a proposal|who:{:?}|for addr:{:?}|proposal:{:?}", from, multi_sig_addr, proposal);
            Self::execute_impl(&from, &multi_sig_addr, proposal)
        }

        fn confirm(origin, multi_sig_addr: T::AccountId, multi_sig_id: T::Hash) -> Result {
            let from = ensure_signed(origin)?;
            debug!("[execute]|confirm for a proposal|who:{:?}|for addr:{:?}|multi_sig_id:{:}", from, multi_sig_addr, multi_sig_id);
            Self::confirm_impl(&from, &multi_sig_addr, multi_sig_id)
        }

        // remove multisig addr
        fn remove_multi_sig_for(origin, multi_sig_addr: T::AccountId, multi_sig_id: T::Hash) -> Result {
            let from: T::AccountId = ensure_signed(origin)?;
            Self::only_owner(&from, &multi_sig_addr, true)?;
            debug!("[remove_multi_sig]|remove a proposal|who:{:?}|for addr:{:?}|multi_sig_id:{:?}", from, multi_sig_addr, multi_sig_id);
            Self::remove_multi_sig_id(&multi_sig_addr, multi_sig_id);
            Ok(())
        }

        /// transition current owners to other group for the multisig addr.
        /// this call can't be called from user directly, only allow call from `execute` proposal.
        fn transition(origin, owners: Vec<(T::AccountId, bool)>, required_num: u32) -> Result {
            let multi_sig_addr = ensure_signed(origin)?;
            if owners.is_empty() {
                return Err("owners can't be empty.");
            }

            let addr_info = Self::multisig_addr_info(&multi_sig_addr).ok_or("multisig address not exist.")?;

            let owners = owners.into_iter().map(|(a, permission)| {
                let p = if permission {
                    MultiSigPermission::ConfirmAndPropose
                } else {
                    MultiSigPermission::ConfirmOnly
                };
                (a, p)
            }).collect::<Vec<_>>();

            let deploy = owners[0].0.clone();
            Self::deploy_impl(addr_info.addr_type, &multi_sig_addr, &deploy, owners, required_num)
        }
    }
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
    ) -> result::Result<u32, &'static str> {
        if let Some(addr_info) = Self::multisig_addr_info(addr) {
            for (index, (id, req)) in addr_info.owner_list.iter().enumerate() {
                if id == who {
                    if required && (*req == MultiSigPermission::ConfirmOnly) {
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
    ) -> result::Result<bool, &'static str> {
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
    ) -> result::Result<u32, &'static str> {
        Self::is_owner(who, addr, required)
    }
    fn only_many_owner(
        who: &T::AccountId,
        multi_sig_addr: &T::AccountId,
        multi_sig_id: T::Hash,
    ) -> result::Result<bool, &'static str> {
        Self::confirm_and_check(who, multi_sig_addr, multi_sig_id)
    }
}

impl<T: Trait> Module<T> {
    pub fn deploy_impl(
        addr_type: AddrType,
        multi_addr: &T::AccountId,
        deployer: &T::AccountId,
        owners: Vec<(T::AccountId, MultiSigPermission)>,
        required_num: u32,
    ) -> Result {
        let mut owner_list = Vec::new();
        // confirm owners has deployer
        owner_list.push((deployer.clone(), MultiSigPermission::ConfirmAndPropose));
        // move others people except deployer
        owner_list.extend(owners.into_iter().filter(|info| info.0 != *deployer));

        let owners_len = owner_list.len() as u32;
        if owners_len > MAX_OWNERS {
            error!("[deploy_impl]|total owners count can't more than `MAX_OWNERS`|owners_len:{:}|MAX_OWNERS:{:?}", owners_len, MAX_OWNERS);
            return Err("total owners can't more than `MAX_OWNERS`");
        }

        if owners_len < required_num {
            error!(
                "[deploy_impl]|owners count can't less than required num|owners_len:{:}|required_num:{:?}",
                owners_len, required_num
            );
            return Err("owners count can't less than required num");
        }

        // 1, set multi_addr for current deployer. notice when transition for same deployer, the
        // multisiglist would log duplicate multisig addr. removing duplicate is meaningless
        // for duplicate can imply different session of transition
        let len = Self::multi_sig_list_len_for(deployer);
        <MultiSigListItemFor<T>>::insert(&(deployer.clone(), len), multi_addr.clone());
        <MultiSigListLenFor<T>>::insert(deployer.clone(), len + 1); // length inc

        let addr_info = AddrInfo::<T::AccountId> {
            addr_type,
            required_num,
            owner_list,
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

    pub fn deploy_impl_unsafe(
        addr_type: AddrType,
        multi_addr: &T::AccountId,
        deployer: &T::AccountId,
        owners: Vec<(T::AccountId, MultiSigPermission)>,
        required_num: u32,
    ) {
        let _ = Self::deploy_impl(addr_type, multi_addr, deployer, owners, required_num)
            .map_err(|e| panic!(e));
    }

    #[cfg(feature = "std")]
    pub fn deploy_in_genesis(
        team: Vec<(T::AccountId, MultiSigPermission)>,
        team_required_num: u32,
        council: Vec<(T::AccountId, MultiSigPermission)>,
        council_required_num: u32,
    ) -> result::Result<T::AccountId, &'static str> {
        use support::StorageValue;

        if team.is_empty() || council.is_empty() {
            error!(
                "[deploy_in_genesis]|the team:{:?} and council:{:?} count can't be zero",
                team.len(),
                council.len()
            );
            panic!("the owners count can't be zero");
        }
        let team_deployer = team.get(0).unwrap().clone().0;

        let (team_multisig_addr, council_multisig_addr) =
            T::GenesisMultiSig::gen_genesis_multisig();

        Self::deploy_impl(
            AddrType::Normal,
            &team_multisig_addr,
            &team_deployer,
            team,
            team_required_num,
        )?;

        let council_deployer = council.get(0).unwrap().clone().0;
        Self::deploy_impl(
            AddrType::Root,
            &council_multisig_addr,
            &council_deployer,
            council,
            council_required_num,
        )?;

        RootAddrList::<T>::put(vec![council_multisig_addr.clone()]);

        xaccounts::TeamAccount::<T>::put(&team_multisig_addr);
        xaccounts::CouncilAccount::<T>::put(council_multisig_addr);

        Ok(team_multisig_addr)
    }

    #[allow(clippy::borrowed_box)]
    fn check_proposal(addr_info: &AddrInfo<T::AccountId>, proposal: &Box<T::Proposal>) -> Result {
        match addr_info.addr_type {
            AddrType::Trustee => {
                // it's an useless clone, but do not have other way
                let p: T::TrusteeCall = (**proposal).clone().into();
                if p.allow() {
                    Ok(())
                } else {
                    error!("[check_proposal]|do not allow trustee multisig addr to call this proposal|proposal:{:?}", proposal);
                    Err("do not allow trustee multisig addr to call this proposal")
                }
            }
            _ => Ok(()),
        }
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
            Self::check_proposal(&info, &proposal)?;

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

                debug!("[execute_impl]|multi_sig_addr:{:?}|new proposal multisig id:{:?}|current all multisig:{:?}", multi_sig_addr, multi_sig_id, pending_list);

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
        let ret = Self::only_many_owner(&from, &multi_sig_addr, multi_sig_id)?;

        let pending_state = if let Some(pending_state) =
            Self::pending_state_for(&(multi_sig_addr.clone(), multi_sig_id))
        {
            pending_state
        } else {
            return Err("no pending state for this addr and id or it has finished");
        };

        debug!("[confirm_impl]|from:{:?}|foraddr:{:?}|multisig id:{:}|ret:{:}|yet_needed:{:?} and owners_down:{:?}", from, multi_sig_addr, multi_sig_id, ret, pending_state.yet_needed, pending_state.owners_done);

        if ret {
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
            debug!(
                "[exec]|real exec|addr:{:?}|addr_type:{:?}",
                addr, info.addr_type
            );
            match info.addr_type {
                AddrType::Normal => Self::exec_tx(addr, proposal),
                AddrType::Root => Self::exec_tx_byroot(addr, proposal),
                AddrType::Trustee => Self::exec_tx_bytrustee(addr, proposal),
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

    fn exec_tx_bytrustee(addr: &T::AccountId, proposal: Box<T::Proposal>) -> Result {
        info!(
            "[exec_tx_bytrustee]|real exec|addr:{:?}|proposal:{:?}",
            addr, proposal
        );
        let p: T::TrusteeCall = (*proposal).into();
        p.exec(addr)
    }
}
