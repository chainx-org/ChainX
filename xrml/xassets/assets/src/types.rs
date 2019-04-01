// Copyright 2018-2019 Chainpool.

use parity_codec::{Decode, Encode};
#[cfg(feature = "std")]
use serde_derive::{Deserialize, Serialize};

// Substrate
use rstd::{prelude::*, result::Result as StdResult, slice::Iter};
use support::dispatch::Result;

// ChainX
use xr_primitives::XString;

use super::{Module, Trait};

const MAX_TOKEN_LEN: usize = 32;
const MAX_DESC_LEN: usize = 128;

pub type TokenString = &'static [u8];
pub type DescString = TokenString;
pub type Token = XString;
pub type Desc = XString;
pub type Precision = u16;
pub type Memo = XString;

pub trait ChainT {
    const TOKEN: &'static [u8];
    fn chain() -> Chain;
    fn check_addr(_addr: &[u8], _ext: &[u8]) -> Result {
        Ok(())
    }
}

#[derive(PartialEq, Eq, Ord, PartialOrd, Clone, Copy, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub enum Chain {
    ChainX,
    Bitcoin,
    Ethereum,
}

impl Default for Chain {
    fn default() -> Self {
        Chain::ChainX
    }
}

impl Chain {
    pub fn iterator() -> Iter<'static, Chain> {
        static CHAINS: [Chain; 3] = [Chain::ChainX, Chain::Bitcoin, Chain::Ethereum];
        CHAINS.iter()
    }
}

#[derive(PartialEq, Eq, Clone, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct Asset {
    token: Token,
    token_name: Token,
    chain: Chain,
    precision: Precision,
    desc: Desc,
}

impl Asset {
    pub fn new(
        token: Token,
        token_name: Token,
        chain: Chain,
        precision: Precision,
        desc: Desc,
    ) -> StdResult<Self, &'static str> {
        let a = Asset {
            token,
            token_name,
            chain,
            precision,
            desc,
        };
        a.is_valid()?;
        Ok(a)
    }
    pub fn is_valid(&self) -> Result {
        is_valid_token(&self.token)?;
        is_valid_token_name(&self.token_name)?;
        is_valid_desc(&self.desc)
    }

    pub fn token(&self) -> Token {
        self.token.clone()
    }
    pub fn token_name(&self) -> Token {
        self.token_name.clone()
    }
    pub fn chain(&self) -> Chain {
        self.chain
    }
    pub fn desc(&self) -> Desc {
        self.desc.clone()
    }
    pub fn set_desc(&mut self, desc: Desc) {
        self.desc = desc
    }
    pub fn precision(&self) -> Precision {
        self.precision
    }
}

#[derive(PartialEq, PartialOrd, Ord, Eq, Clone, Copy, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub enum AssetType {
    Free,
    ReservedStaking,
    ReservedStakingRevocation,
    ReservedWithdrawal,
    ReservedDexSpot,
    ReservedDexFuture,
}

// TODO use marco to improve it
impl AssetType {
    pub fn iterator() -> Iter<'static, AssetType> {
        static TYPES: [AssetType; 6] = [
            AssetType::Free,
            AssetType::ReservedStaking,
            AssetType::ReservedStakingRevocation,
            AssetType::ReservedWithdrawal,
            AssetType::ReservedDexSpot,
            AssetType::ReservedDexFuture,
        ];
        TYPES.iter()
    }
}

impl Default for AssetType {
    fn default() -> Self {
        AssetType::Free
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub enum AssetErr {
    NotEnough,
    OverFlow,
    TotalAssetNotEnough,
    TotalAssetOverFlow,
    InvalidToken,
    InvalidAccount,
}

impl AssetErr {
    pub fn info(self) -> &'static str {
        match self {
            AssetErr::NotEnough => "balance too low for this account",
            AssetErr::OverFlow => "balance too high for this account",
            AssetErr::TotalAssetNotEnough => "total balance too low for this asset",
            AssetErr::TotalAssetOverFlow => "total balance too high for this asset",
            AssetErr::InvalidToken => "not a valid token for this account",
            AssetErr::InvalidAccount => "account Locked",
        }
    }
}

/// Token can only use numbers (0x30~0x39), capital letters (0x41~0x5A), lowercase letters (0x61~0x7A), -(0x2D), .(0x2E), |(0x7C),  ~(0x7E).
pub fn is_valid_token(v: &[u8]) -> Result {
    if v.len() > MAX_TOKEN_LEN || v.is_empty() {
        return Err("Token length is zero or too long.");
    }
    let is_valid = |c: &u8| -> bool {
        (*c >= 0x30 && *c <= 0x39) // number
            || (*c >= 0x41 && *c <= 0x5A) // capital
            || (*c >= 0x61 && *c <= 0x7A) // small
            || (*c == 0x2D) // -
            || (*c == 0x2E) // .
            || (*c == 0x7C) // |
            || (*c == 0x7E) // ~
    };
    for c in v.iter() {
        if !is_valid(c) {
            return Err(
                "Token can only use numbers, capital/lowercase letters or '-', '.', '|', '~'.",
            );
        }
    }
    Ok(())
}

pub fn is_valid_token_name(v: &[u8]) -> Result {
    if v.len() > MAX_TOKEN_LEN || v.is_empty() {
        return Err("Token name is zero or too long.");
    }
    for c in v.iter() {
        // Visible ASCII char [0x20, 0x7E]
        if *c < 0x20 || *c > 0x7E {
            return Err("Token name can not use an invisiable ASCII char.");
        }
    }
    Ok(())
}

/// Desc can only be Visible ASCII chars.
pub fn is_valid_desc(v: &[u8]) -> Result {
    if v.len() > MAX_DESC_LEN {
        return Err("Token desc too long");
    }
    for c in v.iter() {
        // Visible ASCII char [0x20, 0x7E]
        if *c < 0x20 || *c > 0x7E {
            return Err("Desc can not use an invisiable ASCII char.");
        }
    }
    Ok(())
}

pub fn is_valid_memo<T: Trait>(msg: &Memo) -> Result {
    // filter char
    // judge len
    if msg.len() as u32 > Module::<T>::memo_len() {
        return Err("memo is too long");
    }
    Ok(())
}
