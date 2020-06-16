#![cfg_attr(not(feature = "std"), no_std)]

pub mod assets_def {
    // TODO use u32 instead of Vec<u8> to stand for a token/asset
    pub const PCX: u32 = 0;
}

pub use assets_def::*;

// assets
pub const ASSET_SYMBOL_LEN: usize = 24;
pub const ASSET_NAME_LEN: usize = 48;
pub const ASSET_DESC_LEN: usize = 128;

pub const MEMO_BYTES_LEN: usize = 80;
