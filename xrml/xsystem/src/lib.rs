// Copyright 2018 Chainpool.

//! this module is for chainx system

#![cfg_attr(not(feature = "std"), no_std)]

#[macro_use]
extern crate parity_codec_derive;
extern crate parity_codec as codec;

// for substrate
#[cfg(feature = "std")]
extern crate sr_io as runtime_io;
extern crate sr_primitives as runtime_primitives;
extern crate sr_std as rstd;
extern crate substrate_inherents as inherents;
#[cfg(feature = "std")]
extern crate substrate_primitives;
// for substrate runtime module lib
// Needed for type-safe access to storage DB.
#[macro_use]
extern crate srml_support as runtime_support;
extern crate srml_system as system;

#[cfg(test)]
mod tests;

use runtime_support::dispatch::Result;
use runtime_support::StorageValue;

#[cfg(feature = "std")]
use inherents::ProvideInherentData;
use inherents::{InherentData, InherentIdentifier, IsFatalError, ProvideInherent, RuntimeString};
#[cfg(feature = "std")]
use rstd::result::Result as StdResult;

use system::ensure_inherent;

pub trait Trait: system::Trait {}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn set_block_producer(origin, producer: T::AccountId) -> Result {
            ensure_inherent(origin)?;

            BlockProducer::<T>::put(producer);
            Ok(())
        }
        fn on_finalise(_n: T::BlockNumber) {
            BlockProducer::<T>::kill();
        }
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as XSystem {
        pub BlockProducer get(block_producer): Option<T::AccountId>;
        pub DeathAccount get(death_account) config(): T::AccountId;
        // TODO remove this to other module
        pub BurnAccount get(burn_account) config(): T::AccountId;
    }
}

impl<T: Trait> ProvideInherent for Module<T> {
    type Call = Call<T>;
    type Error = InherentError;
    const INHERENT_IDENTIFIER: InherentIdentifier = INHERENT_IDENTIFIER;
    fn create_inherent(data: &InherentData) -> Option<Self::Call> {
        let r = data
            .get_data::<T::AccountId>(&INHERENT_IDENTIFIER)
            .expect("Gets and decodes producer inherent data");
        let producer = r.expect("producer must set before");
        Some(Call::set_block_producer(producer))
    }
}

pub const INHERENT_IDENTIFIER: InherentIdentifier = *b"producer";

#[derive(Encode)]
#[cfg_attr(feature = "std", derive(Debug, Decode))]
pub enum InherentError {
    /// no producer set
    NoBlockProducer,
    /// Some other error.
    Other(RuntimeString),
}

impl IsFatalError for InherentError {
    fn is_fatal_error(&self) -> bool {
        match self {
            _ => true,
        }
    }
}

#[cfg(feature = "std")]
pub struct InherentDataProvider<AccountId> {
    block_producer: AccountId,
}

#[cfg(feature = "std")]
impl<AccountId: codec::Codec + Clone> InherentDataProvider<AccountId> {
    pub fn new(who: &AccountId) -> Self {
        InherentDataProvider::<AccountId> {
            block_producer: who.clone(),
        }
    }
}

#[cfg(feature = "std")]
impl<AccountId: codec::Codec> ProvideInherentData for InherentDataProvider<AccountId> {
    fn inherent_identifier(&self) -> &'static InherentIdentifier {
        &INHERENT_IDENTIFIER
    }

    fn provide_inherent_data(
        &self,
        inherent_data: &mut InherentData,
    ) -> StdResult<(), RuntimeString> {
        inherent_data.put_data(INHERENT_IDENTIFIER, &self.block_producer)
    }

    fn error_to_string(&self, _error: &[u8]) -> Option<String> {
        // do not handle due no check for this inherent
        None
    }
}
