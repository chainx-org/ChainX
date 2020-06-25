use frame_support::dispatch::DispatchResult;
use sp_std::result;

use chainx_primitives::AssetId;

use crate::traits::{OnAssetChanged, OnAssetRegisterOrRevoke};
use crate::types::{AssetErr, AssetType};
use crate::{Module, RawEvent, Trait};

impl<AccountId, Balance> OnAssetChanged<AccountId, Balance> for () {
    fn on_move_before(
        _id: &AssetId,
        _from: &AccountId,
        _from_type: AssetType,
        _to: &AccountId,
        _to_type: AssetType,
        _value: Balance,
    ) {
    }
    fn on_move(
        _id: &AssetId,
        _from: &AccountId,
        _from_type: AssetType,
        _to: &AccountId,
        _to_type: AssetType,
        _value: Balance,
    ) -> result::Result<(), AssetErr> {
        Ok(())
    }
    fn on_issue_before(_: &AssetId, _: &AccountId) {}
    fn on_issue(_: &AssetId, _: &AccountId, _: Balance) -> DispatchResult {
        Ok(())
    }
    fn on_destroy_before(_: &AssetId, _: &AccountId) {}
    fn on_destroy(_: &AssetId, _: &AccountId, _: Balance) -> DispatchResult {
        Ok(())
    }
}

impl OnAssetRegisterOrRevoke for () {
    fn on_register(_: &AssetId, _: bool) -> DispatchResult {
        Ok(())
    }
    fn on_revoke(_: &AssetId) -> DispatchResult {
        Ok(())
    }
}

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

pub struct AssetTriggerEventAfter<T: Trait>(::sp_std::marker::PhantomData<T>);

impl<T: Trait> AssetTriggerEventAfter<T> {
    pub fn on_move_before(
        id: &AssetId,
        from: &T::AccountId,
        from_type: AssetType,
        to: &T::AccountId,
        to_type: AssetType,
        value: T::Balance,
    ) {
        T::OnAssetChanged::on_move_before(id, from, from_type, to, to_type, value);
    }
    pub fn on_move(
        id: &AssetId,
        from: &T::AccountId,
        from_type: AssetType,
        to: &T::AccountId,
        to_type: AssetType,
        value: T::Balance,
    ) -> result::Result<(), AssetErr> {
        Module::<T>::deposit_event(RawEvent::Move(
            id.clone(),
            from.clone(),
            from_type,
            to.clone(),
            to_type,
            value,
        ));
        T::OnAssetChanged::on_move(id, from, from_type, to, to_type, value)?;
        Ok(())
    }
    pub fn on_issue_before(id: &AssetId, who: &T::AccountId) {
        T::OnAssetChanged::on_issue_before(id, who);
    }
    pub fn on_issue(id: &AssetId, who: &T::AccountId, value: T::Balance) -> DispatchResult {
        Module::<T>::deposit_event(RawEvent::Issue(id.clone(), who.clone(), value));
        T::OnAssetChanged::on_issue(id, who, value)?;
        Ok(())
    }
    pub fn on_destroy_before(id: &AssetId, who: &T::AccountId) {
        T::OnAssetChanged::on_destroy_before(id, who);
    }
    pub fn on_destroy(id: &AssetId, who: &T::AccountId, value: T::Balance) -> DispatchResult {
        Module::<T>::deposit_event(RawEvent::Destory(id.clone(), who.clone(), value));
        T::OnAssetChanged::on_destroy(id, who, value)?;
        Ok(())
    }
    pub fn on_set_balance(
        id: &AssetId,
        who: &T::AccountId,
        type_: AssetType,
        value: T::Balance,
    ) -> DispatchResult {
        Module::<T>::deposit_event(RawEvent::Set(id.clone(), who.clone(), type_, value));
        T::OnAssetChanged::on_set_balance(id, who, type_, value)?;
        Ok(())
    }
}
