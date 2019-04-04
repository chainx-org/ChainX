use rstd::result;
use support::dispatch::Result;

use crate::traits::{OnAssetChanged, OnAssetRegisterOrRevoke};
use crate::types::{AssetErr, AssetType, Token};
use crate::{Module, RawEvent, Trait};

impl<AccountId, Balance> OnAssetChanged<AccountId, Balance> for () {
    fn on_move(
        _token: &Token,
        _from: &AccountId,
        _from_type: AssetType,
        _to: &AccountId,
        _to_type: AssetType,
        _value: Balance,
    ) -> result::Result<(), AssetErr> {
        Ok(())
    }
    fn on_issue(_: &Token, _: &AccountId, _: Balance) -> Result {
        Ok(())
    }
    fn on_destroy(_: &Token, _: &AccountId, _: Balance) -> Result {
        Ok(())
    }
}

impl OnAssetRegisterOrRevoke for () {
    fn on_register(_: &Token, _: bool) -> Result {
        Ok(())
    }
    fn on_revoke(_: &Token) -> Result {
        Ok(())
    }
}

impl<A: OnAssetRegisterOrRevoke, B: OnAssetRegisterOrRevoke> OnAssetRegisterOrRevoke for (A, B) {
    fn on_register(token: &Token, is_psedu_intention: bool) -> Result {
        let r = A::on_register(token, is_psedu_intention);
        let r2 = B::on_register(token, is_psedu_intention);
        if r.is_ok() == false {
            return r;
        } else if r2.is_ok() == false {
            return r2;
        }
        Ok(())
    }

    fn on_revoke(token: &Token) -> Result {
        let r = A::on_revoke(token);
        let r2 = B::on_revoke(token);
        if r.is_ok() == false {
            return r;
        } else if r2.is_ok() == false {
            return r2;
        }
        Ok(())
    }
}

pub struct AssetTriggerEventAfter<T: Trait>(::rstd::marker::PhantomData<T>);

impl<T: Trait> AssetTriggerEventAfter<T> {
    pub fn on_move(
        token: &Token,
        from: &T::AccountId,
        from_type: AssetType,
        to: &T::AccountId,
        to_type: AssetType,
        value: T::Balance,
    ) -> result::Result<(), AssetErr> {
        Module::<T>::deposit_event(RawEvent::Move(
            token.clone(),
            from.clone(),
            from_type,
            to.clone(),
            to_type,
            value,
        ));
        T::OnAssetChanged::on_move(token, from, from_type, to, to_type, value)?;
        Ok(())
    }
    pub fn on_issue(token: &Token, who: &T::AccountId, value: T::Balance) -> Result {
        Module::<T>::deposit_event(RawEvent::Issue(token.clone(), who.clone(), value));
        T::OnAssetChanged::on_issue(token, who, value)?;
        Ok(())
    }
    pub fn on_destroy(token: &Token, who: &T::AccountId, value: T::Balance) -> Result {
        Module::<T>::deposit_event(RawEvent::Destory(token.clone(), who.clone(), value));
        T::OnAssetChanged::on_destroy(token, who, value)?;
        Ok(())
    }
    pub fn on_set_balance(
        token: &Token,
        who: &T::AccountId,
        type_: AssetType,
        value: T::Balance,
    ) -> Result {
        Module::<T>::deposit_event(RawEvent::Set(token.clone(), who.clone(), type_, value));
        T::OnAssetChanged::on_set_balance(token, who, type_, value)?;
        Ok(())
    }
}
