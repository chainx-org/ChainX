// Copyright 2018-2019 Chainpool.

#![cfg_attr(not(feature = "std"), no_std)]

mod mock;
mod tests;
pub mod types;

// Substrate
use primitives::traits::{As, CheckedDiv, CheckedMul, CheckedSub};
use rstd::collections::btree_map::BTreeMap;
use rstd::prelude::Vec;
use rstd::result;
use support::{decl_event, decl_module, decl_storage, dispatch::Result, StorageValue};

// ChainX
use chainx_primitives::Acceleration;
use xr_primitives::XString;

use xaccounts::IntentionJackpotAccountIdFor;
#[cfg(feature = "std")]
use xsupport::u8array_to_string;
use xsupport::{info, trace, warn};

pub use self::types::SwitchStore;

/// Simple payment making trait, operating on a single generic `AccountId` type.
pub trait MakePayment<AccountId> {
    /// Make some sort of payment concerning `who` for an extrinsic (transaction) of encoded length
    /// `encoded_len` bytes. Return true iff the payment was successful.
    fn make_payment(who: &AccountId, encoded_len: usize, pay: u64, acc: Acceleration) -> Result;

    fn check_payment(who: &AccountId, encoded_len: usize, pay: u64, acc: Acceleration) -> Result;
}

pub trait Trait: xassets::Trait + xaccounts::Trait + xsystem::Trait {
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

decl_event!(
    pub enum Event<T> where
        <T as system::Trait>::AccountId,
        <T as xassets::Trait>::Balance
    {
        FeeForJackpot(AccountId, Balance),
        FeeForProducer(AccountId, Balance),
        FeeForCouncil(AccountId, Balance),
    }
);

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event<T>() = default;

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

        /// Set a new weight for a method.
        fn set_method_call_weight(method: Vec<u8>, weight: u64) {
            <MethodCallWeight<T>>::mutate(|method_weight| {
                match (*method_weight).insert(method.clone(), weight) {
                    Some(_a) => {
                        info!("reset new fee|key:{:}|new value:{:}|old value:{:}", u8array_to_string(&method), weight, _a);
                    },
                    None => {
                        info!("set new fee|key:{:}|value:{:}", u8array_to_string(&method), weight);
                    },
                }
            });
        }

        /// Remove a method weight.
        fn remove_method_call_weight(method: Vec<u8>) {
            <MethodCallWeight<T>>::mutate(|method_weight| {
                match (*method_weight).remove(&method) {
                    Some(_a) => {
                        info!("remove an existing method weight|key:{:}|value:{:}", u8array_to_string(&method), _a);
                    },
                    None => {
                        info!("method {:} does not exist", u8array_to_string(&method));
                    }
                }
            });
        }
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as XFeeManager {
        /// Emergency control
        pub Switch get(switch): SwitchStore;
        /// Each callable method in runtime normally has a different weight.
        pub MethodCallWeight get(method_call_weight): BTreeMap<XString, u64>;
        /// How much fee of a block should be rewarded to the block producer.
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
    ) -> result::Result<T::Balance, &'static str> {
        let b = xassets::Module::<T>::pcx_free_balance(transactor);

        let transaction_fee = Self::transaction_fee(power, encoded_len as u64) * As::sa(acc as u64);

        if b < transaction_fee {
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
                None => return Err("[fee]calc fee proportion dev overflow!"),
            },
            None => return Err("[fee]calc fee proportion mul overflow!"),
        };

        // for_jackpot = fee - for_producer;
        let for_jackpot = match fee.checked_sub(&for_producer) {
            Some(r) => r,
            None => return Err("[fee]sub overflow!"),
        };

        if let Some(p) = xsystem::Module::<T>::block_producer() {
            let jackpot_addr = T::DetermineIntentionJackpotAccountId::accountid_for(&p);

            trace!(
                "[calc_fee]|move fee|from:{:},{:?}|to jackpot:{:},{:?}|to_producer:{:},{:}",
                from,
                fee,
                for_jackpot,
                jackpot_addr,
                for_producer,
                p
            );

            let _ = xassets::Module::<T>::pcx_move_free_balance(from, &p, for_producer)
                .map_err(|e| e.info())?;

            Self::deposit_event(RawEvent::FeeForProducer(p, for_producer));

            let _ = xassets::Module::<T>::pcx_move_free_balance(from, &jackpot_addr, for_jackpot)
                .map_err(|e| e.info())?;

            Self::deposit_event(RawEvent::FeeForJackpot(jackpot_addr, for_jackpot));
        } else {
            let council = xaccounts::Module::<T>::council_account();

            warn!(
                "[calc_fee]|current block not set producer!|council:{:},{:?}",
                council, fee
            );

            xassets::Module::<T>::pcx_move_free_balance(from, &council, fee)
                .map_err(|e| e.info())?;

            Self::deposit_event(RawEvent::FeeForJackpot(council, fee));
        }

        Ok(())
    }
}
