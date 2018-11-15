//! this module is for bch-bridge

#![cfg_attr(not(feature = "std"), no_std)]
// for encode/decode
// Needed for deriving `Serialize` and `Deserialize` for various types.
// We only implement the serde traits for std builds - they're unneeded
// in the wasm runtime.
#[cfg(feature = "std")]
#[macro_use]
extern crate serde_derive;

// Needed for deriving `Encode` and `Decode` for `RawEvent`.
//#[macro_use]
//extern crate parity_codec_derive;
extern crate parity_codec as codec;

// for substrate
// Needed for the set of mock primitives used in our tests.
#[cfg(feature = "std")]
extern crate substrate_primitives;

// for substrate runtime
// map!, vec! marco.
extern crate sr_std as rstd;
// Needed for tests (`with_externalities`).
#[cfg(feature = "std")]
extern crate sr_io as runtime_io;
extern crate sr_primitives as runtime_primitives;
// for substrate runtime module lib
// Needed for type-safe access to storage DB.
#[macro_use]
extern crate srml_support as runtime_support;
extern crate srml_system as system;

#[cfg(test)]
mod tests;

//use codec::{Codec, Decode, Encode};
use rstd::prelude::*;
//use rstd::marker::PhantomData;
//use rstd::result::Result as StdResult;
use runtime_support::dispatch::Result;
use runtime_support::StorageValue;
use runtime_primitives::traits::OnFinalise;

use system::ensure_inherent;


pub trait Trait: system::Trait {}


decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn set_block_producer(origin, producer: T::AccountId) -> Result;
    }
}

impl<T: Trait> OnFinalise<T::BlockNumber> for Module<T> {
    fn on_finalise(_: T::BlockNumber) {
        BlockProdocer::<T>::kill();
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as CXSystem {
        pub BlockProdocer get(block_producer) config(): Option<T::AccountId>;
    }
}

impl<T: Trait> Module<T> {
    fn set_block_producer(origin: T::Origin, producer: T::AccountId) -> Result {
        ensure_inherent(origin)?;
        BlockProdocer::<T>::put(producer);
        Ok(())
    }
}


