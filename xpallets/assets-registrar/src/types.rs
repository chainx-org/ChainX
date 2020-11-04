// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use sp_std::{fmt, result, slice::Iter};

use codec::{Decode, Encode};
use frame_support::{
    dispatch::{DispatchError, DispatchResult},
    RuntimeDebug,
};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

use chainx_primitives::{Decimals, Desc, Token};

use crate::verifier::*;
use crate::Trait;

#[derive(PartialEq, Eq, Ord, PartialOrd, Clone, Copy, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum Chain {
    ChainX,
    Bitcoin,
    Ethereum,
    Polkadot,
}

const CHAINS: [Chain; 4] = [
    Chain::ChainX,
    Chain::Bitcoin,
    Chain::Ethereum,
    Chain::Polkadot,
];

impl Chain {
    /// Returns an iterator of all `Chain`.
    pub fn iter() -> Iter<'static, Chain> {
        CHAINS.iter()
    }
}

impl Default for Chain {
    fn default() -> Self {
        Chain::ChainX
    }
}

#[derive(PartialEq, Eq, Clone, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct AssetInfo {
    #[cfg_attr(feature = "std", serde(with = "xp_rpc::serde_text"))]
    token: Token,
    #[cfg_attr(feature = "std", serde(with = "xp_rpc::serde_text"))]
    token_name: Token,
    chain: Chain,
    decimals: Decimals,
    #[cfg_attr(feature = "std", serde(with = "xp_rpc::serde_text"))]
    desc: Desc,
}

impl fmt::Debug for AssetInfo {
    #[cfg(feature = "std")]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("AssetInfo")
            .field("token", &String::from_utf8_lossy(&self.token))
            .field("token_name", &String::from_utf8_lossy(&self.token_name))
            .field("chain", &self.chain)
            .field("decimals", &self.decimals)
            .field("desc", &String::from_utf8_lossy(&self.desc))
            .finish()
    }
    #[cfg(not(feature = "std"))]
    fn fmt(&self, _f: &mut fmt::Formatter) -> fmt::Result {
        Ok(())
    }
}

impl AssetInfo {
    pub fn new<T: Trait>(
        token: Token,
        token_name: Token,
        chain: Chain,
        decimals: Decimals,
        desc: Desc,
    ) -> result::Result<Self, DispatchError> {
        let asset = AssetInfo {
            token,
            token_name,
            chain,
            decimals,
            desc,
        };
        asset.is_valid::<T>()?;
        Ok(asset)
    }

    pub fn is_valid<T: Trait>(&self) -> DispatchResult {
        is_valid_token::<T>(&self.token)?;
        is_valid_token_name::<T>(&self.token_name)?;
        is_valid_desc::<T>(&self.desc)
    }

    pub fn token(&self) -> &Token {
        &self.token
    }

    pub fn token_name(&self) -> &Token {
        &self.token_name
    }

    pub fn chain(&self) -> Chain {
        self.chain
    }

    pub fn desc(&self) -> &Desc {
        &self.desc
    }

    pub fn decimals(&self) -> Decimals {
        self.decimals
    }

    pub fn set_desc(&mut self, desc: Desc) {
        self.desc = desc
    }

    pub fn set_token(&mut self, token: Token) {
        self.token = token
    }

    pub fn set_token_name(&mut self, token_name: Token) {
        self.token_name = token_name
    }
}
