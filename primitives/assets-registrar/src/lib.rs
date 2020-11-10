// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

//! The asset registrar primitives.

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(missing_docs)]

use codec::{Decode, Encode};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

use sp_runtime::{DispatchResult, RuntimeDebug};
use sp_std::slice::Iter;

use chainx_primitives::AssetId;

const CHAINS: [Chain; 4] = [
    Chain::ChainX,
    Chain::Bitcoin,
    Chain::Ethereum,
    Chain::Polkadot,
];

/// The blockchain types.
#[derive(PartialEq, Eq, Ord, PartialOrd, Clone, Copy, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum Chain {
    /// ChainX
    ChainX,
    /// Bitcoin
    Bitcoin,
    /// Ethereum
    Ethereum,
    /// Polkadot
    Polkadot,
}

impl Default for Chain {
    fn default() -> Self {
        Chain::ChainX
    }
}

impl Chain {
    /// Returns an iterator of all `Chain`.
    pub fn iter() -> Iter<'static, Chain> {
        CHAINS.iter()
    }
}

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

#[impl_trait_for_tuples::impl_for_tuples(30)]
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
