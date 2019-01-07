// Copyright 2018 Chainpool.

//! this module is for associations

#![cfg_attr(not(feature = "std"), no_std)]
// Needed for deriving `Encode` and `Decode` for `RawEvent`.
#[macro_use]
extern crate parity_codec_derive;
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
extern crate srml_system as system;

#[cfg(test)]
mod tests;

use rstd::prelude::*;
use runtime_primitives::traits::{CheckedAdd, CheckedSub, OnFinalise, Zero};
use runtime_support::dispatch::Result;
use runtime_support::{StorageMap, StorageValue};

use system::ensure_signed;

pub trait OnCalcFee<AccountId, Balance> {
    fn on_calc_fee(who: &AccountId, total_fee: Balance) -> Result;
}

impl<AccountId, Balance> OnCalcFee<AccountId, Balance> for () {
    fn on_calc_fee(_: &AccountId, _: Balance) -> Result {
        Ok(())
    }
}

pub trait Trait: system::Trait + balances::Trait {
    type OnCalcFee: OnCalcFee<Self::AccountId, Self::Balance>;
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

decl_event!(
    pub enum Event<T> where
        <T as system::Trait>::AccountId,
        <T as balances::Trait>::Balance
    {
        InitAccount(AccountId, AccountId, Balance),
        InitExchangeAccount(AccountId, AccountId),
        InitChannelRelationship(Vec<u8>, AccountId),
    }
);

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        //fn init_account(origin, who: T::AccountId) -> Result;
        //fn init_exchange_relationship(origin, who: T::AccountId) -> Result;
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as Associations {
        pub Relationship get(relationship): map T::AccountId => Option<T::AccountId>;
        pub ChannelRelationship get(channel_relationship): map Vec<u8> => Option< T::AccountId >;
        pub RevChannelRelationship get(channel_relationship_rev): map T::AccountId => Option< Vec<u8> >;
        pub ExchangeRelationship get(exchange_relationship): map T::AccountId => Option<T::AccountId>;
        // fee
        pub InitFee get(init_fee) config(): T::Balance;
    }
}

impl<T: Trait> Module<T> {
    // event
    /// Deposit one of this module's events.
    fn deposit_event(event: Event<T>) {
        <system::Module<T>>::deposit_event(<T as Trait>::Event::from(event).into());
    }

    fn check_no_init(who: &T::AccountId) -> Result {
        if Self::relationship(who).is_some() {
            return Err("has register this account");
        } else {
            if balances::FreeBalance::<T>::exists(who) {
                return Err("this account is existing");
            }
        }
        Ok(())
    }

    pub fn is_init(who: &T::AccountId) -> bool {
        if let Err(_) = Self::check_no_init(who) {
            true
        } else {
            false
        }
    }
}

impl<T: Trait> Module<T> {
    pub fn init_account(origin: T::Origin, who: T::AccountId) -> Result {
        runtime_io::print("[associations] init_account");
        let from = ensure_signed(origin)?;
        // deduct fee first
        T::OnCalcFee::on_calc_fee(&from, Self::init_fee())?;

        Self::check_no_init(&who)?;

        Relationship::<T>::insert(&who, from.clone());

        let from_balance = balances::Module::<T>::free_balance(&from);
        let to_balance = balances::Module::<T>::free_balance(&who);
        let value: T::Balance = Zero::zero();
        let new_from_balance = match from_balance.checked_sub(&value) {
            Some(b) => b,
            None => return Err("balance too low to send value"),
        };
        let new_to_balance = match to_balance.checked_add(&value) {
            Some(b) => b,
            None => return Err("destination balance too high to receive value"),
        };

        balances::Module::<T>::set_free_balance(&from, new_from_balance);
        balances::Module::<T>::set_free_balance_creating(&who, new_to_balance);

        Self::deposit_event(RawEvent::InitAccount(from, who, value));
        Ok(())
    }

    pub fn init_exchange_relationship(origin: T::Origin, who: T::AccountId) -> Result {
        runtime_io::print("[associations] init_exchange_relationship");
        let from = ensure_signed(origin)?;
        // deduct fee first
        T::OnCalcFee::on_calc_fee(&from, Self::init_fee())?;

        if Self::exchange_relationship(&who).is_some() {
            return Err("has register this account");
        }

        ExchangeRelationship::<T>::insert(&who, from.clone());

        Self::deposit_event(RawEvent::InitExchangeAccount(from, who));
        Ok(())
    }
    pub fn init_channel_relationship(channel: Vec<u8>, who: &T::AccountId) -> Result {
        if Self::channel_relationship(&channel).is_some() {
            return Err("has register this channel");
        }

        ChannelRelationship::<T>::insert(channel.clone(), who.clone());
        RevChannelRelationship::<T>::insert(who.clone(), channel.clone());

        Self::deposit_event(RawEvent::InitChannelRelationship(channel, who.clone()));
        Ok(())
    }
}
