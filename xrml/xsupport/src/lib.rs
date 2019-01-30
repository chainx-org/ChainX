// Copyright 2018 Chainpool.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
extern crate serde;
#[cfg(feature = "std")]
#[macro_use]
extern crate serde_derive;
extern crate parity_codec;
#[macro_use]
extern crate parity_codec_derive;

extern crate sr_io as runtime_io;
extern crate sr_primitives as runtime_primitives;
extern crate sr_std as rstd;
extern crate srml_support as support;

pub mod storage;
pub use self::storage::double_map::StorageDoubleMap;
