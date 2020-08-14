use super::*;

pub const MAX_TOKEN_LEN: usize = 32;
pub const MAX_DESC_LEN: usize = 128;

/// Visible ASCII char [0x20, 0x7E]
#[inline]
fn is_ascii_invisible(c: &u8) -> bool {
    *c < 0x20 || *c > 0x7E
}

/// A valid token name should have a legal length and be visible ASCII chars only.
pub fn is_valid_token_name<T: Trait>(name: &[u8]) -> DispatchResult {
    if name.len() > MAX_TOKEN_LEN || name.is_empty() {
        Err(Error::<T>::InvalidAssetNameLen)?;
    }
    xp_runtime::xss_check(name)?;
    for c in name.iter() {
        if is_ascii_invisible(c) {
            Err(Error::<T>::InvalidAsscii)?;
        }
    }
    Ok(())
}

/// A valid desc should be visible ASCII chars only and not too long.
pub fn is_valid_desc<T: Trait>(desc: &[u8]) -> DispatchResult {
    if desc.len() > MAX_DESC_LEN {
        Err(Error::<T>::InvalidDescLen)?;
    }
    xp_runtime::xss_check(desc)?;
    for c in desc.iter() {
        if is_ascii_invisible(c) {
            Err(Error::<T>::InvalidAsscii)?;
        }
    }
    Ok(())
}

/// Token can only use ASCII alphanumeric character or "-.|~".
pub fn is_valid_token<T: Trait>(v: &[u8]) -> DispatchResult {
    if v.len() > MAX_TOKEN_LEN || v.is_empty() {
        Err(Error::<T>::InvalidAssetLen)?;
    }
    let is_valid = |c: &u8| -> bool { c.is_ascii_alphanumeric() || "-.|~".as_bytes().contains(c) };
    for c in v.iter() {
        if !is_valid(c) {
            Err(Error::<T>::InvalidChar)?;
        }
    }
    Ok(())
}
