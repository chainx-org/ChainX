pub use integer_sqrt::IntegerSquareRoot;
pub use num_traits::{
    ops::checked::{CheckedAdd, CheckedDiv, CheckedMul, CheckedShl, CheckedShr, CheckedSub},
    Bounded, One, Zero,
};

use sr_primitives::traits::{As, MaybeDisplay, Member, SimpleArithmetic};

/// Work together with sr_primitives::traits::Applyable
pub trait Accelerable: Sized + Send + Sync {
    type AccountId: Member + MaybeDisplay;
    type Index: Member + MaybeDisplay + SimpleArithmetic;
    type Call: Member;
    type Acceleration: Member + MaybeDisplay + SimpleArithmetic + Copy + As<u64>;

    fn acceleration(&self) -> Option<Self::Acceleration>;
}
