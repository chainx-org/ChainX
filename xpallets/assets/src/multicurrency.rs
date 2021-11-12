// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use sp_runtime::{
    traits::{CheckedSub, Saturating, Zero},
    DispatchError, DispatchResult,
};
use sp_std::{convert::TryInto, prelude::*};

use frame_support::{
    ensure,
    log::error,
    traits::{BalanceStatus, LockIdentifier},
    transactional,
};

use orml_traits::{
    currency::TransferAll, arithmetic::Signed, MultiCurrency, MultiCurrencyExtended,
    MultiLockableCurrency, MultiReservableCurrency,
};

use chainx_primitives::AssetId;
use xpallet_support::traits::TreasuryAccount;

use crate::types::{AssetType, BalanceLock};
use crate::{AssetBalance, BalanceOf, Config, Error, Pallet};

impl<T: Config> MultiCurrency<T::AccountId> for Pallet<T> {
    type CurrencyId = AssetId;
    type Balance = BalanceOf<T>;

    fn minimum_balance(_currency_id: Self::CurrencyId) -> Self::Balance {
        Zero::zero()
    }

    fn total_issuance(currency_id: Self::CurrencyId) -> Self::Balance {
        Self::total_issuance(&currency_id)
    }

    fn total_balance(currency_id: Self::CurrencyId, who: &T::AccountId) -> Self::Balance {
        Self::all_type_asset_balance(who, &currency_id)
    }

    fn free_balance(currency_id: Self::CurrencyId, who: &T::AccountId) -> Self::Balance {
        Self::asset_typed_balance(&who, &currency_id, AssetType::Usable)
            + Self::asset_typed_balance(&who, &currency_id, AssetType::Locked)
    }

    fn ensure_can_withdraw(
        currency_id: Self::CurrencyId,
        who: &T::AccountId,
        amount: Self::Balance,
    ) -> DispatchResult {
        if amount.is_zero() {
            return Ok(());
        }

        let new_balance = Self::free_balance(currency_id, who)
            .checked_sub(&amount)
            .ok_or(Error::<T>::InsufficientBalance)?;
        ensure!(
            new_balance >= Self::asset_balance_of(who, &currency_id, AssetType::Locked),
            Error::<T>::LiquidityRestrictions
        );
        Ok(())
    }

    fn transfer(
        currency_id: Self::CurrencyId,
        from: &T::AccountId,
        to: &T::AccountId,
        amount: Self::Balance,
    ) -> DispatchResult {
        if amount.is_zero() || from == to {
            return Ok(());
        }
        Self::ensure_can_withdraw(currency_id, from, amount)?;
        Self::move_usable_balance(&currency_id, from, to, amount)
            .map_err::<Error<T>, _>(Into::into)?;
        Ok(())
    }

    fn deposit(
        currency_id: Self::CurrencyId,
        who: &T::AccountId,
        amount: Self::Balance,
    ) -> DispatchResult {
        Self::issue(&currency_id, who, amount)
    }

    fn withdraw(
        currency_id: Self::CurrencyId,
        who: &T::AccountId,
        amount: Self::Balance,
    ) -> DispatchResult {
        if amount.is_zero() {
            return Ok(());
        }
        Self::ensure_can_withdraw(currency_id, who, amount)?;
        match Self::can_destroy_usable(&currency_id) {
            Ok(()) => Self::destroy_usable(&currency_id, who, amount),
            Err(_) => {
                Self::move_balance(
                    &currency_id,
                    who,
                    AssetType::Usable,
                    who,
                    AssetType::ReservedWithdrawal,
                    amount,
                )
                .map_err::<Error<T>, _>(Into::into)?;
                Self::destroy_reserved_withdrawal(&currency_id, who, amount)
            }
        }
    }

    fn can_slash(currency_id: Self::CurrencyId, who: &T::AccountId, value: Self::Balance) -> bool {
        if value.is_zero() {
            return true;
        }
        Self::free_balance(currency_id, who) >= value
    }

