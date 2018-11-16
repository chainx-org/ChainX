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
extern crate srml_balances as balances;
#[cfg(test)]
extern crate srml_timestamp as timestamp;
#[cfg(test)]
extern crate srml_consensus as consensus;

// chainx runtime module
#[cfg(test)]
extern crate cxrml_system as cxsystem;
#[cfg(test)]
extern crate cxrml_associations as associations;
extern crate cxrml_support as cxsupport;
extern crate cxrml_tokenbalances as tokenbalances;
extern crate cxrml_funds_financialrecords as financialrecords;
// chainx runtime module bridge
extern crate cxrml_bridge_btc as btc;

#[cfg(test)]
extern crate base58;

#[cfg(test)]
mod tests;

use rstd::prelude::*;
//use rstd::result::Result as StdResult;
use runtime_support::dispatch::Result;
use runtime_support::StorageValue;
use runtime_primitives::traits::OnFinalise;

use system::ensure_signed;
use tokenbalances::{Symbol, TokenT};


pub trait Trait: tokenbalances::Trait + financialrecords::Trait + btc::Trait {
//    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

//decl_event!(
//    pub enum Event<T> where
//        <T as system::Trait>::AccountId,
//        <T as balances::Trait>::Balance
//    {
//        Fee(AccountId, Balance),
//    }
//);

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn withdraw(origin, sym: Symbol, value: T::TokenBalance, addr: Vec<u8>, ext: Vec<u8>) -> Result;
    }
}

impl<T: Trait> OnFinalise<T::BlockNumber> for Module<T> {
    fn on_finalise(_: T::BlockNumber) {
        // do nothing
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as Withdrawal {
        pub WithdrawalFee get(withdrawal_fee) config(): T::Balance;
    }
}

impl<T: Trait> Module<T> {
    // event
//    /// Deposit one of this module's events.
//    fn deposit_event(event: Event<T>) {
//        <system::Module<T>>::deposit_event(<T as Trait>::Event::from(event).into());
//    }

    fn withdraw(origin: T::Origin, sym: Symbol, value: T::TokenBalance, addr: Vec<u8>, ext: Vec<u8>) -> Result {
        let who = ensure_signed(origin)?;

        cxsupport::Module::<T>::handle_fee_before(&who, Self::withdrawal_fee(), true, || Ok(()))?;

        let d = Self::verify_addr(&sym, &addr, &ext)?;

        financialrecords::Module::<T>::withdrawal(&who, &sym, value, addr, ext)?;
        Ok(())
    }

    fn verify_addr(sym: &Symbol, addr: &[u8], ext: &[u8]) -> Result {
        match sym.as_ref() {
            btc::Module::<T>::SYMBOL => { btc::Module::<T>::check_addr(&addr, b"") }
            _ => return Err("not found match token symbol addr checker")
        }
    }

    pub fn verify_address(sym: Symbol, addr: Vec<u8>, ext: Vec<u8>) -> Result {
        Self::verify_addr(&sym, &addr, &ext)
    }
}


