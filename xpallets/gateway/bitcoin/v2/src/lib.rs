// Copyright 2021 ChainX Project Authors. Licensed under GPL-3.0.

//! This module implements Bitcoin Bridge V2.

#![cfg_attr(not(feature = "std"), no_std)]

mod assets;
mod issue;
mod vault;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
