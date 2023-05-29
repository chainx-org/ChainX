// Copyright 2019-2023 ChainX Project Authors. Licensed under GPL-3.0.

//! # BTC Ledger Pallet
//!
//! The BTC Ledger Pallet provides functionality for handling accounts and btc balances.
//!

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::comparison_chain)]

#[cfg(test)]
mod tests;
#[cfg(test)]
mod mock;

pub use self::imbalances::{NegativeImbalance, PositiveImbalance};
use codec::{Codec, Decode, Encode, MaxEncodedLen};
#[cfg(feature = "std")]
use frame_support::traits::GenesisBuild;
use frame_support::{
    ensure, PalletId,
    pallet_prelude::{DispatchResult, Get},
    traits::{
        tokens::{fungible, DepositConsequence, WithdrawConsequence}, WithdrawReasons,
        Currency, ExistenceRequirement, Imbalance, SignedImbalance, TryDrop,
    },
};
use scale_info::TypeInfo;
use sp_runtime::{
    traits::{
        AtLeast32BitUnsigned, Bounded, CheckedAdd, CheckedSub, MaybeSerializeDeserialize,
        Saturating, StaticLookup, Zero, AccountIdConversion
    },
    ArithmeticError, DispatchError, RuntimeDebug,
};
use sp_std::{fmt::Debug, mem, prelude::*};

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The balance of an account.
        type Balance: Parameter
            + Member
            + AtLeast32BitUnsigned
            + Codec
            + Default
            + Copy
            + MaybeSerializeDeserialize
            + Debug
            + MaxEncodedLen
            + TypeInfo;

        /// The overarching event type.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// A majority of the council can excute some transactions.
        type CouncilOrigin: EnsureOrigin<Self::Origin>;

        /// The btc-ledger's pallet id, used for deriving its sovereign account ID.
        #[pallet::constant]
        type PalletId: Get<PalletId>;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Transfer some liquid free balance to another account.
        ///
        /// `transfer` will set the `FreeBalance` of the sender and receiver.
        /// The dispatch origin for this call must be `Signed` by the transactor.
        #[pallet::weight(0)]
        pub fn transfer(
            origin: OriginFor<T>,
            dest: <T::Lookup as StaticLookup>::Source,
            #[pallet::compact] value: T::Balance,
        ) -> DispatchResultWithPostInfo {
            let transactor = ensure_signed(origin)?;
            let dest = T::Lookup::lookup(dest)?;
            <Self as Currency<_>>::transfer(
                &transactor,
                &dest,
                value,
                ExistenceRequirement::AllowDeath,
            )?;
            Ok(().into())
        }

        /// Set the balances of a given account.
        ///
        /// This will alter `FreeBalance` in storage. it will
        /// also alter the total issuance of the system (`TotalIssuance`) appropriately.
        /// The dispatch origin for this call is `root`.
        #[pallet::weight(0)]
        pub fn set_balance(
            origin: OriginFor<T>,
            who: <T::Lookup as StaticLookup>::Source,
            #[pallet::compact] new_free: T::Balance
        ) -> DispatchResultWithPostInfo {
            T::CouncilOrigin::try_origin(origin)
                .map(|_| ())
                .or_else(ensure_root)?;

            let who = T::Lookup::lookup(who)?;

            // First we try to modify the account's balance to the forced balance.
            let old_free = Self::mutate_account(&who, |account| {
                let old_free = account.free;

                account.free = new_free;

                old_free
            })?;

            // This will adjust the total issuance, which was not done by the `mutate_account`
            // above.
            if new_free > old_free {
                mem::drop(PositiveImbalance::<T>::new(new_free - old_free));
            } else if new_free < old_free {
                mem::drop(NegativeImbalance::<T>::new(old_free - new_free));
            }

            Self::deposit_event(Event::BalanceSet { who, free: new_free });

            Ok(().into())
        }

        /// Exactly as `transfer`, except the origin must be root and
        /// the source account may be specified.
        /// The dispatch origin for this call is `root`.
        #[pallet::weight(0)]
        pub fn force_transfer(
            origin: OriginFor<T>,
            source: <T::Lookup as StaticLookup>::Source,
            dest: <T::Lookup as StaticLookup>::Source,
            #[pallet::compact] value: T::Balance,
        ) -> DispatchResultWithPostInfo {
            T::CouncilOrigin::try_origin(origin)
                .map(|_| ())
                .or_else(ensure_root)?;

            let source = T::Lookup::lookup(source)?;
            let dest = T::Lookup::lookup(dest)?;
            <Self as Currency<_>>::transfer(
                &source,
                &dest,
                value,
                ExistenceRequirement::AllowDeath,
            )?;
            Ok(().into())
        }
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// An account was created with some free balance.
        Endowed { account: T::AccountId, free_balance: T::Balance },
        /// Transfer succeeded.
        Transfer { from: T::AccountId, to: T::AccountId, amount: T::Balance },
        /// A balance was set by root.
        BalanceSet { who: T::AccountId, free: T::Balance },
        /// Some amount was deposited (e.g. for transaction fees).
        Deposit { who: T::AccountId, amount: T::Balance },
        /// Some amount was withdrawn from the account (e.g. for transaction fees).
        Withdraw { who: T::AccountId, amount: T::Balance }
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Balance too low to send value
        InsufficientBalance,
        /// Beneficiary account must pre-exist
        DeadAccount
    }

    /// The total units issued in the system.
    #[pallet::storage]
    #[pallet::getter(fn total_issuance)]
    pub type TotalIssuance<T: Config> = StorageValue<_, T::Balance, ValueQuery>;

    /// The Balances pallet example of storing the balance of an account.
    #[pallet::storage]
    pub type AccountStore<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        AccountData<T::Balance>,
        ValueQuery,
        GetDefault
    >;

    /// Storage version of the pallet.
    ///
    #[pallet::storage]
    pub(super) type StorageVersion<T: Config> = StorageValue<_, Releases, ValueQuery>;

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        pub balances: Vec<(T::AccountId, T::Balance)>,
    }

    #[cfg(feature = "std")]
    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            Self { balances: Default::default() }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
        fn build(&self) {
            let total = self.balances.iter().fold(Zero::zero(), |acc: T::Balance, &(_, n)| acc + n);

            <TotalIssuance<T>>::put(total);
            <StorageVersion<T>>::put(Releases::V1_0_0);

            // ensure no duplicates exist.
            let endowed_accounts = self
                .balances
                .iter()
                .map(|(x, _)| x)
                .cloned()
                .collect::<std::collections::BTreeSet<_>>();

            assert!(
                endowed_accounts.len() == self.balances.len(),
                "duplicate balances in genesis."
            );

            for &(ref who, free) in self.balances.iter() {
                AccountStore::<T>::insert(who, AccountData { free });
            }
        }
    }
}

