use frame_support::dispatch::DispatchResult;
use frame_support::traits::{
    BalanceStatus, Currency, ExistenceRequirement, ReservableCurrency, SignedImbalance,
    WithdrawReason, WithdrawReasons,
};
use sp_runtime::{
    traits::{CheckedSub, Zero},
    DispatchError,
};
use sp_std::{cmp, result};

use crate::traits::ChainT;
use crate::types::{AssetType, NegativeImbalance, PositiveImbalance};
use crate::{Error, Module, Trait};

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
        _reason: WithdrawReasons,
        _new_balance: Self::Balance,
    ) -> DispatchResult {
        Ok(())
    }

    fn transfer(
        source: &T::AccountId,
        dest: &T::AccountId,
        value: Self::Balance,
        _: ExistenceRequirement,
    ) -> DispatchResult {
        Self::pcx_move_free_balance(&source, &dest, value).map_err::<Error<T>, _>(Into::into)?;
        Ok(())
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
    ) -> result::Result<Self::PositiveImbalance, DispatchError> {
        Self::inner_issue(&Self::TOKEN.to_vec(), who, AssetType::Free, value)
    }

    fn withdraw(
        _who: &T::AccountId,
        _value: Self::Balance,
        _reason: WithdrawReasons,
        _liveness: ExistenceRequirement,
    ) -> result::Result<Self::NegativeImbalance, DispatchError> {
        unimplemented!()
    }

    fn deposit_creating(_who: &T::AccountId, _value: Self::Balance) -> Self::PositiveImbalance {
        unimplemented!()
    }

    fn make_free_balance_be(
        _who: &T::AccountId,
        _balance: Self::Balance,
    ) -> SignedImbalance<Self::Balance, Self::PositiveImbalance> {
        unimplemented!()
    }

    fn burn(amount: Self::Balance) -> Self::PositiveImbalance {
        unimplemented!()
    }

    fn issue(amount: Self::Balance) -> Self::NegativeImbalance {
        unimplemented!()
    }
}

impl<T: Trait> ReservableCurrency<T::AccountId> for Module<T> {
    fn can_reserve(who: &T::AccountId, value: Self::Balance) -> bool {
        Self::free_balance(who)
            .checked_sub(&value)
            .map_or(false, |new_balance| {
                Self::ensure_can_withdraw(who, value, WithdrawReason::Reserve.into(), new_balance)
                    .is_ok()
            })
    }

    fn slash_reserved(
        _who: &T::AccountId,
        _value: Self::Balance,
    ) -> (Self::NegativeImbalance, Self::Balance) {
        unimplemented!()
    }

    fn reserved_balance(who: &T::AccountId) -> Self::Balance {
        Self::pcx_type_balance(&who, AssetType::ReservedCurrency)
    }

    fn reserve(who: &T::AccountId, value: Self::Balance) -> result::Result<(), DispatchError> {
        let b = Self::free_balance(who);
        if b < value {
            Err(Error::<T>::InsufficientBalance)?;
        }
        let new_balance = b - value;
        Self::ensure_can_withdraw(who, value, WithdrawReason::Reserve.into(), new_balance)?;
        Self::pcx_move_balance(
            who,
            AssetType::Free,
            who,
            AssetType::ReservedCurrency,
            value,
        )
        .map_err::<Error<T>, _>(Into::into)?;
        Ok(())
    }

    fn unreserve(who: &T::AccountId, value: Self::Balance) -> Self::Balance {
        let b = Self::reserved_balance(who);
        let actual = cmp::min(b, value);
        match Self::pcx_move_balance(
            who,
            AssetType::ReservedCurrency,
            who,
            AssetType::Free,
            actual,
        ) {
            Ok(()) => value - actual,
            Err(_) => Zero::zero(),
        }
    }

    fn repatriate_reserved(
        slashed: &T::AccountId,
        beneficiary: &T::AccountId,
        value: Self::Balance,
        _status: BalanceStatus,
    ) -> result::Result<Self::Balance, DispatchError> {
        let b = Self::reserved_balance(slashed);
        let slash = cmp::min(b, value);

        let _ = Self::pcx_move_balance(
            slashed,
            AssetType::ReservedCurrency,
            beneficiary,
            AssetType::Free,
            slash,
        )
        .map_err::<Error<T>, _>(Into::into)?;

        Ok(value - slash)
    }
}
