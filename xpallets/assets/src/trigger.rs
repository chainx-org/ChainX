use frame_support::dispatch::DispatchResult;
use sp_std::result;

use chainx_primitives::AssetId;

use crate::traits::{OnAssetChanged, OnAssetRegisterOrRevoke};
use crate::types::{AssetErr, AssetType};
use crate::{BalanceOf, Module, RawEvent, Trait};

impl<AccountId, Balance> OnAssetChanged<AccountId, Balance> for () {}

impl OnAssetRegisterOrRevoke for () {}

impl<A: OnAssetRegisterOrRevoke, B: OnAssetRegisterOrRevoke> OnAssetRegisterOrRevoke for (A, B) {
    fn on_register(id: &AssetId, is_psedu_intention: bool) -> DispatchResult {
        let r = A::on_register(id, is_psedu_intention);
        let r2 = B::on_register(id, is_psedu_intention);
        if r.is_ok() == false {
            return r;
        } else if r2.is_ok() == false {
            return r2;
        }
        Ok(())
    }

    fn on_revoke(id: &AssetId) -> DispatchResult {
        let r = A::on_revoke(id);
        let r2 = B::on_revoke(id);
        if r.is_ok() == false {
            return r;
        } else if r2.is_ok() == false {
            return r2;
        }
        Ok(())
    }
}

pub struct AssetChangedTrigger<T: Trait>(::sp_std::marker::PhantomData<T>);

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
    ) -> result::Result<(), AssetErr> {
        Module::<T>::deposit_event(RawEvent::Move(
            id.clone(),
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
        Module::<T>::deposit_event(RawEvent::Issue(id.clone(), who.clone(), value));
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
        Module::<T>::deposit_event(RawEvent::Destory(id.clone(), who.clone(), value));
        T::OnAssetChanged::on_destroy_post(id, who, value)?;
        Ok(())
    }

    pub fn on_set_balance(
        id: &AssetId,
        who: &T::AccountId,
        type_: AssetType,
        value: BalanceOf<T>,
    ) -> DispatchResult {
        Module::<T>::deposit_event(RawEvent::Set(id.clone(), who.clone(), type_, value));
        T::OnAssetChanged::on_set_balance(id, who, type_, value)?;
        Ok(())
    }
}
