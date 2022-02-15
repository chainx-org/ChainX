// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use frame_support::dispatch::DispatchResult;

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