#[cfg(feature = "std")]
impl<T: Config> GenesisConfig<T> {
    /// Direct implementation of `GenesisBuild::build_storage`.
    ///
    /// Kept in order not to break dependency.
    pub fn build_storage(&self) -> Result<sp_runtime::Storage, String> {
        <Self as GenesisBuild<T>>::build_storage(self)
    }

    /// Direct implementation of `GenesisBuild::assimilate_storage`.
    ///
    /// Kept in order not to break dependency.
    pub fn assimilate_storage(&self, storage: &mut sp_runtime::Storage) -> Result<(), String> {
        <Self as GenesisBuild<T>>::assimilate_storage(self, storage)
    }
}

/// All balance information for an account.
#[derive(Encode, Decode, Clone, PartialEq, Eq, Default, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct AccountData<Balance> {
    /// This is the only balance that matters in terms of most operations on tokens. It
    /// alone is used to determine the balance when in the contract execution environment.
    pub free: Balance,
}

impl<Balance: Saturating + Copy + Ord> AccountData<Balance> {
    fn total(&self) -> Balance {
        self.free
    }
}

// A value placed in storage that represents the current version of the Balances storage.
// This value is used by the `on_runtime_upgrade` logic to determine whether we run
// storage migration logic. This should match directly with the semantic versions of the Rust crate.
#[derive(Encode, Decode, Clone, Copy, PartialEq, Eq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
enum Releases {
    V1_0_0
}

impl Default for Releases {
    fn default() -> Self {
        Releases::V1_0_0
    }
}

impl<T: Config> Pallet<T> {
    pub fn account_id() -> T::AccountId {
        T::PalletId::get().into_account()
    }

    /// Get the free balance of an account.
    pub fn free_balance(who: impl sp_std::borrow::Borrow<T::AccountId>) -> T::Balance {
        Self::account(who.borrow()).free
    }

    /// Get the total balance
    pub fn get_total() -> T::Balance {
        TotalIssuance::<T>::get()
    }

