use sp_std::result;

use frame_support::dispatch::DispatchResult;

use crate::types::{AssetErr, AssetType, Chain, Token};

pub trait TokenJackpotAccountIdFor<AccountId: Sized, BlockNumber> {
    fn accountid_for_unsafe(token: &Token) -> AccountId;
    fn accountid_for_safe(token: &Token) -> Option<AccountId>;
}

pub trait ChainT {
    const TOKEN: &'static [u8];
    fn chain() -> Chain;
    fn check_addr(_addr: &[u8], _ext: &[u8]) -> DispatchResult {
        Ok(())
    }
}

pub trait OnAssetChanged<AccountId, Balance> {
    fn on_move_before(
        token: &Token,
        from: &AccountId,
        from_type: AssetType,
        to: &AccountId,
        to_type: AssetType,
        value: Balance,
    );
    fn on_move(
        token: &Token,
        from: &AccountId,
        from_type: AssetType,
        to: &AccountId,
        to_type: AssetType,
        value: Balance,
    ) -> result::Result<(), AssetErr>;
    fn on_issue_before(token: &Token, who: &AccountId);
    fn on_issue(token: &Token, who: &AccountId, value: Balance) -> DispatchResult;
    fn on_destroy_before(token: &Token, who: &AccountId);
    fn on_destroy(token: &Token, who: &AccountId, value: Balance) -> DispatchResult;
    fn on_set_balance(
        _token: &Token,
        _who: &AccountId,
        _type: AssetType,
        _value: Balance,
    ) -> DispatchResult {
        Ok(())
    }
}

pub trait OnAssetRegisterOrRevoke {
    fn on_register(_: &Token, _: bool) -> DispatchResult;
    fn on_revoke(_: &Token) -> DispatchResult;
}
