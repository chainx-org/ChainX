// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

#![cfg_attr(not(feature = "std"), no_std)]

#[macro_use]
mod macros;
#[cfg(feature = "std")]
mod serde;
pub mod traits;
#[cfg(feature = "std")]
pub mod x_std;

pub use frame_support::fail;

pub use self::macros::*;
#[cfg(feature = "std")]
pub use self::serde::{serde_hex, serde_text};