    /// Get both the free balances of an account.
    fn account(who: &T::AccountId) -> AccountData<T::Balance> {
        AccountStore::<T>::get(&who)
    }

    fn deposit_consequence(
        _who: &T::AccountId,
        amount: T::Balance,
        account: &AccountData<T::Balance>,
    ) -> DepositConsequence {
        if amount.is_zero() {
            return DepositConsequence::Success
        }

        if TotalIssuance::<T>::get().checked_add(&amount).is_none() {
            return DepositConsequence::Overflow
        }

        match account.total().checked_add(&amount) {
            // NOTE: We assume that we are a self-sufficient,
            // so don't need to do any checks in the case of account creation.
            Some(_x) => DepositConsequence::Success,
            None => DepositConsequence::Overflow,
        }
    }

    fn withdraw_consequence(
        _who: &T::AccountId,
        amount: T::Balance,
        account: &AccountData<T::Balance>,
    ) -> WithdrawConsequence<T::Balance> {
        if amount.is_zero() {
            return WithdrawConsequence::Success
        }

        if TotalIssuance::<T>::get().checked_sub(&amount).is_none() {
            return WithdrawConsequence::Underflow
        }

        if account.total().checked_sub(&amount).is_none() {
            return WithdrawConsequence::NoFunds
        };

        // Enough free funds to have them be reduced.
        WithdrawConsequence::Success
    }

    /// Mutate an account to some new value
    /// NOTE: Doesn't do any preparatory work for creating a new account,
    /// so should only be used when it is known that the account already exists.
    ///
    /// NOTE: LOW-LEVEL: This will not attempt to maintain total issuance.
    /// It is expected that the caller will do this.
    pub fn mutate_account<R>(
        who: &T::AccountId,
        f: impl FnOnce(&mut AccountData<T::Balance>) -> R,
    ) -> Result<R, DispatchError> {
        Self::try_mutate_account(who, |a, _| -> Result<R, DispatchError> { Ok(f(a)) })
    }

    /// Mutate an account to some new value
    /// This will do nothing if the result of `f` is an `Err`.
    ///
    /// NOTE: Doesn't do any preparatory work for creating a new account,
    /// so should only be used when it is known that the account already exists.
    ///
    /// NOTE: LOW-LEVEL: This will not attempt to maintain total issuance.
    /// It is expected that the caller will do this.
    fn try_mutate_account<R, E: From<DispatchError>>(
        who: &T::AccountId,
        f: impl FnOnce(&mut AccountData<T::Balance>, bool) -> Result<R, E>,
    ) -> Result<R, E> {
        AccountStore::<T>::try_mutate_exists(who, |maybe_account| {
            let is_new = maybe_account.is_none();
            let mut account = maybe_account.take().unwrap_or_default();
            f(&mut account, is_new)
                .map(move |result| {
                    if is_new {
                        frame_system::Pallet::<T>::inc_sufficients(who);

                        Self::deposit_event(Event::Endowed { account: who.clone(), free_balance: account.free });
                    }

                    *maybe_account = Some(account);

                    result
                })
        })
    }
}

impl<T: Config> fungible::Inspect<T::AccountId> for Pallet<T> {
    type Balance = T::Balance;

    fn total_issuance() -> Self::Balance {
        TotalIssuance::<T>::get()
    }
    fn minimum_balance() -> Self::Balance {
        Self::Balance::default()
    }
    fn balance(who: &T::AccountId) -> Self::Balance {
        Self::account(who).total()
    }
    fn reducible_balance(who: &T::AccountId, _keep_alive: bool) -> Self::Balance {
        Self::account(who).total()
    }
    fn can_deposit(who: &T::AccountId, amount: Self::Balance) -> DepositConsequence {
        Self::deposit_consequence(who, amount, &Self::account(who))
    }
    fn can_withdraw(who: &T::AccountId, amount: Self::Balance) -> WithdrawConsequence<Self::Balance> {
        Self::withdraw_consequence(who, amount, &Self::account(who))
    }
}

impl<T: Config> fungible::Mutate<T::AccountId> for Pallet<T> {
    fn mint_into(who: &T::AccountId, amount: Self::Balance) -> DispatchResult {
        if amount.is_zero() {
            return Ok(())
        }

        Self::try_mutate_account(who, |account, _is_new| -> DispatchResult {
            Self::deposit_consequence(who, amount, account).into_result()?;
            account.free += amount;

            Ok(())
        })?;

        TotalIssuance::<T>::mutate(|t| *t += amount);

        Self::deposit_event(Event::Deposit { who: who.clone(), amount });

        Ok(())
    }