    fn slash(
        currency_id: Self::CurrencyId,
        who: &T::AccountId,
        amount: Self::Balance,
    ) -> Self::Balance {
        if amount.is_zero() {
            return amount;
        }
        let treasury = T::TreasuryAccount::treasury_account();

        let slash_func =
            |remaining_slash: BalanceOf<T>, type_: AssetType| -> Option<BalanceOf<T>> {
                let mut remaining_slash = remaining_slash;
                if !remaining_slash.is_zero() {
                    let slashed = Self::asset_balance_of(who, &currency_id, type_);
                    let slashed_amount = slashed.min(remaining_slash);
                    remaining_slash -= slashed_amount;
                    // no matter what type asset, all move to treasury usable type
                    let _ = Self::move_balance(
                        &currency_id,
                        who,
                        type_,
                        &treasury,
                        AssetType::Usable,
                        slashed_amount,
                    )
                    .ok()?;
                }
                Some(remaining_slash)
            };

        let mut remaining_slash = amount;
        // slash usable balance
        remaining_slash = match slash_func(remaining_slash, AssetType::Usable) {
            Some(remained) => remained,
            None => return remaining_slash,
        };

        // slash locked balance
        remaining_slash = match slash_func(remaining_slash, AssetType::Locked) {
            Some(remained) => remained,
            None => return remaining_slash,
        };

        // slash reserved balance
        remaining_slash = match slash_func(remaining_slash, AssetType::Reserved) {
            Some(remained) => remained,
            None => return remaining_slash,
        };
        remaining_slash
    }
}

impl<T: Config> MultiCurrencyExtended<T::AccountId> for Pallet<T> {
    type Amount = T::Amount;

    fn update_balance(
        currency_id: Self::CurrencyId,
        who: &T::AccountId,
        by_amount: Self::Amount,
    ) -> DispatchResult {
        if by_amount.is_zero() {
            return Ok(());
        }

        let by_balance = TryInto::<Self::Balance>::try_into(by_amount.abs())
            .map_err(|_| Error::<T>::AmountIntoBalanceFailed)?;
        if by_amount.is_positive() {
            Self::deposit(currency_id, who, by_balance)
        } else {
            Self::withdraw(currency_id, who, by_balance)
        }
    }
}

impl<T: Config> MultiReservableCurrency<T::AccountId> for Pallet<T> {
    fn can_reserve(
        currency_id: Self::CurrencyId,
        who: &T::AccountId,
        value: Self::Balance,
    ) -> bool {
        if value.is_zero() {
            return true;
        }
        Self::ensure_can_withdraw(currency_id, who, value).is_ok()
    }

    fn slash_reserved(
        currency_id: Self::CurrencyId,
        who: &T::AccountId,
        value: Self::Balance,
    ) -> Self::Balance {
        if value.is_zero() {
            return Zero::zero();
        }

        let reserved_balance = Self::reserved_balance(currency_id, who);
        let actual = reserved_balance.min(value);

        let treasury = T::TreasuryAccount::treasury_account();
        if let Err(err) = Self::move_balance(
            &currency_id,
            who,
            AssetType::Reserved,
            &treasury,
            AssetType::Usable,
            actual,
        ) {
            error!(
                target: "runtime::assets",
                "[slash_reserved] Should not be failed when move asset (reserved => usable), \
                who:{:?}, id:{}, err:{:?}",
                who, currency_id, err
            );
        }
        value - actual
    }

    fn reserved_balance(currency_id: Self::CurrencyId, who: &T::AccountId) -> Self::Balance {
        Self::asset_balance_of(who, &currency_id, AssetType::Reserved)
    }

    fn reserve(
        currency_id: Self::CurrencyId,
        who: &T::AccountId,
        value: Self::Balance,
    ) -> DispatchResult {
        if value.is_zero() {
            return Ok(());
        }
        Self::move_balance(
            &currency_id,
            who,
            AssetType::Usable,
            who,
            AssetType::Reserved,
            value,
        )
        .map_err::<Error<T>, _>(Into::into)?;
        Ok(())
    }

    fn unreserve(
        currency_id: Self::CurrencyId,
        who: &T::AccountId,
        value: Self::Balance,
    ) -> Self::Balance {
        if value.is_zero() {
            return Zero::zero();
        }
        let actual = Self::reserved_balance(currency_id, who).min(value);
        if let Err(err) = Self::move_balance(
            &currency_id,
            who,
            AssetType::Reserved,
            who,
            AssetType::Usable,
            actual,
        ) {
            error!(
                target: "runtime::assets",
                "[unreserve] Should not be failed when move asset (reserved => usable), \
                who:{:?}, id:{}, err:{:?}",
                who, currency_id, err
            );
        }
        value - actual
    }

