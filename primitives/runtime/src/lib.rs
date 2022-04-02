// Copyright 2019-2022 ChainX Project Authors. Licensed under GPL-3.0.

//! ChainX Runtime Modules shared primitive types.

#![warn(missing_docs)]
#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use scale_info::TypeInfo;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::{DispatchError, DispatchResult};
use sp_std::prelude::Vec;

const MAXIMUM_MEMO_LEN: u8 = 128;

/// Returns Ok(_) if the input slice passes the xss check.
///
/// Although xss is imperceptible on-chain, we want to make it
/// look safer off-chain.
#[inline]
pub fn xss_check(input: &[u8]) -> DispatchResult {
    if input.contains(&b'<') || input.contains(&b'>') {
        return Err(DispatchError::Other(
            "'<' and '>' are not allowed, which could be abused off-chain.",
        ));
    }
    Ok(())
}

/// Type for leaving a note when sending a transaction.
#[derive(PartialEq, Eq, Clone, sp_core::RuntimeDebug, Encode, Decode, Default, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct Memo(Vec<u8>);

impl From<Vec<u8>> for Memo {
    fn from(raw: Vec<u8>) -> Self {
        Self(raw)
    }
}

impl From<&[u8]> for Memo {
    fn from(raw: &[u8]) -> Self {
        Self(raw.to_vec())
    }
}

impl AsRef<[u8]> for Memo {
    fn as_ref(&self) -> &[u8] {
        self.0.as_slice()
    }
}

#[cfg(feature = "std")]
impl std::fmt::Display for Memo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", String::from_utf8_lossy(&self.0))
    }
}

#[cfg(not(feature = "std"))]
impl core::fmt::Display for Memo {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

impl Memo {
    /// Returns true if the inner byte length is in the range of [0, 128] and passes the xss check.
    pub fn check_validity(&self) -> DispatchResult {
        if self.0.len() > MAXIMUM_MEMO_LEN as usize {
            Err(DispatchError::Other(
                "transaction memo too long, valid byte length range: [0, 128]",
            ))
        } else {
            xss_check(&self.0)
        }
    }
}
