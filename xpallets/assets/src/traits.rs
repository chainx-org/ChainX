use sp_std::result;

use frame_support::dispatch::DispatchResult;

use chainx_primitives::AssetId;

use crate::types::{AssetErr, AssetType, Chain};

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

/// Hooks for doing stuff when the assets are minted/moved/destroyed.
pub trait OnAssetChanged<AccountId, Balance> {
    /// Triggered before issuing the fresh assets.
    fn on_issue_pre(_id: &AssetId, _who: &AccountId) {}

    /// Triggered after issuing the fresh assets.
    fn on_issue_post(_id: &AssetId, _who: &AccountId, _value: Balance) -> DispatchResult {
        Ok(())
    }

    /// Triggered before moving the assets.
    fn on_move_pre(
        _id: &AssetId,
        _from: &AccountId,
        _from_type: AssetType,
        _to: &AccountId,
        _to_type: AssetType,
        _value: Balance,
    ) {
    }

    /// Triggered after moving the assets.
    fn on_move_post(
        _id: &AssetId,
        _from: &AccountId,
        _from_type: AssetType,
        _to: &AccountId,
        _to_type: AssetType,
        _value: Balance,
    ) -> result::Result<(), AssetErr> {
        Ok(())
    }

    /// Triggered before destroying the assets.
    fn on_destroy_pre(_id: &AssetId, _who: &AccountId) {}

    /// Triggered after the assets has been destroyed.
    fn on_destroy_post(_id: &AssetId, _who: &AccountId, _value: Balance) -> DispatchResult {
        Ok(())
    }

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
    fn on_register(_: &AssetId, _: bool) -> DispatchResult {
        Ok(())
    }
    fn on_revoke(_: &AssetId) -> DispatchResult {
        Ok(())
    }
}
