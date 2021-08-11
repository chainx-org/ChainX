// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

//! The genesis builder primitives.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
mod genesis_params;

#[cfg(feature = "std")]
pub use self::genesis_params::*;
