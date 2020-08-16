#![cfg_attr(not(feature = "std"), no_std)]

use chainx_primitives::AssetId;
use frame_support::dispatch::DispatchResult;

/// Trait for doing some stuff on the registration/deregistration of a foreign asset.
pub trait RegistrarHandler {
    /// Called when a new asset is added or a deregistered asset is recovered.
    fn on_register(_asset_id: &AssetId, _has_mining_rights: bool) -> DispatchResult {
        Ok(())
    }

    /// Called when an asset is deregistered.
    fn on_deregister(_asset_id: &AssetId) -> DispatchResult {
        Ok(())
    }
}

#[impl_trait_for_tuples::impl_for_tuples(10)]
impl RegistrarHandler for Tuple {
    fn on_register(asset_id: &AssetId, has_mining_rights: bool) -> DispatchResult {
        for_tuples!( #( Tuple::on_register(asset_id, has_mining_rights)?; )* );
        Ok(())
    }

    fn on_deregister(asset_id: &AssetId) -> DispatchResult {
        for_tuples!( #( Tuple::on_deregister(asset_id)?; )* );
        Ok(())
    }
}
