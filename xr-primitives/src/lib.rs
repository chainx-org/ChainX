// Copyright 2018-2019 Chainpool.
//! System manager: Handles all of the top-level stuff; executing block/transaction, setting code
//! and depositing logs.

#![cfg_attr(not(feature = "std"), no_std)]

pub mod generic;
pub mod traits;

use rstd::prelude::Vec;

pub type XString = Vec<u8>;
