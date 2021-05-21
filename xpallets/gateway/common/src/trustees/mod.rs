// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

pub mod bitcoin;

use frame_support::{
    dispatch::DispatchError,
    log::{error, warn},
    traits::SortedMembers,
};
use sp_std::{convert::TryFrom, marker::PhantomData, prelude::*};

use xpallet_assets::Chain;
use xpallet_support::traits::MultiSig;

use crate::traits::{BytesLike, ChainProvider, TrusteeSession};
use crate::types::TrusteeSessionInfo;
use crate::{Config, Error, Module};

pub struct TrusteeSessionManager<T: Config, TrusteeAddress>(
    PhantomData<T>,
    PhantomData<TrusteeAddress>,
);

impl<T: Config, TrusteeAddress: BytesLike + ChainProvider>
    TrusteeSession<T::AccountId, TrusteeAddress> for TrusteeSessionManager<T, TrusteeAddress>
{
    fn trustee_session(
        number: u32,
    ) -> Result<TrusteeSessionInfo<T::AccountId, TrusteeAddress>, DispatchError> {
        let chain = TrusteeAddress::chain();
        let generic_info =
            Module::<T>::trustee_session_info_of(chain, number).ok_or_else(|| {
                error!(
                    target: "runtime::gateway::common",
                    "[trustee_session] Can not find session info, chain:{:?}, number:{}",
                    chain,
                    number
                );
                Error::<T>::InvalidTrusteeSession
            })?;
        let info = TrusteeSessionInfo::<T::AccountId, TrusteeAddress>::try_from(generic_info)
            .map_err(|_| Error::<T>::InvalidGenericData)?;
        Ok(info)
    }

    fn current_trustee_session(
    ) -> Result<TrusteeSessionInfo<T::AccountId, TrusteeAddress>, DispatchError> {
        let chain = TrusteeAddress::chain();
        let number = match Module::<T>::trustee_session_info_len(chain).checked_sub(1) {
            Some(r) => r,
            None => u32::max_value(),
        };
        Self::trustee_session(number)
    }

    fn last_trustee_session(
    ) -> Result<TrusteeSessionInfo<T::AccountId, TrusteeAddress>, DispatchError> {
        let chain = TrusteeAddress::chain();
        let number = match Module::<T>::trustee_session_info_len(chain).checked_sub(2) {
            Some(r) => r,
            None => u32::max_value(),
        };
        Self::trustee_session(number).map_err(|err| {
            warn!(
                target: "runtime::gateway::common",
                "[last_trustee_session] Last trustee session not exist yet for chain:{:?}",
                chain
            );
            err
        })
    }

    #[cfg(feature = "std")]
    fn genesis_trustee(chain: Chain, trustees: &[T::AccountId]) {
        Module::<T>::transition_trustee_session_impl(chain, trustees.to_vec())
            .expect("trustee session transition can not fail; qed");
    }
}

pub struct TrusteeMultisigProvider<T: Config, C: ChainProvider>(PhantomData<T>, PhantomData<C>);
impl<T: Config, C: ChainProvider> TrusteeMultisigProvider<T, C> {
    pub fn new() -> Self {
        TrusteeMultisigProvider::<_, _>(Default::default(), Default::default())
    }
}

impl<T: Config, C: ChainProvider> MultiSig<T::AccountId> for TrusteeMultisigProvider<T, C> {
    fn multisig() -> T::AccountId {
        Module::<T>::trustee_multisig_addr(C::chain())
    }
}

impl<T: Config, C: ChainProvider> SortedMembers<T::AccountId> for TrusteeMultisigProvider<T, C> {
    fn sorted_members() -> Vec<T::AccountId> {
        vec![Self::multisig()]
    }
}
