pub use integer_sqrt::IntegerSquareRoot;
pub use num_traits::{
    ops::checked::{CheckedAdd, CheckedDiv, CheckedMul, CheckedShl, CheckedShr, CheckedSub},
    Bounded, One, Zero,
};
use sr_primitives::traits::{As, MaybeDisplay, MaybeSerializeDebug, Member, SimpleArithmetic};
use sr_std::prelude::Vec;
use srml_support::Parameter;

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

    fn new(Vec<u8>) -> Self;
    fn account_info(&self) -> Option<(Vec<u8>, Self::AccountId)>;
    fn split(&self) -> Vec<Vec<u8>>;
}
