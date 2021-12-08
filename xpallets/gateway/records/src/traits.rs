// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use frame_support::dispatch::{DispatchError, DispatchResult};
use xp_assets_registrar::Chain;

use crate::types::WithdrawalLimit;

pub trait ChainT<AssetId, Balance: Default> {
    /// ASSET should be the native Asset for this chain.
    /// e.g.
    ///     if ChainT for Bitcoin, then ASSET is X_BTC
    ///     if ChainT for Ethereum, then ASSET is X_ETH
    ///     if ChainT for Polkadot, then ASSET is X_DOT
    fn chain() -> Chain;
    fn check_addr(_addr: &[u8], _ext: &[u8]) -> DispatchResult {
        Ok(())
    }
    fn withdrawal_limit(_asset_id: &AssetId) -> Result<WithdrawalLimit<Balance>, DispatchError> {
        Ok(WithdrawalLimit::default())
    }
}

/// Hooks for doing stuff when the assets are minted/moved/destroyed.
pub trait OnAssetChanged<AssetId, AccountId, Balance> {
    /// Triggered before issuing the fresh assets.
    fn on_issue_pre(_id: &AssetId, _who: &AccountId) {}

    /// Triggered after issuing the fresh assets.
    fn on_issue_post(_id: &AssetId, _who: &AccountId, _value: Balance) -> DispatchResult {
        Ok(())
    }

    /// Triggered before moving the assets.
    fn on_move_pre(_id: &AssetId, _from: &AccountId, _to: &AccountId, _value: Balance) {}

    /// Triggered after moving the assets.
    fn on_move_post(
        _id: &AssetId,
        _from: &AccountId,
        _to: &AccountId,
        _value: Balance,
    ) -> DispatchResult {
        Ok(())
    }

    /// Triggered before destroying the assets.
    fn on_destroy_pre(_id: &AssetId, _who: &AccountId) {}

    /// Triggered after the assets has been destroyed.
    fn on_destroy_post(_id: &AssetId, _who: &AccountId, _value: Balance) -> DispatchResult {
        Ok(())
    }

    /// Triggered after the balance has been set to a new value.
    fn on_set_balance(_id: &AssetId, _who: &AccountId, _value: Balance) -> DispatchResult {
        Ok(())
    }
}
