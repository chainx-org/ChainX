// Copyright 2018-2019 Chainpool.

//! The Chainx runtime reexported for WebAssembly compile.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate chainx_runtime;
pub use chainx_runtime::*;
