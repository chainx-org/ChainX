// Copyright 2018-2019 Chainpool.

/// equal or more than 2/3, return an unsigned integer
#[inline]
pub fn two_thirds(sum: u32) -> Option<u32> {
    2_u32
        .checked_mul(sum)
        .map(|m| if m % 3 == 0 { m / 3 } else { m / 3 + 1 })
}

#[inline]
pub fn two_thirds_unsafe(sum: u32) -> u32 {
    two_thirds(sum).expect("the params should not overflow; qed")
}
