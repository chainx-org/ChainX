use sp_std::result;

use frame_support::dispatch::DispatchResult;

use chainx_primitives::AssetId;

use crate::types::{AssetErr, AssetType, Chain};

pub trait TokenJackpotAccountIdFor<AccountId: Sized, BlockNumber> {
    fn accountid_for(id: &AssetId) -> AccountId;
}

pub trait ChainT {
    /// ASSET should be the native Asset for this chain.
    /// e.g.
    ///     if ChainT for Bitcoin, then ASSET is X_BTC
    ///     if ChainT for Ethereum, then ASSET is X_ETH
    ///     if ChainT for Polkadot, then ASSET is X_ETH
    const ASSET_ID: AssetId;
    fn chain() -> Chain;
    fn check_addr(_addr: &[u8], _ext: &[u8]) -> DispatchResult {
        Ok(())
    }
}

pub trait OnAssetChanged<AccountId, Balance> {
    fn on_move_before(
        id: &AssetId,
        from: &AccountId,
        from_type: AssetType,
        to: &AccountId,
        to_type: AssetType,
        value: Balance,
    );
    fn on_move(
        id: &AssetId,
        from: &AccountId,
        from_type: AssetType,
        to: &AccountId,
        to_type: AssetType,
        value: Balance,
    ) -> result::Result<(), AssetErr>;
    fn on_issue_before(id: &AssetId, who: &AccountId);
    fn on_issue(id: &AssetId, who: &AccountId, value: Balance) -> DispatchResult;
    fn on_destroy_before(id: &AssetId, who: &AccountId);
    fn on_destroy(id: &AssetId, who: &AccountId, value: Balance) -> DispatchResult;
    fn on_set_balance(
        _id: &AssetId,
        _who: &AccountId,
        _type: AssetType,
        _value: Balance,
    ) -> DispatchResult {
        Ok(())
    }
}

pub trait OnAssetRegisterOrRevoke {
    fn on_register(_: &AssetId, _: bool) -> DispatchResult;
    fn on_revoke(_: &AssetId) -> DispatchResult;
}
