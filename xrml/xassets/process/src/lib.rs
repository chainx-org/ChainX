// Copyright 2018 Chainpool.

//! this module is for funds-withdrawal

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

extern crate sr_io as runtime_io;
extern crate sr_primitives as runtime_primitives;
// for substrate runtime module lib
// Needed for type-safe access to storage DB.
#[macro_use]
extern crate srml_support as runtime_support;
extern crate srml_balances as balances;
#[cfg(test)]
extern crate srml_consensus as consensus;
extern crate srml_system as system;
#[cfg(test)]
extern crate srml_timestamp as timestamp;

// chainx runtime module
extern crate xrml_xassets_assets as xassets;
extern crate xrml_xassets_records as xrecords;

#[cfg(test)]
extern crate base58;

//#[cfg(test)]
//mod tests;

use rstd::prelude::*;
//use rstd::result::Result as StdResult;
use runtime_primitives::traits::OnFinalise;
use runtime_support::dispatch::Result;
use runtime_support::StorageValue;

use system::ensure_signed;
use xassets::{Token, ChainT};

pub trait Trait: xassets::Trait + xrecords::Trait { // + btc::Trait {
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn withdraw(origin, token: Token, value: T::Balance, addr: Vec<u8>, ext: Vec<u8>) -> Result {
            runtime_io::print("[xassets process withdrawal] withdraw");
            let who = ensure_signed(origin)?;

            Self::verify_addr(&token, &addr, &ext)?;

            xrecords::Module::<T>::withdrawal(&who, &token, value, addr, ext)?;
            Ok(())
        }
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as Withdrawal {
    }
}

impl<T: Trait> Module<T> {
    fn verify_addr(token: &Token, addr: &[u8], _ext: &[u8]) -> Result {
        match token.as_slice() {
//            btc::Module::<T>::Token => btc::Module::<T>::check_addr(&addr, b""),
            _ => return Err("not found match token Token addr checker"),
        }
    }

    pub fn verify_address(token: Token, addr: Vec<u8>, ext: Vec<u8>) -> Result {
        Self::verify_addr(&token, &addr, &ext)
    }
}
