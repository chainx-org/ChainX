// Copyright 2018-2019 Chainpool.
//! this module is for bridge common parts
//! define trait and type for
//! `trustees`, `crosschain binding` and something others

#![cfg_attr(not(feature = "std"), no_std)]

pub mod extractor;
pub mod traits;
pub mod types;
pub mod utils;

mod trustees;