    fn burn_from(
        who: &T::AccountId,
        amount: Self::Balance,
    ) -> Result<Self::Balance, DispatchError> {
        if amount.is_zero() {
            return Ok(Self::Balance::zero())
        }

        Self::try_mutate_account(
            who,
            |account, _is_new| -> Result<T::Balance, DispatchError> {
                Self::withdraw_consequence(who, amount, account).into_result()?;
                account.free -= amount;

                Ok(amount)
            },
        )?;

        TotalIssuance::<T>::mutate(|t| *t -= amount);

        Self::deposit_event(Event::Withdraw { who: who.clone(), amount });

        Ok(amount)
    }
}

// wrapping these imbalances in a private module is necessary to ensure absolute privacy
// of the inner member.
mod imbalances {
    use super::{Config, Imbalance, RuntimeDebug, Saturating, TryDrop, Zero};
    use frame_support::traits::SameOrOther;
    use sp_std::mem;

    /// Opaque, move-only struct with private fields that serves as a token denoting that
    /// funds have been created without any equal and opposite accounting.
    #[must_use]
    #[derive(RuntimeDebug, PartialEq, Eq)]
    pub struct PositiveImbalance<T: Config>(T::Balance);

    impl<T: Config> PositiveImbalance<T> {
        /// Create a new positive imbalance from a balance.
        pub fn new(amount: T::Balance) -> Self {
            PositiveImbalance(amount)
        }
    }

    /// Opaque, move-only struct with private fields that serves as a token denoting that
    /// funds have been destroyed without any equal and opposite accounting.
    #[must_use]
    #[derive(RuntimeDebug, PartialEq, Eq)]
    pub struct NegativeImbalance<T: Config>(T::Balance);

    impl<T: Config> NegativeImbalance<T> {
        /// Create a new negative imbalance from a balance.
        pub fn new(amount: T::Balance) -> Self {
            NegativeImbalance(amount)
        }
    }

    impl<T: Config> TryDrop for PositiveImbalance<T> {
        fn try_drop(self) -> Result<(), Self> {
            self.drop_zero()
        }
    }

    impl<T: Config> Default for PositiveImbalance<T> {
        fn default() -> Self {
            Self::zero()
        }
    }

    impl<T: Config> Imbalance<T::Balance> for PositiveImbalance<T> {
        type Opposite = NegativeImbalance<T>;

        fn zero() -> Self {
            Self(Zero::zero())
        }
        fn drop_zero(self) -> Result<(), Self> {
            if self.0.is_zero() {
                Ok(())
            } else {
                Err(self)
            }
        }
        fn split(self, amount: T::Balance) -> (Self, Self) {
            let first = self.0.min(amount);
            let second = self.0 - first;

            mem::forget(self);
            (Self(first), Self(second))
        }
        fn merge(mut self, other: Self) -> Self {
            self.0 = self.0.saturating_add(other.0);
            mem::forget(other);

            self
        }
        fn subsume(&mut self, other: Self) {
            self.0 = self.0.saturating_add(other.0);
            mem::forget(other);
        }
        fn offset(self, other: Self::Opposite) -> SameOrOther<Self, Self::Opposite> {
            let (a, b) = (self.0, other.0);
            mem::forget((self, other));

            if a > b {
                SameOrOther::Same(Self(a - b))
            } else if b > a {
                SameOrOther::Other(NegativeImbalance::new(b - a))
            } else {
                SameOrOther::None
            }
        }
        fn peek(&self) -> T::Balance {
            self.0
        }
    }

    impl<T: Config> TryDrop for NegativeImbalance<T> {
        fn try_drop(self) -> Result<(), Self> {
            self.drop_zero()
        }
    }

    impl<T: Config> Default for NegativeImbalance<T> {
        fn default() -> Self {
            Self::zero()
        }
    }

    impl<T: Config> Imbalance<T::Balance> for NegativeImbalance<T> {
        type Opposite = PositiveImbalance<T>;

