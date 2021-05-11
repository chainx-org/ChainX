use frame_support::{dispatch::DispatchResult, traits::Get};

use chainx_primitives::AssetId;

/// Manage collateral for bridges.
pub trait MultiCollateral<Balance, AccountId> {
    /// Get total collateral.
    ///
    /// Zero by default.
    fn total() -> Balance;
    /// Get collateral of `vault`.
    fn collateral_of(vault: &AccountId) -> Balance;
    /// Lock collateral for vault.
    ///
    /// The backed asset for issuing bridge target asset.
    fn lock(vault: &AccountId, amount: Balance) -> DispatchResult;
    /// Slash `amount` from vault to requester.
    ///
    /// Only vault could be slashed.
    fn slash(vault: &AccountId, requester: &AccountId, amount: Balance) -> DispatchResult;
}

pub trait BridgeAssetManager<AccountId, Balance> {
    type TargetAssetId: Get<AssetId>;
    type TokenAssetId: Get<AssetId>;

    /// Total issuance from bridge.
    fn total_issuance() -> Balance;

    /// Get `who`'s usable asset.
    fn asset_of(who: &AccountId) -> Balance;
    /// Get `who`'s token amount.
    fn token_of(who: &AccountId) -> Balance;

    /// Reserved `who`'s asset for withdrawal.
    fn lock_asset(who: &AccountId, amount: Balance) -> DispatchResult;
    /// Release `who`'s asset.
    fn release_asset(who: &AccountId, amount: Balance) -> DispatchResult;

    /// Mint `amount` assets to `who`.
    ///
    /// It will also increase `by`'s token amount.
    fn mint(who: &AccountId, by: &AccountId, amount: Balance) -> DispatchResult;
    /// Burn `amount` assets from `who`.
    ///
    /// It will also decrease `by`'s token amount.
    fn burn(who: &AccountId, by: &AccountId, amount: Balance) -> DispatchResult;
}
