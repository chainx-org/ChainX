// Copyright 2018-2019 Chainpool.

#![cfg(test)]

use super::*;

use primitives::testing::{Digest, DigestItem, Header};
use primitives::traits::{BlakeTwo256, IdentityLookup};
use substrate_primitives::H256;
use support::impl_outer_origin;

impl_outer_origin! {
    pub enum Origin for Test {}
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Test;

impl system::Trait for Test {
    type Origin = Origin;
    type Index = u64;
    type BlockNumber = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type Digest = Digest;
    type AccountId = u64;
    type Lookup = IdentityLookup<u64>;
    type Header = Header;
    type Event = ();
    type Log = DigestItem;
}
