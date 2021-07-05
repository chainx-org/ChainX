use frame_support::traits::BalanceStatus;
use sp_arithmetic::traits::Saturating;

use frame_support::{
    dispatch::DispatchResult,
    ensure,
    traits::{Get, ReservableCurrency},
};

use crate::pallet::{BalanceOf, Collaterals, Config, CurrencyOf, Error, Pallet};

/// Collateral related stuff.
///
/// It wraps `Currency` operations to modify `Collaterals` storage.
impl<T: Config<I>, I: 'static> Pallet<T, I> {
    /// Collateral of `vault`
    pub(crate) fn collateral_of(vault: &T::AccountId) -> BalanceOf<T> {
        Pallet::<T, I>::collaterals(vault)
    }

    /// Lock `vault`'s native asset(aka pcx) as collateral.
    ///
    /// WARN: Only vault should use the function. Other locking operations should use `Currency::reserve` directly.
    pub(crate) fn lock_collateral(vault: &T::AccountId, amount: BalanceOf<T>) -> DispatchResult {
        CurrencyOf::<T>::reserve(vault, amount)?;
        Collaterals::<T, I>::mutate(vault, |collateral| {
            *collateral = collateral.saturating_add(amount)
        });
        Ok(())
    }

    /// Slash `vault`'s collateral to `requester`.
    ///
    /// Only vault could be slashed.
    pub(crate) fn slash_vault(
        vault: &T::AccountId,
        requester: &T::AccountId,
        amount: BalanceOf<T>,
    ) -> DispatchResult {
        let reserved_collateral = Self::collateral_of(vault);
        ensure!(
            reserved_collateral >= amount,
            Error::<T, I>::InsufficientCollateral
        );
        CurrencyOf::<T>::repatriate_reserved(vault, requester, amount, BalanceStatus::Free)?;
        Collaterals::<T, I>::mutate(vault, |collateral| {
            *collateral = collateral.saturating_sub(amount)
        });
        Ok(())
    }
}

impl<T: Config<I>, I: 'static> Pallet<T, I> {
    #[inline]
    pub(crate) fn target_asset_of(who: &T::AccountId) -> BalanceOf<T> {
        xpallet_assets::Pallet::<T>::usable_balance(who, &T::TargetAssetId::get())
    }

    #[inline]
    pub(crate) fn lock_asset(who: &T::AccountId, amount: BalanceOf<T>) -> DispatchResult {
        xpallet_assets::Pallet::<T>::move_balance(
            &T::TargetAssetId::get(),
            who,
            xpallet_assets::AssetType::Usable,
            who,
            xpallet_assets::AssetType::ReservedWithdrawal,
            amount,
        )
        .map_err(|_| Error::<T, I>::AssetError)?;
        Ok(())
    }

    #[inline]
    pub(crate) fn release_asset(who: &T::AccountId, amount: BalanceOf<T>) -> DispatchResult {
        xpallet_assets::Pallet::<T>::move_balance(
            &T::TargetAssetId::get(),
            who,
            xpallet_assets::AssetType::ReservedWithdrawal,
            who,
            xpallet_assets::AssetType::Usable,
            amount,
        )
        .map_err(|_| Error::<T, I>::AssetError)?;
        Ok(())
    }

    /// Mint `amount` of asset to `who` by 'by'.
    ///
    /// WARN: It will decrease the `to_be_issued_tokens`. 
    pub(crate) fn mint(
        who: &T::AccountId,
        by: &T::AccountId,
        amount: BalanceOf<T>,
    ) -> DispatchResult {
        xpallet_assets::Pallet::<T>::issue(&T::TargetAssetId::get(), who, amount)?;
        Self::process_vault_issue(by, amount);
        Ok(())
    }


    /// Burn `amount` of asset from `who` by `by`.
    /// 
    /// WARN: It will decrease the `to_be_redeemed_tokens`. 
    pub(crate) fn burn(
        who: &T::AccountId,
        by: &T::AccountId,
        amount: BalanceOf<T>,
    ) -> DispatchResult {
        xpallet_assets::Pallet::<T>::destroy_reserved_withdrawal(
            &T::TargetAssetId::get(),
            who,
            amount,
        )?;
        Self::process_vault_redeem(by, amount);
        Ok(())
    }
}
