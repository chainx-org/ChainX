// Copyright 2019 Chainpool.

mod checked_extrinsic;
mod unchecked_mortal_extrinsic;

pub use self::checked_extrinsic::CheckedExtrinsic;
pub use self::unchecked_mortal_extrinsic::UncheckedMortalExtrinsic;

use codec::Encode;
use rstd::prelude::*;

fn encode_with_vec_prefix<T: Encode, F: Fn(&mut Vec<u8>)>(encoder: F) -> Vec<u8> {
    let size = ::rstd::mem::size_of::<T>();
    let reserve = match size {
        0...0b00111111 => 1,
        0...0b00111111_11111111 => 2,
        _ => 4,
    };
    let mut v = Vec::with_capacity(reserve + size);
    v.resize(reserve, 0);
    encoder(&mut v);

    // need to prefix with the total length to ensure it's binary comptible with
    // Vec<u8>.
    let mut length: Vec<()> = Vec::new();
    length.resize(v.len() - reserve, ());
    length.using_encoded(|s| {
        v.splice(0..reserve, s.iter().cloned());
    });

    v
}
