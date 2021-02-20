// Copyright 2021 ChainX Project Authors. Licensed under GPL-3.0.

//! This module implements Bitcoin Bridge V2.

#![cfg_attr(not(feature = "std"), no_std)]

pub mod assets;
pub mod issue;
pub mod redeem;
pub mod vault;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
