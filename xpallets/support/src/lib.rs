#![cfg_attr(not(feature = "std"), no_std)]

pub mod base58;
mod macros;
#[cfg(feature = "std")]
pub mod x_std;

use frame_support::dispatch::{DispatchError, DispatchResult};

pub use frame_support::fail;
pub use macros::*;

/// Although xss is imperceptible on-chain, we merely want to make it look safer off-chain.
#[inline]
pub fn xss_check(input: &[u8]) -> DispatchResult {
    if input.contains(&b'<') || input.contains(&b'>') {
        Err(DispatchError::Other(
            "'<' and '>' are not allowed, which could be abused off-chain.",
        ))?;
    }
    Ok(())
}
