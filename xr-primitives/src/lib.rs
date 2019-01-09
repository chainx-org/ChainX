// Copyright 2019 Chainpool.
//! System manager: Handles all of the top-level stuff; executing block/transaction, setting code
//! and depositing logs.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
extern crate serde;

#[cfg(all(test, feature = "std"))]
#[macro_use]
extern crate serde_derive;

#[cfg(test)]
#[macro_use]
extern crate parity_codec_derive;

extern crate integer_sqrt;
extern crate num_traits;
#[doc(hidden)]
pub extern crate parity_codec as codec;
extern crate sr_io as runtime_io;
extern crate sr_primitives;
extern crate sr_std as rstd;
extern crate substrate_primitives;

#[cfg(test)]
extern crate serde_json;

pub mod generic;
pub mod traits;

use rstd::prelude::Vec;

pub type XString = Vec<u8>;