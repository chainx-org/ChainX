pub mod bitcoin;

use codec::{Decode, Encode, Error as CodecError};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
// Substrate
use frame_support::dispatch::DispatchError;
use sp_std::{convert::TryFrom, marker::PhantomData, prelude::Vec};

use xpallet_assets::Chain;
use xpallet_support::{error, traits::MultiSig, warn};

use crate::traits::{BytesLike, ChainProvider, TrusteeSession};
use crate::types::{TrusteeIntentionProps, TrusteeSessionInfo};
use crate::{Error, Module, Trait};

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
}

pub struct ChainContext;
impl ChainContext {
    pub fn new(chain: Chain) -> Self {
        use frame_support::StorageValue;
        crate::TmpChain::put(chain);
        ChainContext
    }
}

impl Drop for ChainContext {
    fn drop(&mut self) {
        use frame_support::StorageValue;
        crate::TmpChain::kill();
    }
}
impl ChainProvider for ChainContext {
    fn chain() -> Chain {
        use frame_support::StorageValue;
        crate::TmpChain::get()
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
