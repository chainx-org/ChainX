// Copyright 2018 Chainpool.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg_attr(not(feature = "std"), macro_use)]
extern crate sr_std as rstd;

extern crate parity_codec as codec;
extern crate sr_primitives;

extern crate srml_balances as balances;
extern crate srml_system as system;
#[macro_use]
extern crate srml_support as support;

extern crate xrml_xaccounts as xaccounts;
extern crate xrml_xsystem as xsystem;

use rstd::prelude::*;
use rstd::result::Result as StdResult;

use sr_primitives::traits::{As, CheckedAdd, CheckedSub, Zero};
use support::dispatch::Result;
#[cfg(feature = "std")]
use support::StorageValue;

/// Simple payment making trait, operating on a single generic `AccountId` type.
pub trait MakePayment<AccountId> {
    /// Make some sort of payment concerning `who` for an extrinsic (transaction) of encoded length
    /// `encoded_len` bytes. Return true iff the payment was successful.
    fn make_payment(who: &AccountId, encoded_len: usize, pay: u64) -> Result;

    fn check_payment(who: &AccountId, encoded_len: usize, pay: u64) -> Result;
}

pub trait Trait: balances::Trait + xsystem::Trait + xaccounts::Trait {
    //    /// The overarching event type.
    //    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
//         fn deposit_event() = default;
    }
}

//decl_event!(
//    pub enum Event<T> where B = <T as balances::Trait>::Balance {
//        Fee(B),
//    }
//);

decl_storage! {
    trait Store for Module<T: Trait> as XFeeManager {
        pub Switch get(switch) config(): bool; // Emergency control
    }
}

impl<T: Trait> MakePayment<T::AccountId> for Module<T> {
    fn make_payment(transactor: &T::AccountId, encoded_len: usize, power: u64) -> Result {
        let b = Self::calc_fee_and_check(transactor, encoded_len, power)?;
        //        <balances::Module<T>>::set_free_balance(transactor, b - transaction_fee);
        //        <balances::Module<T>>::decrease_total_stake_by(transaction_fee);
        Self::calc_fee(transactor, b)?;
        Ok(())
    }

    fn check_payment(transactor: &T::AccountId, encoded_len: usize, power: u64) -> Result {
        Self::calc_fee_and_check(transactor, encoded_len, power).map(|_| ())
    }
}

impl<T: Trait> Module<T> {
    fn calc_fee_and_check(
        transactor: &T::AccountId,
        encoded_len: usize,
        power: u64,
    ) -> StdResult<T::Balance, &'static str> {
        let b = <balances::Module<T>>::free_balance(transactor);
        let transaction_fee = <balances::Module<T>>::transaction_base_fee()
            * <T::Balance as As<u64>>::sa(power)
            + <balances::Module<T>>::transaction_byte_fee()
                * <T::Balance as As<u64>>::sa(encoded_len as u64);
        if b < transaction_fee + <balances::Module<T>>::existential_deposit() {
            return Err("not enough funds for transaction fee");
        }
        Ok(transaction_fee)
    }

    fn calc_fee(from_who: &T::AccountId, fee: T::Balance) -> Result {
        let mut v = Vec::new();
        // 50% for block producer
        if let Some(p) = xsystem::Module::<T>::block_producer() {
            v.push((5, p));
        } else {
            v.push((5, xsystem::Module::<T>::death_account()));
        }
        // 50% for account_relationships accountid
        if let Some(to) = xaccounts::Module::<T>::account_relationships(from_who) {
            v.push((5, to))
        } else {
            if let Some(p) = xsystem::Module::<T>::block_producer() {
                v.push((5, p));
            } else {
                v.push((5, xsystem::Module::<T>::death_account()));
            }
        }

        Self::calc_fee_withaccount(from_who, fee, v.as_slice())
    }

    fn calc_fee_withaccount(
        who: &T::AccountId,
        fee: T::Balance,
        rate: &[(usize, T::AccountId)],
    ) -> Result {
        let from_balance = <balances::Module<T>>::free_balance(who);
        let new_from_balance = match from_balance.checked_sub(&fee) {
            Some(b) => b,
            None => return Err("chainx balance too low to exec this option"),
        };

        if rate.len() < 1 {
            panic!("can't input a empty rate array")
        }
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
}
