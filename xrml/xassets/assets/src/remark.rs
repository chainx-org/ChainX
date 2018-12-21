// Copyright 2018 Chainpool.
use runtime_support::dispatch::Result;

use super::{Trait, Module};


pub fn is_valid_remark<T: Trait>(msg: &[u8]) -> Result {
    // filter char
    // judge len
    if msg.len() as u32 > Module::<T>::remark_len() {
        return Err("remark is too long")
    }
    Ok(())
}