// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use xpallet_protocol::{ASSET_DESC_MAX_LEN, ASSET_TOKEN_NAME_MAX_LEN, ASSET_TOKEN_SYMBOL_MAX_LEN};

use super::*;

/// Token can only use ASCII alphanumeric character or "-.|~".
pub fn is_valid_token<T: Trait>(token: &[u8]) -> DispatchResult {
    if token.len() > ASSET_TOKEN_SYMBOL_MAX_LEN || token.is_empty() {
        return Err(Error::<T>::InvalidAssetTokenSymbolLength.into());
    }
    let is_valid = |c: &u8| -> bool { c.is_ascii_alphanumeric() || b"-.|~".contains(c) };
    for c in token {
        if !is_valid(c) {
            return Err(Error::<T>::InvalidAssetTokenSymbolChar.into());
        }
    }
    Ok(())
}

/// A valid token name should have a legal length and be visible ASCII chars only.
pub fn is_valid_token_name<T: Trait>(token_name: &[u8]) -> DispatchResult {
    if token_name.len() > ASSET_TOKEN_NAME_MAX_LEN || token_name.is_empty() {
        return Err(Error::<T>::InvalidAssetTokenNameLength.into());
    }
    xp_runtime::xss_check(token_name)?;
    for c in token_name {
        if !is_ascii_visible(c) {
            return Err(Error::<T>::InvalidAscii.into());
        }
    }
    Ok(())
}

/// A valid desc should be visible ASCII chars only and not too long.
pub fn is_valid_desc<T: Trait>(desc: &[u8]) -> DispatchResult {
    if desc.len() > ASSET_DESC_MAX_LEN {
        return Err(Error::<T>::InvalidAssetDescLength.into());
    }
    xp_runtime::xss_check(desc)?;
    for c in desc {
        if !is_ascii_visible(c) {
            return Err(Error::<T>::InvalidAscii.into());
        }
    }
    Ok(())
}

/// Visible ASCII char [0x20, 0x7E]
#[inline]
fn is_ascii_visible(c: &u8) -> bool {
    *c == b' ' || c.is_ascii_graphic()
}
