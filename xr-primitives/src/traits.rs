// Copyright 2018-2019 Chainpool.

pub use integer_sqrt::IntegerSquareRoot;
pub use num_traits::{
    ops::checked::{CheckedAdd, CheckedDiv, CheckedMul, CheckedShl, CheckedShr, CheckedSub},
    Bounded, One, Zero,
};

use rstd::prelude::Vec;
use rstd::result;
use runtime_primitives::traits::{As, MaybeDisplay, Member, SimpleArithmetic};
use support::dispatch::Result;

/// Work together with sr_primitives::traits::Applyable
pub trait Accelerable: Sized + Send + Sync {
    type AccountId: Member + MaybeDisplay;
    type Index: Member + MaybeDisplay + SimpleArithmetic;
    type Call: Member;
    type Acceleration: Member + MaybeDisplay + SimpleArithmetic + Copy + As<u64>;

    fn acceleration(&self) -> Option<Self::Acceleration>;
}

pub trait Extractable<AccountId> {
    fn account_info(data: &[u8]) -> Option<(AccountId, Vec<u8>)>;
}

pub trait TrusteeForChain<AccountId, Address> {
    fn check_address(raw_addr: &[u8]) -> Result;
    fn to_address(raw_addr: &[u8]) -> Address;
    fn generate_new_trustees(
        candidates: &Vec<AccountId>,
    ) -> result::Result<Vec<AccountId>, &'static str>;
}
