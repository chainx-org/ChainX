// Copyright 2018-2019 Chainpool.

pub use integer_sqrt::IntegerSquareRoot;
pub use num_traits::{
    ops::checked::{CheckedAdd, CheckedDiv, CheckedMul, CheckedShl, CheckedShr, CheckedSub},
    Bounded, One, Zero,
};

use rstd::prelude::Vec;
use rstd::result::Result as StdResult;
use runtime_primitives::traits::{As, MaybeDisplay, MaybeSerializeDebug, Member, SimpleArithmetic};
use support::{dispatch::Result, Parameter};

/// Work together with sr_primitives::traits::Applyable
pub trait Accelerable: Sized + Send + Sync {
    type AccountId: Member + MaybeDisplay;
    type Index: Member + MaybeDisplay + SimpleArithmetic;
    type Call: Member;
    type Acceleration: Member + MaybeDisplay + SimpleArithmetic + Copy + As<u64>;

    fn acceleration(&self) -> Option<Self::Acceleration>;
}

pub trait Extractable {
    type AccountId: Parameter + Member + MaybeSerializeDebug + MaybeDisplay + Ord + Default;

    fn new(script: Vec<u8>) -> Self;
    fn account_info(&self) -> Option<(Self::AccountId, Vec<u8>)>;
    fn split(&self) -> Vec<Vec<u8>>;
}

pub trait TrusteeForChain<AccountId, Address> {
    fn check_address(raw_addr: &[u8]) -> Result;
    fn to_address(raw_addr: &[u8]) -> Address;
    fn generate_new_trustees(
        candidates: &Vec<AccountId>,
    ) -> StdResult<Vec<AccountId>, &'static str>;
}
