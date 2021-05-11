use frame_support::{
    dispatch::DispatchResult,
    ensure,
    traits::{Currency, Get, ReservableCurrency},
};

use sp_arithmetic::traits::Saturating;

use crate::pallet::{BalanceOf, Collaterals, Config, CurrencyOf, Error, Pallet, TotalCollateral};
use crate::traits::BridgeAssetManager;
use crate::traits::MultiCollateral;

impl<T: Config<I>, I: 'static> MultiCollateral<BalanceOf<T>, T::AccountId> for Pallet<T, I> {
    fn total() -> BalanceOf<T> {
        Self::total_collateral()
    }

    fn collateral_of(vault: &T::AccountId) -> BalanceOf<T> {
        Self::collaterals(vault)
    }

    fn lock(vault: &T::AccountId, amount: BalanceOf<T>) -> DispatchResult {
        CurrencyOf::<T>::reserve(vault, amount)?;
        TotalCollateral::<T, I>::mutate(|total| *total = total.saturating_add(amount));
        Collaterals::<T, I>::mutate(vault, |collateral| {
            *collateral = collateral.saturating_add(amount)
        });
        Ok(())
    }

    fn slash(
        sender: &T::AccountId,
        receiver: &T::AccountId,
        amount: BalanceOf<T>,
    ) -> DispatchResult {
        let reserved_collateral = Self::collateral_of(sender);
        ensure!(
            reserved_collateral >= amount,
            Error::<T, I>::InsufficientCollateral
        );
        let (slashed, _) = <CurrencyOf<T>>::slash_reserved(sender, amount);

        CurrencyOf::<T>::resolve_creating(receiver, slashed);
        TotalCollateral::<T, I>::mutate(|total| *total = total.saturating_sub(amount));
        Collaterals::<T, I>::mutate(sender, |collateral| {
            *collateral = collateral.saturating_sub(amount)
        });
        Ok(())
    }
}

impl<T: Config<I>, I: 'static> BridgeAssetManager<T::AccountId, BalanceOf<T>> for Pallet<T, I> {
    type TargetAssetId = T::TargetAssetId;
    type TokenAssetId = T::TokenAssetId;

    fn total_issuance() -> BalanceOf<T> {
        xpallet_assets::Pallet::<T>::total_issuance(&Self::TargetAssetId::get())
    }

    fn asset_of(who: &T::AccountId) -> BalanceOf<T> {
        xpallet_assets::Pallet::<T>::usable_balance(who, &Self::TargetAssetId::get())
    }

    fn token_of(who: &T::AccountId) -> BalanceOf<T> {
        xpallet_assets::Pallet::<T>::usable_balance(who, &Self::TokenAssetId::get())
    }

    fn lock_asset(who: &T::AccountId, amount: BalanceOf<T>) -> DispatchResult {
        xpallet_assets::Pallet::<T>::move_balance(
            &Self::TargetAssetId::get(),
            who,
            xpallet_assets::AssetType::Usable,
            who,
            xpallet_assets::AssetType::ReservedWithdrawal,
            amount,
        )
        .map_err(|_| Error::<T, I>::AssetError)?;
        Ok(())
    }

    fn release_asset(who: &T::AccountId, amount: BalanceOf<T>) -> DispatchResult {
        xpallet_assets::Pallet::<T>::move_balance(
            &Self::TargetAssetId::get(),
            who,
            xpallet_assets::AssetType::ReservedWithdrawal,
            who,
            xpallet_assets::AssetType::Usable,
            amount,
        )
        .map_err(|_| Error::<T, I>::AssetError)?;
        Ok(())
    }

    fn mint(who: &T::AccountId, by: &T::AccountId, amount: BalanceOf<T>) -> DispatchResult {
        xpallet_assets::Pallet::<T>::issue(&Self::TargetAssetId::get(), who, amount)?;
        xpallet_assets::Pallet::<T>::issue(&Self::TokenAssetId::get(), by, amount)?;
        Ok(())
    }

    fn burn(who: &T::AccountId, by: &T::AccountId, amount: BalanceOf<T>) -> DispatchResult {
        xpallet_assets::Pallet::<T>::destroy_reserved_withdrawal(
            &Self::TargetAssetId::get(),
            who,
            amount,
        )?;
        xpallet_assets::Pallet::<T>::destroy_usable(&Self::TokenAssetId::get(), by, amount)?;
        Ok(())
    }
}
