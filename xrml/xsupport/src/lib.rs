// Copyright 2018 Chainpool.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate parity_codec as codec;
#[macro_use]
extern crate parity_codec_derive;
extern crate sr_io as runtime_io;
extern crate sr_primitives as primitives;
extern crate sr_std as rstd;

extern crate srml_support as runtime_support;

pub use storage::double_map::StorageDoubleMap;

pub mod storage;
