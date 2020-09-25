// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

pub mod bitcoin;

use codec::{Decode, Encode, Error as CodecError};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
// Substrate
use frame_support::dispatch::DispatchError;
use sp_std::{convert::TryFrom, marker::PhantomData, prelude::*};

use xpallet_assets::Chain;
use xpallet_support::{error, traits::MultiSig, warn};

use crate::traits::{BytesLike, ChainProvider, TrusteeSession};
use crate::types::{TrusteeIntentionProps, TrusteeSessionInfo};
use crate::{Error, Module, Trait};
use frame_support::traits::Contains;

pub struct TrusteeSessionManager<T: Trait, TrusteeAddress>(
    PhantomData<T>,
    PhantomData<TrusteeAddress>,
);

impl<T: Trait, TrusteeAddress: BytesLike + ChainProvider>
    TrusteeSession<T::AccountId, TrusteeAddress> for TrusteeSessionManager<T, TrusteeAddress>
{
    fn trustee_session(
        number: u32,
    ) -> Result<TrusteeSessionInfo<T::AccountId, TrusteeAddress>, DispatchError> {
        let chain = TrusteeAddress::chain();
        let generic_info =
            Module::<T>::trustee_session_info_of(chain, number).ok_or_else(|| {
                error!(
                    "[trustee_session]|not found info for this session|chain:{:?}|number:{:}",
                    chain, number
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
        Self::trustee_session(number).map_err(|e| {
            warn!(
                "[last_trustee_session]|last trustee session not exist yet for this chain|Chain:{:?}",
                chain
            );
            e
        })
    }

    #[cfg(feature = "std")]
    fn genesis_trustee(chain: Chain, trustees: &[T::AccountId]) {
        Module::<T>::transition_trustee_session_impl(chain, trustees.to_vec())
            .expect("trustee session transition can not fail; qed");
    }
}

pub struct TrusteeMultisigProvider<T: Trait, C: ChainProvider>(PhantomData<T>, PhantomData<C>);
impl<T: Trait, C: ChainProvider> TrusteeMultisigProvider<T, C> {
    pub fn new() -> Self {
        TrusteeMultisigProvider::<_, _>(Default::default(), Default::default())
    }
}

impl<T: Trait, C: ChainProvider> MultiSig<T::AccountId> for TrusteeMultisigProvider<T, C> {
    fn multisig() -> T::AccountId {
        Module::<T>::trustee_multisig_addr(C::chain())
    }
}

impl<T: Trait, C: ChainProvider> Contains<T::AccountId> for TrusteeMultisigProvider<T, C> {
    fn sorted_members() -> Vec<T::AccountId> {
        vec![Self::multisig()]
    }
}
