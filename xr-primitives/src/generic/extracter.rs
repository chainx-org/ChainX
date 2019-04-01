// Copyright 2018-2019 Chainpool.

use parity_codec::Decode;

use rstd::prelude::Vec;
use runtime_primitives::traits::{MaybeDisplay, MaybeSerializeDebug, Member};
use support::Parameter;

use super::b58::from;
use crate::traits::Extractable;

/// Definition of something that the external world might want to say; its
/// existence implies that it has been checked and is good, particularly with
/// regards to the signature.
#[derive(PartialEq, Eq, Clone)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Extracter<AccountId>(Vec<u8>, ::rstd::marker::PhantomData<AccountId>);

impl<AccountId> Extractable for Extracter<AccountId>
where
    AccountId: Parameter + Member + MaybeSerializeDebug + MaybeDisplay + Ord + Default,
{
    type AccountId = AccountId;

    fn new(script: Vec<u8>) -> Self {
        Extracter(script, ::rstd::marker::PhantomData)
    }

    fn account_info(&self) -> Option<(Self::AccountId, Vec<u8>)> {
        let v = self.split();
        let op = &v[0];
        let mut account: Vec<u8> = match from(op.to_vec()) {
            Ok(a) => a,
            Err(_) => return None,
        };

        if account.len() != 35 {
            return None;
        }

        let account_id: Self::AccountId =
            match Decode::decode(&mut account[1..33].to_vec().as_slice()) {
                Some(a) => a,
                None => return None,
            };
        // channel is a validator
        let channel_name = if v.len() > 1 {
            v[1].to_vec()
        } else {
            Vec::new()
        };

        Some((account_id, channel_name))
    }

    fn split(&self) -> Vec<Vec<u8>> {
        self.0.split(|x| *x == b'@').map(|d| d.to_vec()).collect()
    }
}
