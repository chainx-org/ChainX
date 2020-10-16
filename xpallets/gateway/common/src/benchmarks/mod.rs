// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
// hack operation!
// in rust, test part in crate internal and in the outside of crate is different when this situation:
// child crate use parent crate types, and parent crate test case call child crate method which would
// use those types. In this situation, test case in parent crate would meet though it's truely same
// type, but rust would think there are different type.
// But move tests to outside of crate would ok for this situation.
// However, substrate benchmarks must inside the crate, thus we move source test case framework outside
// of the crate, and in current crate, we make a simple mocked test case framework(`ExtBuilder`),
// just use this `ExtBuilder` for benchmarks, not for test case.
#[cfg(test)]
mod mock;
#[cfg(test)]
pub mod mock_impls;

#[cfg(test)]
use mock::*;

// for compile
#[cfg(test)]
mod tests {
    use crate::benchmarks::mock::ExtBuilder;

    #[test]
    fn base() {
        ExtBuilder::default().build().execute_with(|| {})
    }
}
