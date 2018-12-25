// Copyright 2018 Chainpool.

//! this module is for chainx system

#![cfg_attr(not(feature = "std"), no_std)]

extern crate parity_codec as codec;

// for substrate
#[cfg(feature = "std")]
extern crate substrate_primitives;

// for substrate runtime
// map!, vec! marco.
extern crate sr_std as rstd;

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

use rstd::prelude::*;
use rstd::result;
use runtime_primitives::traits::{Block as BlockT, ProvideInherent};
use runtime_primitives::CheckInherentError;
use runtime_support::dispatch::Result;
use runtime_support::StorageValue;

use system::ensure_inherent;

pub trait Trait: system::Trait {
    const XSYSTEM_SET_POSITION: u32;
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn set_block_producer(origin, producer: T::AccountId) -> Result {
            ensure_inherent(origin)?;

            assert!(
                <system::Module<T>>::extrinsic_index() == Some(T::XSYSTEM_SET_POSITION),
                "BlockProducer extrinsic must be at position {} in the block",
                T::XSYSTEM_SET_POSITION
            );

            BlockProdocer::<T>::put(producer);
            Ok(())
        }
        fn on_finalise(_n: T::BlockNumber) {
            BlockProdocer::<T>::kill();
        }
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as XSystem {
        pub BlockProdocer get(block_producer): Option<T::AccountId>;
        pub DeathAccount get(death_account) config(): T::AccountId;
        // TODO remove this to other module
        pub BurnAccount get(burn_account) config(): T::AccountId;
    }
}

impl<T: Trait> ProvideInherent for Module<T> {
    type Inherent = T::AccountId;
    type Call = Call<T>;

    fn create_inherent_extrinsics(data: Self::Inherent) -> Vec<(u32, Self::Call)> {
        vec![(T::XSYSTEM_SET_POSITION, Call::set_block_producer(data))]
    }

    fn check_inherent<Block: BlockT, F: Fn(&Block::Extrinsic) -> Option<&Self::Call>>(
        _block: &Block,
        _data: Self::Inherent,
        _extract_function: &F,
    ) -> result::Result<(), CheckInherentError> {
        Ok(())
    }
}
