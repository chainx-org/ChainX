// Copyright 2018 Chainpool.

use super::b58::from;
use sr_io::codec::Decode;
use sr_primitives::traits::{MaybeDisplay, MaybeSerializeDebug, Member};
use sr_std::prelude::Vec;
use srml_support::Parameter;
/// Definition of something that the external world might want to say; its
/// existence implies that it has been checked and is good, particularly with
/// regards to the signature.
#[derive(PartialEq, Eq, Clone)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Extracter<AccountId>(Vec<u8>, ::sr_std::marker::PhantomData<AccountId>);

impl<AccountId> ::traits::Extractable for Extracter<AccountId>
where
    AccountId: Parameter + Member + MaybeSerializeDebug + MaybeDisplay + Ord + Default,
{
    type AccountId = AccountId;

    fn new(script: Vec<u8>) -> Self {
        Extracter(script, ::sr_std::marker::PhantomData)
    }

    fn account_info(&self) -> Option<(Vec<u8>, Self::AccountId)> {
        let v = self.split();
        let op = &v[0];
        let mut account: Vec<u8> = match from(op.to_vec()) {
            Ok(a) => a,
            Err(_) => return None,
        };

        let account_id: Self::AccountId =
            match Decode::decode(&mut account[1..33].to_vec().as_slice()) {
                Some(a) => a,
                None => return None,
            };
        let node_name = &v[1];
        Some((node_name.to_vec(), account_id))
    }

    fn split(&self) -> Vec<Vec<u8>> {
        let s = &self.0;
        let mut iter = s.split(|x| *x == '@' as u8);
        let mut v = Vec::new();
        while let Some(d) = iter.next() {
            let d: Vec<u8> = d.iter().cloned().collect();
            v.push(d)
        }
        v
    }
}
