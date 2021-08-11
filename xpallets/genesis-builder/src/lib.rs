// Copyright 2020 ChainX Project Authors. Licensed under GPL-3.0.

//! This crate provides the feature of initializing the genesis state from ChainX 1.0.

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
use xpallet_assets::BalanceOf as AssetBalanceOf;
#[cfg(feature = "std")]
use xpallet_mining_staking::BalanceOf as StakingBalanceOf;

mod deprecated_builder;

pub use self::deprecated_builder::*;
