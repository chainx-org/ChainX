// Copyright 2018 Chainpool.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate parity_codec as codec;
#[macro_use]
extern crate parity_codec_derive;
#[cfg(feature = "std")]
#[macro_use]
extern crate serde_derive;
#[cfg(test)]
extern crate substrate_primitives;
extern crate sr_io as runtime_io;
extern crate sr_primitives as primitives;
extern crate sr_std as rstd;
extern crate srml_balances as balances;
#[macro_use]
extern crate srml_support as runtime_support;
extern crate srml_system as system;

extern crate cxrml_system as cxsystem;
extern crate cxrml_associations as associations;

// use balances::EnsureAccountLiquid;
use rstd::prelude::*;
use primitives::traits::{CheckedSub, CheckedAdd, OnFinalise, As, Zero};
use runtime_support::dispatch::Result;
pub use storage::double_map::StorageDoubleMap;

pub mod storage;
#[cfg(test)]
mod tests;

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
    }
}

impl<T: Trait> OnFinalise<T::BlockNumber> for Module<T> {
    fn on_finalise(_: T::BlockNumber) {
        // do nothing
    }
}

impl<T: Trait> associations::OnCalcFee<T::AccountId, T::Balance> for Module<T> {
    fn on_calc_fee(who: &T::AccountId, total_fee: T::Balance) -> Result {
        Self::calc_fee(who, total_fee)
    }
}

pub trait Trait: associations::Trait + cxsystem::Trait {}

impl<T: Trait> Module<T> {
    fn calc_fee_withaccount(who: &T::AccountId, fee: T::Balance, rate: &[(usize, T::AccountId)]) -> Result {
        let from_balance = <balances::Module<T>>::free_balance(who);
        let new_from_balance = match from_balance.checked_sub(&fee) {
            Some(b) => b,
            None => return Err("chainx balance too low to exec this option"),
        };

        if rate.len() < 1 { panic!("can't input a empty rate array") }
        if rate.len() == 1 {
            let to_balance = <balances::Module<T>>::free_balance(&rate[0].1);
            let new_to_balance = match to_balance.checked_add(&fee) {
                Some(b) => b,
                None => return Err("chainx balance too high to exec this option"),
            };

            <balances::Module<T>>::set_free_balance(who, new_from_balance);
            <balances::Module<T>>::set_free_balance(&rate[0].1, new_to_balance);
            return Ok(());
        }

        if rate.iter().fold(0, |acc, i| acc + i.0) != 10 {
            panic!("the rate sum must be 10 part");
        }

        let mut v = Vec::new();
        let mut fee_sum: T::Balance = Zero::zero();
        for (r, accountid) in rate[1..].iter() {
            let a: T::Balance = As::sa(*r);
            let fee = a * fee / As::sa(10);
            fee_sum += fee;
            v.push((fee, accountid));
        }
        v.insert(0, (fee - fee_sum, &rate[0].1));

        for (fee, accoundid) in v {
            let to_balance = <balances::Module<T>>::free_balance(accoundid);
            let new_to_balance = match to_balance.checked_add(&fee) {
                Some(b) => b,
                None => Zero::zero(),
            };
            <balances::Module<T>>::set_free_balance(accoundid, new_to_balance);
        }
        <balances::Module<T>>::set_free_balance(who, new_from_balance);
        Ok(())
    }

    fn calc_fee(from_who: &T::AccountId, fee: T::Balance) -> Result {
        let mut v = Vec::new();
        // 50% for block producer
        if let Some(p) = cxsystem::Module::<T>::block_producer() {
            v.push((5, p));
        } else {
            v.push((5, cxsystem::Module::<T>::death_account()));
        }
        // 50% for relationship accountid
        if let Some(to) = associations::Module::<T>::relationship(from_who) {
            v.push((5, to))
        } else {
            if let Some(p) = cxsystem::Module::<T>::block_producer() {
                v.push((5, p));
            } else {
                v.push((5, cxsystem::Module::<T>::death_account()));
            }
        }

        Self::calc_fee_withaccount(from_who, fee, v.as_slice())
    }


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
        // <T as balances::Trait>::EnsureAccountLiquid::ensure_account_liquid(who)?;
        if check_after_open && new_from_balance < <balances::Module<T>>::existential_deposit() {
            return Err("chainx balance is not enough after this tx, not allow to be killed at here");
        }

        // deduct free
        Self::calc_fee(who, fee)?;

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
        // <T as balances::Trait>::EnsureAccountLiquid::ensure_account_liquid(who)?;
        if check_after_open && new_from_balance < <balances::Module<T>>::existential_deposit() {
            return Err("chainx balance is not enough after this tx, not allow to be killed at here");
        }

        func()?;

        // deduct free
        Self::calc_fee(who, fee)?;
        Ok(())
    }
}
