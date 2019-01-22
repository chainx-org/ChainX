// Copyright 2018 Chainpool.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate sr_std as rstd;

extern crate parity_codec as codec;
extern crate sr_primitives;

extern crate srml_balances as balances;
extern crate srml_system as system;
#[macro_use]
extern crate srml_support as support;

extern crate xrml_xassets_assets as xassets;
extern crate xrml_xsystem as xsystem;

use rstd::prelude::*;
use rstd::result::Result as StdResult;

use sr_primitives::traits::{As, Zero};
use support::dispatch::Result;

/// Simple payment making trait, operating on a single generic `AccountId` type.
pub trait MakePayment<AccountId> {
    /// Make some sort of payment concerning `who` for an extrinsic (transaction) of encoded length
    /// `encoded_len` bytes. Return true iff the payment was successful.
    fn make_payment(who: &AccountId, encoded_len: usize, pay: u64) -> Result;

    fn check_payment(who: &AccountId, encoded_len: usize, pay: u64) -> Result;
}

pub trait Trait: xassets::Trait + xsystem::Trait {}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as XFeeManager {
        pub Switch get(switch) config(): bool; // Emergency control
    }
}

impl<T: Trait> MakePayment<T::AccountId> for Module<T> {
    fn make_payment(transactor: &T::AccountId, encoded_len: usize, power: u64) -> Result {
        let b = Self::calc_fee_and_check(transactor, encoded_len, power)?;

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
        let b = xassets::Module::<T>::pcx_free_balance(transactor);

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
        // 10% for block producer
        if let Some(p) = xsystem::Module::<T>::block_producer() {
            v.push((1, p));
        } else {
            v.push((1, xsystem::Module::<T>::death_account()));
        }
        // 90% for death account
        v.push((9, xsystem::Module::<T>::death_account()));

        Self::calc_fee_withaccount(from_who, fee, v.as_slice())
    }

    fn calc_fee_withaccount(
        from: &T::AccountId,
        fee: T::Balance,
        to_list: &[(usize, T::AccountId)],
    ) -> Result {
        assert!(
            to_list.iter().fold(0, |acc, (rate, _)| acc + rate) == 10,
            "the rate sum must be 10 part."
        );

        let mut v = Vec::new();
        let mut fee_sum: T::Balance = Zero::zero();
        for (r, accountid) in to_list[1..].iter() {
            let a: T::Balance = As::sa(*r);
            let fee = a * fee / As::sa(10);
            fee_sum += fee;
            v.push((fee, accountid));
        }
        v.insert(0, (fee - fee_sum, &to_list[0].1));

        let chainx = <xassets::Module<T> as xassets::ChainT>::TOKEN.to_vec();
        for (fee, to) in v {
            // do not handle err
            xassets::Module::<T>::move_free_balance(&chainx, from, &to, fee)
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
    use sr_primitives::traits::BlakeTwo256;
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
        type Header = Header;
        type Event = ();
        type Log = DigestItem;
    }

    impl balances::Trait for Test {
        type Balance = u64;
        type AccountIndex = u64;
        type OnFreeBalanceZero = ();
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
        const XSYSTEM_SET_POSITION: u32 = 1;
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
                reclaim_rebate: 0,
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
                banned_account: 1000,
            }
            .build_storage()
            .unwrap()
            .0,
        );
        // xassets
        r.extend(
            xassets::GenesisConfig::<Test> {
                pcx: (3, b"PCX onchain token".to_vec()),
                memo_len: 128,
                // asset, is_psedu_intention, init for account
                // Vec<(Asset, bool, Vec<(T::AccountId, u64)>)>;
                asset_list: vec![],
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
            xsystem::BlockProdocer::<Test>::put(99);

            assert_ok!(Module::<Test>::make_payment(&1, 10, 10));
            // base fee = 10, bytes fee = 1
            let fee = 10 * 10 + 1 * 10;
            assert_eq!(XAssets::pcx_free_balance(&1), 1000 - fee);
            // block producer
            assert_eq!(XAssets::pcx_free_balance(&99), fee / 10);
            // death account
            assert_eq!(XAssets::pcx_free_balance(&0), fee * 9 / 10);
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
            xsystem::BlockProdocer::<Test>::put(99);
            assert_ok!(Module::<Test>::make_payment(&1, 11, 10));
            // base fee = 10, bytes fee = 1
            let fee = 10 * 10 + 1 * 11;
            assert_eq!(XAssets::pcx_free_balance(&1), 1000 - fee);
            // block producer
            assert_eq!(XAssets::pcx_free_balance(&99), fee / 10 + 1);
            // death account
            assert_eq!(XAssets::pcx_free_balance(&0), fee * 9 / 10);
        });
    }
}