        fn zero() -> Self {
            Self(Zero::zero())
        }
        fn drop_zero(self) -> Result<(), Self> {
            if self.0.is_zero() {
                Ok(())
            } else {
                Err(self)
            }
        }
        fn split(self, amount: T::Balance) -> (Self, Self) {
            let first = self.0.min(amount);
            let second = self.0 - first;

            mem::forget(self);
            (Self(first), Self(second))
        }
        fn merge(mut self, other: Self) -> Self {
            self.0 = self.0.saturating_add(other.0);
            mem::forget(other);

            self
        }
        fn subsume(&mut self, other: Self) {
            self.0 = self.0.saturating_add(other.0);
            mem::forget(other);
        }
        fn offset(self, other: Self::Opposite) -> SameOrOther<Self, Self::Opposite> {
            let (a, b) = (self.0, other.0);
            mem::forget((self, other));

            if a > b {
                SameOrOther::Same(Self(a - b))
            } else if b > a {
                SameOrOther::Other(PositiveImbalance::new(b - a))
            } else {
                SameOrOther::None
            }
        }
        fn peek(&self) -> T::Balance {
            self.0
        }
    }

    impl<T: Config> Drop for PositiveImbalance<T> {
        /// Basic drop handler will just square up the total issuance.
        fn drop(&mut self) {
            <super::TotalIssuance<T>>::mutate(|v| *v = v.saturating_add(self.0));
        }
    }

    impl<T: Config> Drop for NegativeImbalance<T> {
        /// Basic drop handler will just square up the total issuance.
        fn drop(&mut self) {
            <super::TotalIssuance<T>>::mutate(|v| *v = v.saturating_sub(self.0));
        }
    }
}

