// Copyright 2018 Chainpool.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate parity_codec as codec;
#[macro_use]
extern crate parity_codec_derive;
#[cfg(feature = "std")]
#[macro_use]
extern crate serde_derive;
extern crate sr_io as runtime_io;
extern crate sr_primitives as primitives;
extern crate sr_std as rstd;
extern crate srml_balances as balances;
#[macro_use]
extern crate srml_support as runtime_support;
extern crate srml_system as system;

use balances::EnsureAccountLiquid;
use primitives::traits::{CheckedSub, OnFinalise};
use runtime_support::dispatch::Result;
pub use storage::double_map::StorageDoubleMap;

pub mod storage;

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
    }
}

impl<T: Trait> OnFinalise<T::BlockNumber> for Module<T> {
    fn on_finalise(_: T::BlockNumber) {
        // do nothing
    }
}

pub trait Trait: balances::Trait {}

impl<T: Trait> Module<T> {
    // util function
    /// handle the fee with the func, deduct fee before exec func, notice the fee have been deducted before func, so if the func return err, the balance already be deducted.
    pub fn handle_fee_before<F>(who: &T::AccountId, fee: T::Balance, check_after_open: bool, mut func: F) -> Result
        where F: FnMut() -> Result
    {
        let from_balance = <balances::Module<T>>::free_balance(who);
        let new_from_balance = match from_balance.checked_sub(&fee) {
            Some(b) => b,
            None => return Err("chainx balance too low to exec this option"),
        };
        <T as balances::Trait>::EnsureAccountLiquid::ensure_account_liquid(who)?;
        if check_after_open && new_from_balance < <balances::Module<T>>::existential_deposit() {
            return Err("chainx balance is not enough after this tx, not allow to be killed at here");
        }

        // deduct free
        <balances::Module<T>>::set_free_balance(who, new_from_balance);
        <balances::Module<T>>::decrease_total_stake_by(fee);

        func()
    }


    /// handle the fee with the func, deduct fee after exec func, notice the func can't do anything related with balance
    pub fn handle_fee_after<F>(who: &T::AccountId, fee: T::Balance, check_after_open: bool, mut func: F) -> Result
        where F: FnMut() -> Result
    {
        let from_balance = <balances::Module<T>>::free_balance(who);
        let new_from_balance = match from_balance.checked_sub(&fee) {
            Some(b) => b,
            None => return Err("chainx balance too low to exec this option"),
        };
        <T as balances::Trait>::EnsureAccountLiquid::ensure_account_liquid(who)?;
        if check_after_open && new_from_balance < <balances::Module<T>>::existential_deposit() {
            return Err("chainx balance is not enough after this tx, not allow to be killed at here");
        }

        func()?;

        // deduct free
        <balances::Module<T>>::set_free_balance(who, new_from_balance);
        <balances::Module<T>>::decrease_total_stake_by(fee);
        Ok(())
    }
}
