#![cfg_attr(not(feature = "std"), no_std)]

pub mod base58;
mod macros;
#[cfg(feature = "std")]
pub mod serde_impl;
pub mod traits;
mod u128;
#[cfg(feature = "std")]
pub mod x_std;

pub use crate::u128::*;
pub use frame_support::fail;
pub use macros::*;
