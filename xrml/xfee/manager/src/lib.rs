// Copyright 2018 Chainpool.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate sr_std as rstd;

extern crate parity_codec as codec;
extern crate sr_primitives;

extern crate srml_balances as balances;
extern crate srml_system as system;
#[macro_use]
extern crate srml_support as support;

extern crate xrml_xaccounts as xaccounts;
extern crate xrml_xassets_assets as xassets;
extern crate xrml_xsystem as xsystem;

extern crate chainx_primitives;

//use rstd::prelude::*;
use rstd::result::Result as StdResult;

use sr_primitives::traits::{As, CheckedDiv, CheckedMul, CheckedSub};
use support::dispatch::Result;
use support::StorageValue;

use xaccounts::IntentionJackpotAccountIdFor;

use chainx_primitives::Acceleration;
use codec::{Decode, Encode};

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

#[derive(PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct SwitchStore {
    pub global: bool,
    pub spot: bool,
    pub xbtc: bool,
    pub sdot: bool,
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
        build(|_: &mut sr_primitives::StorageOverlay, _: &mut sr_primitives::ChildrenStorageOverlay, config: &GenesisConfig<T>| {
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

#[cfg(test)]
mod tests {
    extern crate sr_io as runtime_io;
    extern crate substrate_primitives;

    use self::runtime_io::with_externalities;
    use self::substrate_primitives::{Blake2Hasher, H256};
    use super::*;
    use sr_primitives::testing::{Digest, DigestItem, Header};
    use sr_primitives::traits::{BlakeTwo256, IdentityLookup};
    use sr_primitives::BuildStorage;

    impl_outer_origin! {
        pub enum Origin for Test {}
    }

    #[derive(Clone, Eq, PartialEq)]
    pub struct Test;

    impl system::Trait for Test {
        type Origin = Origin;
        type Index = u64;
        type BlockNumber = u64;
        type Hash = H256;
        type Hashing = BlakeTwo256;
        type Digest = Digest;
        type AccountId = u64;
        type Lookup = IdentityLookup<u64>;
        type Header = Header;
        type Event = ();
        type Log = DigestItem;
    }

    impl balances::Trait for Test {
        type Balance = u64;
        type OnFreeBalanceZero = ();
        type OnNewAccount = ();
        type EnsureAccountLiquid = ();
        type Event = ();
    }

    impl xassets::Trait for Test {
        /// Event
        type Event = ();
        type OnAssetChanged = ();
        type OnAssetRegisterOrRevoke = ();
    }

    impl xsystem::Trait for Test {
        type ValidatorList = ();
    }

    pub struct MockDeterminator;

    impl xaccounts::IntentionJackpotAccountIdFor<u64> for MockDeterminator {
        fn accountid_for(_: &u64) -> u64 {
            1000
        }
    }

    impl xaccounts::Trait for Test {
        type Event = ();
        type DetermineIntentionJackpotAccountId = MockDeterminator;
    }

    impl Trait for Test {}

    pub fn new_test_ext() -> runtime_io::TestExternalities<Blake2Hasher> {
        let mut r = system::GenesisConfig::<Test>::default()
            .build_storage()
            .unwrap()
            .0;
        // balance
        r.extend(
            balances::GenesisConfig::<Test> {
                balances: vec![(1, 1000), (2, 510), (3, 1000)],
                transaction_base_fee: 10,
                transaction_byte_fee: 1,
                existential_deposit: 0,
                transfer_fee: 0,
                creation_fee: 0,
            }
            .build_storage()
            .unwrap()
            .0,
        );
        // xsystem
        r.extend(
            xsystem::GenesisConfig::<Test> {
                death_account: 0,
                burn_account: 100,
            }
            .build_storage()
            .unwrap()
            .0,
        );
        // xassets
        r.extend(
            xassets::GenesisConfig::<Test> {
                pcx: (b"PolkadotChainX".to_vec(), 3, b"PCX onchain token".to_vec()),
                memo_len: 128,
                // asset, is_psedu_intention, init for account
                // Vec<(Asset, bool, Vec<(T::AccountId, u64)>)>;
                asset_list: vec![],
            }
            .build_storage()
            .unwrap()
            .0,
        );

        r.extend(
            GenesisConfig::<Test> {
                switch: true,
                producer_fee_proportion: (1, 10),
                _genesis_phantom_data: Default::default(),
            }
            .build_storage()
            .unwrap()
            .0,
        );
        r.into()
    }

    type XAssets = xassets::Module<Test>;

    #[test]
    fn test_fee() {
        with_externalities(&mut new_test_ext(), || {
            xsystem::BlockProducer::<Test>::put(99);

            assert_ok!(Module::<Test>::make_payment(&1, 10, 10));
            // base fee = 10, bytes fee = 1
            let fee = 10 * 10 + 1 * 10;
            assert_eq!(XAssets::pcx_free_balance(&1), 1000 - fee);
            // block producer
            assert_eq!(XAssets::pcx_free_balance(&99), fee / 10);
            // jackpot account
            assert_eq!(XAssets::pcx_free_balance(&1000), fee * 9 / 10);
            // death account
            assert_eq!(XAssets::pcx_free_balance(&0), 0);
        });
    }

    #[test]
    fn test_fee_no_blockproducer() {
        with_externalities(&mut new_test_ext(), || {
            assert_ok!(Module::<Test>::make_payment(&1, 10, 10));
            // base fee = 10, bytes fee = 1
            let fee = 10 * 10 + 1 * 10;
            assert_eq!(XAssets::pcx_free_balance(&1), 1000 - fee);
            // block producer
            // death account
            assert_eq!(XAssets::pcx_free_balance(&0), fee);
        });
    }

    #[test]
    fn test_fee_not_divisible() {
        with_externalities(&mut new_test_ext(), || {
            xsystem::BlockProducer::<Test>::put(99);
            assert_ok!(Module::<Test>::make_payment(&1, 11, 10));
            // base fee = 10, bytes fee = 1
            let fee = 10 * 10 + 1 * 11; // 111
            assert_eq!(XAssets::pcx_free_balance(&1), 1000 - fee);
            // block producer
            assert_eq!(XAssets::pcx_free_balance(&99), fee / 10); // 11
                                                                  // jackpot account
            assert_eq!(XAssets::pcx_free_balance(&1000), fee * 9 / 10 + 1); // 111 * 9 / 10 = 99 + 1 = 100
        });
    }
}
