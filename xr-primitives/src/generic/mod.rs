// Copyright 2018-2019 Chainpool.

pub mod b58;
mod checked_extrinsic;
mod extractor;
mod unchecked_mortal_compact_extrinsic;
mod unchecked_mortal_extrinsic;

pub use self::checked_extrinsic::CheckedExtrinsic;
pub use self::extractor::Extractor;
pub use self::unchecked_mortal_compact_extrinsic::UncheckedMortalCompactExtrinsic;
pub use self::unchecked_mortal_extrinsic::UncheckedMortalExtrinsic;

use parity_codec::Encode;
use rstd::prelude::*;

fn encode_with_vec_prefix<T: Encode, F: Fn(&mut Vec<u8>)>(encoder: F) -> Vec<u8> {
    let size = ::rstd::mem::size_of::<T>();
    let reserve = match size {
        0...0b0011_1111 => 1,
        0...0b0011_1111_1111_1111 => 2,
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
