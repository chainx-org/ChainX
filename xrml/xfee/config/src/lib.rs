// Copyright 2018 Chainpool
#![cfg_attr(not(feature = "std"), no_std)]

extern crate sr_primitives;
extern crate sr_std;

use sr_primitives::traits::Member;
use sr_std::marker::PhantomData;

pub struct FeeMap<Call> {
    pub _marker: PhantomData<Call>,
}

impl<Call> FeeMap<Call>
where
    Call: Member,
{
    pub fn find(&self, function: &Call) -> Option<u64> {
        match function {
            _ => None,
        }
    }
}
