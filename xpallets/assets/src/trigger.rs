// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use frame_support::dispatch::DispatchResult;

use chainx_primitives::AssetId;

use crate::traits::OnAssetChanged;
use crate::types::{AssetErr, AssetType};
use crate::{BalanceOf, Event, Module, Trait};

impl<AccountId, Balance> OnAssetChanged<AccountId, Balance> for () {}

pub struct AssetChangedTrigger<T: Trait>(sp_std::marker::PhantomData<T>);

impl<T: Trait> AssetChangedTrigger<T> {
    pub fn on_move_pre(
        id: &AssetId,
        from: &T::AccountId,
        from_type: AssetType,
        to: &T::AccountId,
        to_type: AssetType,
        value: BalanceOf<T>,
    ) {
        T::OnAssetChanged::on_move_pre(id, from, from_type, to, to_type, value);
    }

    pub fn on_move_post(
        id: &AssetId,
        from: &T::AccountId,
        from_type: AssetType,
        to: &T::AccountId,
        to_type: AssetType,
        value: BalanceOf<T>,
    ) -> Result<(), AssetErr> {
        Module::<T>::deposit_event(Event::<T>::Moved(
            *id,
            from.clone(),
            from_type,
            to.clone(),
            to_type,
            value,
        ));
        T::OnAssetChanged::on_move_post(id, from, from_type, to, to_type, value)?;
        Ok(())
    }

    pub fn on_issue_pre(id: &AssetId, who: &T::AccountId) {
        T::OnAssetChanged::on_issue_pre(id, who);
    }

    pub fn on_issue_post(id: &AssetId, who: &T::AccountId, value: BalanceOf<T>) -> DispatchResult {
        Module::<T>::deposit_event(Event::<T>::Issued(*id, who.clone(), value));
        T::OnAssetChanged::on_issue_post(id, who, value)?;
        Ok(())
    }

    pub fn on_destroy_pre(id: &AssetId, who: &T::AccountId) {
        T::OnAssetChanged::on_destroy_pre(id, who);
    }

    pub fn on_destroy_post(
        id: &AssetId,
        who: &T::AccountId,
        value: BalanceOf<T>,
    ) -> DispatchResult {
        Module::<T>::deposit_event(Event::<T>::Destroyed(*id, who.clone(), value));
        T::OnAssetChanged::on_destroy_post(id, who, value)?;
        Ok(())
    }

    pub fn on_set_balance(
        id: &AssetId,
        who: &T::AccountId,
        type_: AssetType,
        value: BalanceOf<T>,
    ) -> DispatchResult {
        Module::<T>::deposit_event(Event::<T>::BalanceSet(*id, who.clone(), type_, value));
        T::OnAssetChanged::on_set_balance(id, who, type_, value)?;
        Ok(())
    }
}
