use primitives::traits::{CheckedSub, Saturating, Zero};
use rstd::{cmp, result};
use support::dispatch::Result;
use support::traits::{
    Currency, ExistenceRequirement, Imbalance, ReservableCurrency, SignedImbalance,
    UpdateBalanceOutcome, WithdrawReason,
};

use crate::traits::ChainT;
use crate::types::{AssetType, NegativeImbalance, PositiveImbalance};
use crate::{Module, Trait};

impl<T: Trait> Currency<T::AccountId> for Module<T> {
    type Balance = T::Balance;
    type PositiveImbalance = PositiveImbalance<T>;
    type NegativeImbalance = NegativeImbalance<T>;

    fn total_balance(who: &T::AccountId) -> Self::Balance {
        Self::pcx_all_type_balance(who)
    }

    fn can_slash(who: &T::AccountId, value: Self::Balance) -> bool {
        Self::free_balance(who) >= value
    }

    fn total_issuance() -> Self::Balance {
        Self::pcx_total_balance()
    }

    fn minimum_balance() -> Self::Balance {
        Zero::zero()
    }

    fn free_balance(who: &T::AccountId) -> Self::Balance {
        Self::pcx_free_balance(who)
    }

    fn ensure_can_withdraw(
        _who: &T::AccountId,
        _amount: Self::Balance,
        _reason: WithdrawReason,
        _new_balance: Self::Balance,
    ) -> Result {
        Ok(())
    }

    fn transfer(source: &T::AccountId, dest: &T::AccountId, value: Self::Balance) -> Result {
        Self::pcx_move_free_balance(&source, &dest, value).map_err(|e| e.info())
    }

    fn slash(
        _who: &T::AccountId,
        _value: Self::Balance,
    ) -> (Self::NegativeImbalance, Self::Balance) {
        unimplemented!()
    }

    fn deposit_into_existing(
        who: &T::AccountId,
        value: Self::Balance,
    ) -> result::Result<Self::PositiveImbalance, &'static str> {
        Self::inner_issue(&Self::TOKEN.to_vec(), who, AssetType::Free, value)
    }

    fn withdraw(
        who: &T::AccountId,
        value: Self::Balance,
        reason: WithdrawReason,
        liveness: ExistenceRequirement,
    ) -> result::Result<Self::NegativeImbalance, &'static str> {
        if let Some(new_balance) = Self::free_balance(who).checked_sub(&value) {
            if liveness == ExistenceRequirement::KeepAlive {
                return Err("payment would kill account");
            }
            Self::ensure_can_withdraw(who, value, reason, new_balance)?;
            let (imbalance, _) = Self::make_free_balance_be(who, new_balance);

            let negative = if let SignedImbalance::Negative(n) = imbalance {
                n
            } else {
                // Impossible, but be defensive.
                NegativeImbalance::<T>::new(Zero::zero(), Self::TOKEN.to_vec(), AssetType::Free)
            };
            Ok(negative)
        } else {
            Err("too few free funds in account")
        }
    }

    fn deposit_creating(who: &T::AccountId, value: Self::Balance) -> Self::PositiveImbalance {
        Self::inner_issue(&Self::TOKEN.to_vec(), who, AssetType::Free, value)
            .unwrap_or(PositiveImbalance::<T>::zero())
    }

    fn make_free_balance_be(
        who: &T::AccountId,
        balance: Self::Balance,
    ) -> (
        SignedImbalance<Self::Balance, Self::PositiveImbalance>,
        UpdateBalanceOutcome,
    ) {
        let key = (who.clone(), Self::TOKEN.to_vec());
        (
            Self::make_type_balance_be(&key, AssetType::Free, balance),
            UpdateBalanceOutcome::Updated,
        )
    }
}

impl<T: Trait> ReservableCurrency<T::AccountId> for Module<T> {
    fn can_reserve(who: &T::AccountId, value: Self::Balance) -> bool {
        Self::free_balance(who)
            .checked_sub(&value)
            .map_or(false, |new_balance| {
                Self::ensure_can_withdraw(who, value, WithdrawReason::Reserve, new_balance).is_ok()
            })
    }

    fn slash_reserved(
        who: &T::AccountId,
        value: Self::Balance,
    ) -> (Self::NegativeImbalance, Self::Balance) {
        let b = Self::reserved_balance(who);
        let slash = cmp::min(b, value);
        // underflow should never happen, but it if does, there's nothing to be done here.

        let key = (who.clone(), Self::TOKEN.to_vec());
        let type_ = AssetType::ReservedCurrency;
        Self::make_type_balance_be(&key, type_, b.saturating_sub(slash));

        (
            NegativeImbalance::new(slash, Self::TOKEN.to_vec(), type_),
            value - slash,
        )
    }

    fn reserved_balance(who: &T::AccountId) -> Self::Balance {
        Self::pcx_type_balance(&who, AssetType::ReservedCurrency)
    }

    fn reserve(who: &T::AccountId, value: Self::Balance) -> result::Result<(), &'static str> {
        let b = Self::free_balance(who);
        if b < value {
            return Err("not enough free funds");
        }
        let new_balance = b - value;
        Self::ensure_can_withdraw(who, value, WithdrawReason::Reserve, new_balance)?;
        Self::pcx_move_balance(
            who,
            AssetType::Free,
            who,
            AssetType::ReservedCurrency,
            value,
        )
        .map_err(|e| e.info())
    }

    fn unreserve(who: &T::AccountId, value: Self::Balance) -> Self::Balance {
        let b = Self::reserved_balance(who);
        let actual = cmp::min(b, value);
        let _ = Self::pcx_move_balance(
            who,
            AssetType::ReservedCurrency,
            who,
            AssetType::Free,
            actual,
        );
        value - actual
    }

    fn repatriate_reserved(
        slashed: &T::AccountId,
        beneficiary: &T::AccountId,
        value: Self::Balance,
    ) -> result::Result<Self::Balance, &'static str> {
        let b = Self::reserved_balance(slashed);
        let slash = cmp::min(b, value);

        let _ = Self::pcx_move_balance(
            slashed,
            AssetType::ReservedCurrency,
            beneficiary,
            AssetType::Free,
            slash,
        );

        Ok(value - slash)
    }
}