impl<T: Config> Currency<T::AccountId> for Pallet<T>
where
    T::Balance: MaybeSerializeDeserialize + Debug,
{
    type Balance = T::Balance;
    type PositiveImbalance = PositiveImbalance<T>;
    type NegativeImbalance = NegativeImbalance<T>;

    fn total_balance(who: &T::AccountId) -> Self::Balance {
        Self::account(who).total()
    }

    fn can_slash(_who: &T::AccountId, _value: Self::Balance) -> bool {
        false
    }

    fn total_issuance() -> Self::Balance {
        <TotalIssuance<T>>::get()
    }

    fn minimum_balance() -> Self::Balance {
        Self::Balance::default()
    }

    // Burn funds from the total issuance, returning a positive imbalance for the amount burned.
    // Is a no-op if amount to be burned is zero.
    fn burn(mut amount: Self::Balance) -> Self::PositiveImbalance {
        if amount.is_zero() {
            return PositiveImbalance::zero()
        }
        <TotalIssuance<T>>::mutate(|issued| {
            *issued = issued.checked_sub(&amount).unwrap_or_else(|| {
                amount = *issued;
                Zero::zero()
            });
        });
        PositiveImbalance::new(amount)
    }

    // Create new funds into the total issuance, returning a negative imbalance
    // for the amount issued.
    // Is a no-op if amount to be issued it zero.
    fn issue(mut amount: Self::Balance) -> Self::NegativeImbalance {
        if amount.is_zero() {
            return NegativeImbalance::zero()
        }
        <TotalIssuance<T>>::mutate(|issued| {
            *issued = issued.checked_add(&amount).unwrap_or_else(|| {
                amount = Self::Balance::max_value() - *issued;
                Self::Balance::max_value()
            })
        });
        NegativeImbalance::new(amount)
    }

    fn free_balance(who: &T::AccountId) -> Self::Balance {
        Self::account(who).free
    }

    fn ensure_can_withdraw(
        _who: &T::AccountId,
        _amount: T::Balance,
        _reasons: WithdrawReasons,
        _new_balance: T::Balance,
    ) -> DispatchResult {
        Ok(())
    }

    // Transfer some free balance from `transactor` to `dest`, respecting existence requirements.
    // Is a no-op if value to be transferred is zero or the `transactor` is the same as `dest`.
    fn transfer(
        transactor: &T::AccountId,
        dest: &T::AccountId,
        value: Self::Balance,
        _existence_requirement: ExistenceRequirement,
    ) -> DispatchResult {
        if value.is_zero() || transactor == dest {
            return Ok(())
        }

        Self::try_mutate_account(
            dest,
            |to_account, _| -> DispatchResult {
                Self::try_mutate_account(
                    transactor,
                    |from_account, _| -> DispatchResult {
                        from_account.free = from_account
                            .free
                            .checked_sub(&value)
                            .ok_or(Error::<T>::InsufficientBalance)?;

                        // NOTE: total stake being stored in the same type means that this could
                        // never overflow but better to be safe than sorry.
                        to_account.free = to_account
                            .free
                            .checked_add(&value)
                            .ok_or(ArithmeticError::Overflow)?;

                        Self::ensure_can_withdraw(
                            transactor,
                            value,
                            WithdrawReasons::TRANSFER,
                            from_account.free,
                        )?;

                        Ok(())
                    },
                )
            },
        )?;

        // Emit transfer event.
        Self::deposit_event(Event::Transfer {
            from: transactor.clone(),
            to: dest.clone(),
            amount: value,
        });

        Ok(())
    }

    fn slash(_who: &T::AccountId, value: Self::Balance) -> (Self::NegativeImbalance, Self::Balance) {
        (Self::NegativeImbalance::zero(), value)
    }

    /// Deposit some `value` into the free balance of an existing target account `who`.
    ///
    /// Is a no-op if the `value` to be deposited is zero.
    fn deposit_into_existing(
        who: &T::AccountId,
        value: Self::Balance,
    ) -> Result<Self::PositiveImbalance, DispatchError> {
        if value.is_zero() {
            return Ok(PositiveImbalance::zero())
        }

        Self::try_mutate_account(
            who,
            |account, is_new| -> Result<Self::PositiveImbalance, DispatchError> {
                ensure!(!is_new, Error::<T>::DeadAccount);
                account.free = account.free.checked_add(&value).ok_or(ArithmeticError::Overflow)?;
                Self::deposit_event(Event::Deposit { who: who.clone(), amount: value });
                Ok(PositiveImbalance::new(value))
            },
        )
    }

    /// Deposit some `value` into the free balance of `who`, possibly adding a new account.
    ///
    /// This function is a no-op if:
    /// - the `value` to be deposited is zero; or
    /// - `value` is so large it would cause the balance of `who` to overflow.
    fn deposit_creating(who: &T::AccountId, value: Self::Balance) -> Self::PositiveImbalance {
        if value.is_zero() {
            return Self::PositiveImbalance::zero()
        }

        Self::try_mutate_account(
            who,
            |account, _| -> Result<Self::PositiveImbalance, DispatchError> {
                // defensive only: overflow should never happen, however in case it does, then this
                // operation is a no-op.
                account.free = match account.free.checked_add(&value) {
                    Some(x) => x,
                    None => return Ok(Self::PositiveImbalance::zero()),
                };

                Self::deposit_event(Event::Deposit { who: who.clone(), amount: value });

                Ok(PositiveImbalance::new(value))
            },
        )
        .unwrap_or_default()
    }

    /// Withdraw some free balance from an account, respecting existence requirements.
    ///
    /// Is a no-op if value to be withdrawn is zero.
    fn withdraw(
        who: &T::AccountId,
        value: Self::Balance,
        reasons: WithdrawReasons,
        _liveness: ExistenceRequirement,
    ) -> Result<Self::NegativeImbalance, DispatchError> {
        if value.is_zero() {
            return Ok(NegativeImbalance::zero())
        }

        Self::try_mutate_account(
            who,
            |account, _| -> Result<Self::NegativeImbalance, DispatchError> {
                let new_free_account = account
                    .free
                    .checked_sub(&value)
                    .ok_or(Error::<T>::InsufficientBalance)?;

                Self::ensure_can_withdraw(who, value, reasons, new_free_account)?;

                account.free = new_free_account;

                Self::deposit_event(Event::Withdraw { who: who.clone(), amount: value });
                Ok(NegativeImbalance::new(value))
            },
        )
    }

    /// Force the new free balance of a target account `who` to some new value `balance`.
    fn make_free_balance_be(
        who: &T::AccountId,
        value: Self::Balance,
    ) -> SignedImbalance<Self::Balance, Self::PositiveImbalance> {
        Self::try_mutate_account(
            who,
            |account, _|
             -> Result<SignedImbalance<Self::Balance, Self::PositiveImbalance>, DispatchError> {
                let imbalance = if account.free <= value {
                    SignedImbalance::Positive(PositiveImbalance::new(value - account.free))
                } else {
                    SignedImbalance::Negative(NegativeImbalance::new(account.free - value))
                };
                account.free = value;
                Self::deposit_event(Event::BalanceSet {
                    who: who.clone(),
                    free: account.free
                });
                Ok(imbalance)
            },
        )
        .unwrap_or_else(|_| SignedImbalance::Positive(Self::PositiveImbalance::zero()))
    }
}

