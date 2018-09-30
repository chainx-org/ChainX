// Copyright 2018 Chainpool.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate srml_support as runtime_support;
extern crate sr_std as rstd;

extern crate parity_codec_derive;
extern crate parity_codec as codec;

extern crate sr_io as runtime_io;

pub mod storage;

pub use storage::double_map::StorageDoubleMap;