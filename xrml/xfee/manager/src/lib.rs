// Copyright 2018-2019 Chainpool.

#![cfg_attr(not(feature = "std"), no_std)]

mod mock;
mod tests;
pub mod types;

// Substrate
use primitives::traits::{As, CheckedDiv, CheckedMul, CheckedSub};
use rstd::result::Result as StdResult;
use support::{decl_module, decl_storage, dispatch::Result, StorageValue};

// ChainX
use chainx_primitives::Acceleration;
use xaccounts::IntentionJackpotAccountIdFor;

pub use self::types::SwitchStore;

/// Simple payment making trait, operating on a single generic `AccountId` type.
pub trait MakePayment<AccountId> {
    /// Make some sort of payment concerning `who` for an extrinsic (transaction) of encoded length
    /// `encoded_len` bytes. Return true iff the payment was successful.
    fn make_payment(who: &AccountId, encoded_len: usize, pay: u64, acc: Acceleration) -> Result;

    fn check_payment(who: &AccountId, encoded_len: usize, pay: u64, acc: Acceleration) -> Result;
}

pub trait Trait: xassets::Trait + xaccounts::Trait + xsystem::Trait {}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn set_producer_producer_fee_proportion(proportion: (u32, u32)) -> Result {
            assert!(proportion.1 != 0, "the proportion denominator can't be Zero");
            assert!(proportion.0 < proportion.1, "the proportion numerator should less than denominator");
            ProducerFeeProportion::<T>::put(proportion);
            Ok(())
        }

        /// first version, when add more SWITCH, should use new switch
        fn set_switch_store(switch: SwitchStore) {
            Switch::<T>::put(switch)
        }
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as XFeeManager {
        pub Switch get(switch): SwitchStore; // Emergency control
        pub ProducerFeeProportion get(producer_fee_proportion) config(): (u32, u32);
        /// The fee to be paid for making a transaction; the base.
        pub TransactionBaseFee get(transaction_base_fee) config(): T::Balance;
        /// The fee to be paid for making a transaction; the per-byte portion.
        pub TransactionByteFee get(transaction_byte_fee) config(): T::Balance;
    }
    add_extra_genesis {
        build(|_: &mut primitives::StorageOverlay, _: &mut primitives::ChildrenStorageOverlay, config: &GenesisConfig<T>| {
            assert!(config.producer_fee_proportion.1 != 0, "the proportion denominator can't be Zero");
            assert!(config.producer_fee_proportion.0 < config.producer_fee_proportion.1, "the proportion numerator should less than denominator");
        })
    }
}

impl<T: Trait> MakePayment<T::AccountId> for Module<T> {
    fn make_payment(
        transactor: &T::AccountId,
        encoded_len: usize,
        power: u64,
        acc: Acceleration,
    ) -> Result {
        let b = Self::calc_fee_and_check(transactor, encoded_len, power, acc)?;

        Self::calc_fee(transactor, b)?;
        Ok(())
    }

    fn check_payment(
        transactor: &T::AccountId,
        encoded_len: usize,
        power: u64,
        acc: Acceleration,
    ) -> Result {
        Self::calc_fee_and_check(transactor, encoded_len, power, acc).map(|_| ())
    }
}

impl<T: Trait> Module<T> {
    pub fn set_switch(store: SwitchStore) {
        Switch::<T>::put(store);
    }

    pub fn transaction_fee(power: u64, encoded_len: u64) -> T::Balance {
        Self::transaction_base_fee() * <T::Balance as As<u64>>::sa(power)
            + Self::transaction_byte_fee() * <T::Balance as As<u64>>::sa(encoded_len)
    }

    fn calc_fee_and_check(
        transactor: &T::AccountId,
        encoded_len: usize,
        power: u64,
        acc: Acceleration,
    ) -> StdResult<T::Balance, &'static str> {
        let b = xassets::Module::<T>::pcx_free_balance(transactor);

        let transaction_fee = Self::transaction_fee(power, encoded_len as u64) * As::sa(acc as u64);

        if b < transaction_fee + <balances::Module<T>>::existential_deposit() {
            return Err("not enough funds for transaction fee");
        }
        Ok(transaction_fee)
    }

    fn calc_fee(from: &T::AccountId, fee: T::Balance) -> Result {
        let proportion = Self::producer_fee_proportion();

        // for_producer = fee * rate.0 / rate.1
        let for_producer = match fee.checked_mul(&As::sa(proportion.0 as u64)) {
            Some(r) => match r.checked_div(&As::sa(proportion.1 as u64)) {
                Some(r) => r,
                None => panic!("dev overflow!"),
            },
            None => panic!("mul overflow!"),
        };

        // for_jackpot = fee - for_producer;
        let for_jackpot = match fee.checked_sub(&for_producer) {
            Some(r) => r,
            None => panic!("sub overflow!"),
        };

        if let Some(p) = xsystem::Module::<T>::block_producer() {
            let jackpot_addr = T::DetermineIntentionJackpotAccountId::accountid_for(&p);
            xassets::Module::<T>::pcx_move_free_balance(from, &p, for_producer)
                .map_err(|e| e.info())?;
            xassets::Module::<T>::pcx_move_free_balance(from, &jackpot_addr, for_jackpot)
                .map_err(|e| e.info())?;
        } else {
            let death_account = xsystem::Module::<T>::death_account();
            xassets::Module::<T>::pcx_move_free_balance(from, &death_account, fee)
                .map_err(|e| e.info())?;
        }

        Ok(())
    }
}
