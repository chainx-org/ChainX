// Copyright 2019 Chainpool.
//! System manager: Handles all of the top-level stuff; executing block/transaction, setting code
//! and depositing logs.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate integer_sqrt;
extern crate num_traits;
extern crate parity_codec;
#[cfg(feature = "std")]
extern crate serde;

extern crate sr_primitives as runtime_primitives;
extern crate sr_std as rstd;
extern crate srml_support as support;

#[cfg(test)]
#[macro_use]
extern crate serde_derive;
#[cfg(test)]
#[macro_use]
extern crate parity_codec_derive;

pub mod generic;
pub mod traits;

use rstd::prelude::Vec;

pub type XString = Vec<u8>;
