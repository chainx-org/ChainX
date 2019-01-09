// Copyright 2018 Chainpool.
use runtime_support::dispatch::Result;

use super::{Module, Trait};

use xr_primitives::XString;

pub type Memo = XString;

pub fn is_valid_memo<T: Trait>(msg: &Memo) -> Result {
    // filter char
    // judge len
    if msg.len() as u32 > Module::<T>::memo_len() {
        return Err("memo is too long");
    }
    Ok(())
}
