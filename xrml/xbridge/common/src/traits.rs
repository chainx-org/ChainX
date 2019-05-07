// Copyright 2018-2019 Chainpool.

use rstd::{prelude::Vec, result};

use primitives::traits::MaybeDebug;

use xr_primitives::Name;
use xsupport::error;

use crate::types::{TrusteeInfoConfig, TrusteeIntentionProps, TrusteeSessionInfo};

pub trait Extractable<AccountId> {
    fn account_info(data: &[u8]) -> Option<(AccountId, Option<Name>)>;
}

pub trait TrusteeForChain<AccountId, TrusteeEntity: IntoVecu8, TrusteeAddress: IntoVecu8> {
    fn check_trustee_entity(raw_addr: &[u8]) -> result::Result<TrusteeEntity, &'static str>;

    fn generate_trustee_session_info(
        props: Vec<(AccountId, TrusteeIntentionProps<TrusteeEntity>)>,
        config: TrusteeInfoConfig,
    ) -> result::Result<TrusteeSessionInfo<AccountId, TrusteeAddress>, &'static str>;
}

pub trait TrusteeSession<AccountId, TrusteeAddress: IntoVecu8> {
    fn current_trustee_session(
    ) -> result::Result<TrusteeSessionInfo<AccountId, TrusteeAddress>, &'static str>;

    fn last_trustee_session(
    ) -> result::Result<TrusteeSessionInfo<AccountId, TrusteeAddress>, &'static str>;
}

pub trait TrusteeMultiSig<AccountId: PartialEq + MaybeDebug> {
    fn multisig_for_trustees() -> AccountId;

    fn check_multisig(who: &AccountId) -> result::Result<(), &'static str> {
        let current_multisig_addr = Self::multisig_for_trustees();
        if current_multisig_addr != *who {
            error!("[check_multisig]|the account not match current trustee multisig addr for this chain|current:{:?}|who:{:?}", current_multisig_addr, who);
            return Err("the account not match current trustee multisig addr for this chain");
        }
        Ok(())
    }
}

pub trait IntoVecu8 {
    fn into_vecu8(self) -> Vec<u8>;
    fn from_vecu8(src: &[u8]) -> Option<Self>
    where
        Self: Sized;
}

impl IntoVecu8 for Vec<u8> {
    fn into_vecu8(self) -> Vec<u8> {
        self
    }
    fn from_vecu8(src: &[u8]) -> Option<Self> {
        Some(src.to_vec())
    }
}

impl IntoVecu8 for [u8; 20] {
    fn into_vecu8(self) -> Vec<u8> {
        self.to_vec()
    }

    fn from_vecu8(src: &[u8]) -> Option<Self> {
        if src.len() != 20 {
            return None;
        }
        let mut a: [u8; 20] = Default::default();
        let len = a.len();
        a.copy_from_slice(&src[..len]);
        Some(a)
    }
}

pub trait CrossChainBinding<AccountId, Address> {
    fn update_binding(who: &AccountId, addr: Address, channel_name: Option<Name>);
    /// return accountid, and option channel name
    fn get_binding_info(input_addr: &Address) -> Option<(AccountId, Option<AccountId>)>;
}

pub trait AsRefAndMutOption<T> {
    fn as_ref(&self) -> Option<&T>;
    fn as_mut(&mut self) -> Option<&mut T>;
}

impl<T> AsRefAndMutOption<T> for Option<T> {
    fn as_ref(&self) -> Option<&T> {
        self.as_ref()
    }

    fn as_mut(&mut self) -> Option<&mut T> {
        self.as_mut()
    }
}