    fn repatriate_reserved(
        currency_id: Self::CurrencyId,
        slashed: &T::AccountId,
        beneficiary: &T::AccountId,
        value: Self::Balance,
        status: BalanceStatus,
    ) -> Result<Self::Balance, DispatchError> {
        if value.is_zero() {
            return Ok(Zero::zero());
        }
        if slashed == beneficiary {
            return match status {
                BalanceStatus::Free => Ok(Self::unreserve(currency_id, slashed, value)),
                BalanceStatus::Reserved => {
                    Ok(value.saturating_sub(Self::reserved_balance(currency_id, slashed)))
                }
            };
        }

        let actual = Self::reserved_balance(currency_id, slashed).min(value);
        let to_type = match status {
            BalanceStatus::Free => AssetType::Usable,
            BalanceStatus::Reserved => AssetType::Reserved,
        };
        Self::move_balance(
            &currency_id,
            slashed,
            AssetType::Reserved,
            beneficiary,
            to_type,
            actual,
        )
        .map_err::<Error<T>, _>(Into::into)?;
        Ok(value - actual)
    }
}

impl<T: Config> MultiLockableCurrency<T::AccountId> for Pallet<T> {
    type Moment = T::BlockNumber;

    fn set_lock(
        lock_id: LockIdentifier,
        currency_id: Self::CurrencyId,
        who: &T::AccountId,
        amount: Self::Balance,
    ) -> DispatchResult {
        if amount.is_zero() {
            return Ok(());
        }
        let mut new_lock = Some(BalanceLock {
            id: lock_id,
            amount,
        });
        let mut locks = Self::locks(who, currency_id)
            .into_iter()
            .filter_map(|lock| {
                if lock.id == lock_id {
                    new_lock.take()
                } else {
                    Some(lock)
                }
            })
            .collect::<Vec<_>>();
        if let Some(lock) = new_lock {
            locks.push(lock)
        }
        Self::update_locks(currency_id, who, &locks[..]);
        Ok(())
    }

    fn extend_lock(
        lock_id: LockIdentifier,
        currency_id: Self::CurrencyId,
        who: &T::AccountId,
        amount: Self::Balance,
    ) -> DispatchResult {
        if amount.is_zero() {
            return Ok(());
        }
        let mut new_lock = Some(BalanceLock {
            id: lock_id,
            amount,
        });
        let mut locks = Self::locks(who, currency_id)
            .into_iter()
            .filter_map(|lock| {
                if lock.id == lock_id {
                    new_lock.take().map(|nl| BalanceLock {
                        id: lock.id,
                        amount: lock.amount.max(nl.amount),
                    })
                } else {
                    Some(lock)
                }
            })
            .collect::<Vec<_>>();
        if let Some(lock) = new_lock {
            locks.push(lock)
        }
        Self::update_locks(currency_id, who, &locks[..]);
        Ok(())
    }

    fn remove_lock(
        lock_id: LockIdentifier,
        currency_id: Self::CurrencyId,
        who: &T::AccountId,
    ) -> DispatchResult {
        let mut locks = Self::locks(who, currency_id);
        locks.retain(|lock| lock.id != lock_id);
        Self::update_locks(currency_id, who, &locks[..]);
        Ok(())
    }
}

impl<T: Config> TransferAll<T::AccountId> for Pallet<T> {
    #[transactional]
    fn transfer_all(source: &T::AccountId, dest: &T::AccountId) -> DispatchResult {
        AssetBalance::<T>::iter_prefix(source).try_for_each(
            |(currency_id, _account_data)| -> DispatchResult {
                // ensure the account has no active reserved of non-native token
                //
                // TODO: Should use Reserved + ReservedWithdrawal + ReservedWithdrawal?
                ensure!(
                    Self::reserved_balance(currency_id, source).is_zero(),
                    Error::<T>::StillHasActiveReserved
                );

                // transfer all free to recipient
                <Self as MultiCurrency<T::AccountId>>::transfer(
                    currency_id,
                    source,
                    dest,
                    Self::usable_balance(source, &currency_id),
                )?;
                Ok(())
            },
        )
    }
}
