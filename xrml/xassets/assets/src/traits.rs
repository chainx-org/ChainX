use rstd::result;

use support::dispatch::Result;

use crate::types::{AssetErr, AssetType, Chain, Token};

pub trait ChainT {
    const TOKEN: &'static [u8];
    fn chain() -> Chain;
    fn check_addr(_addr: &[u8], _ext: &[u8]) -> Result {
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
    fn on_issue(token: &Token, who: &AccountId, value: Balance) -> Result;
    fn on_destroy(token: &Token, who: &AccountId, value: Balance) -> Result;
    fn on_set_balance(
        _token: &Token,
        _who: &AccountId,
        _type: AssetType,
        _value: Balance,
    ) -> Result {
        Ok(())
    }
}

pub trait OnAssetRegisterOrRevoke {
    fn on_register(_: &Token, _: bool) -> Result;
    fn on_revoke(_: &Token) -> Result;
}
