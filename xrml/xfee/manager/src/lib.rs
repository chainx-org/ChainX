// Copyright 2018 Chainpool.

#![cfg_attr(not(feature = "std"), no_std)]
extern crate srml_balances as balances;
extern crate srml_system as system;
#[macro_use]
extern crate srml_support as runtime_support;
extern crate sr_primitives;
#[macro_use]
extern crate parity_codec_derive;
extern crate parity_codec as codec;

use runtime_support::dispatch::Result;
use runtime_support::StorageValue;
use sr_primitives::traits::As;

/// Simple payment making trait, operating on a single generic `AccountId` type.
pub trait MakePayment<AccountId> {
    /// Make some sort of payment concerning `who` for an extrinsic (transaction) of encoded length
    /// `encoded_len` bytes. Return true iff the payment was successful.
    fn make_payment(who: &AccountId, encoded_len: usize, pay: u64) -> Result;
}

pub trait Trait: balances::Trait {
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
         fn deposit_event() = default;
    }
}

decl_event!(
    pub enum Event<T>
    where
        B = <T as balances::Trait>::Balance
    {
        Fee(B),
    }
);

decl_storage! {
    trait Store for Module<T: Trait> as XFeeManager {
        pub Switch get(switch) config(): bool; // Emergency control
    }
}

impl<T: Trait> MakePayment<T::AccountId> for Module<T> {
    fn make_payment(transactor: &T::AccountId, encoded_len: usize, pay: u64) -> Result {
        let b = <balances::Module<T>>::free_balance(transactor);
        let transaction_fee = <balances::Module<T>>::transaction_base_fee()
            + T::Balance::sa(pay)
            + <balances::Module<T>>::transaction_byte_fee()
                * <T::Balance as As<u64>>::sa(encoded_len as u64);
        if b < transaction_fee + <balances::Module<T>>::existential_deposit() {
            return Err("not enough funds for transaction fee");
        }
        <balances::Module<T>>::set_free_balance(transactor, b - transaction_fee);
        <balances::Module<T>>::decrease_total_stake_by(transaction_fee);
        Ok(())
    }
}
