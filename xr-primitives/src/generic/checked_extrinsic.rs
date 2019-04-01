// Copyright 2018-2019 Chainpool.

//! Generic implementation of an extrinsic that has passed the verification
//! stage.

use runtime_primitives::traits::{Applyable, MaybeDisplay, Member, SimpleArithmetic};

use crate::traits::Accelerable;

/// Definition of something that the external world might want to say; its
/// existence implies that it has been checked and is good, particularly with
/// regards to the signature.
#[derive(PartialEq, Eq, Clone)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct CheckedExtrinsic<AccountId, Index, Call, Acceleration> {
    /// Who this purports to be from and the number of extrinsics have come before
    /// from the same signer, if anyone (note this is not a signature).
    pub signed: Option<(AccountId, Index, Acceleration)>,
    /// The function that should be called.
    pub function: Call,
}

impl<AccountId, Index, Call, Acceleration> Applyable
    for CheckedExtrinsic<AccountId, Index, Call, Acceleration>
where
    AccountId: Member + MaybeDisplay,
    Index: Member + MaybeDisplay + SimpleArithmetic,
    Acceleration: Member + MaybeDisplay + SimpleArithmetic,
    Call: Member,
{
    type AccountId = AccountId;
    type Index = Index;
    type Call = Call;

    fn index(&self) -> Option<&Self::Index> {
        self.signed.as_ref().map(|x| &x.1)
    }

    fn sender(&self) -> Option<&Self::AccountId> {
        self.signed.as_ref().map(|x| &x.0)
    }

    fn deconstruct(self) -> (Self::Call, Option<Self::AccountId>) {
        (self.function, self.signed.map(|x| x.0))
    }
}

impl<AccountId, Index, Call, Acceleration> Accelerable
    for CheckedExtrinsic<AccountId, Index, Call, Acceleration>
where
    AccountId: Member + MaybeDisplay,
    Index: Member + MaybeDisplay + SimpleArithmetic,
    Call: Member,
    Acceleration: Member + MaybeDisplay + SimpleArithmetic + Copy,
{
    type AccountId = AccountId;
    type Index = Index;
    type Call = Call;
    type Acceleration = Acceleration;

    fn acceleration(&self) -> Option<Self::Acceleration> {
        self.signed.as_ref().map(|x| x.2)
    }
}
