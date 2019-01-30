// Copyright 2019 Chainpool.
//! System manager: Handles all of the top-level stuff; executing block/transaction, setting code
//! and depositing logs.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate integer_sqrt;
extern crate num_traits;
#[cfg(feature = "std")]
extern crate serde;

extern crate parity_codec;
extern crate sr_io;
extern crate sr_primitives;
extern crate sr_std;
extern crate srml_support;
extern crate substrate_primitives;
#[cfg(test)]
#[macro_use]
extern crate serde_derive;
#[cfg(test)]
#[macro_use]
extern crate parity_codec_derive;

pub mod generic;
pub mod traits;

use sr_std::prelude::Vec;

pub type XString = Vec<u8>;
