// Copyright 2018-2019 Chainpool.

use rstd::prelude::Vec;
use xr_primitives::{generic::b58, Name};

use crate::traits::Extractable;

/// Definition of something that the external world might want to say; its
/// existence implies that it has been checked and is good, particularly with
/// regards to the signature.
#[derive(PartialEq, Eq, Clone)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Extractor<AccountId>(Vec<u8>, ::rstd::marker::PhantomData<AccountId>);

fn split(data: &[u8]) -> Vec<Vec<u8>> {
    data.split(|x| *x == b'@').map(|d| d.to_vec()).collect()
}

impl<AccountId> Extractable<AccountId> for Extractor<AccountId>
where
    AccountId: Default + AsMut<[u8]> + AsRef<[u8]>,
{
    /// same to `substrate/core/primitives/src/crypto.rs:trait Ss58Codec`
    fn account_info(data: &[u8], addr_type: u8) -> Option<(AccountId, Option<Name>)> {
        let v = split(data);
        if v.len() < 1 {
            return None;
        }

        let op = &v[0];
        let d: Vec<u8> = match b58::from(op) {
            Ok(a) => a,
            Err(_) => return None,
        };

        let mut res = AccountId::default();
        let len = res.as_ref().len();
        if d.len() != len + 3 {
            // Invalid length. AccountId is Public, Public is 32 bytes, 1 bytes version, 2 bytes checksum
            return None;
        }

        // Check if the deposit address matches the current network.
        //
        // The first byte of pubkey indicates the network type.
        if d[0] != addr_type {
            return None;
        }
        // check checksum

        // first byte is version
        res.as_mut().copy_from_slice(&d[1..len + 1]);
        // channel is a validator
        let channel_name = if v.len() > 1 {
            Some(v[1].to_vec())
        } else {
            None
        };

        Some((res, channel_name))
    }
}

#[test]
fn test_extractor() {
    use rustc_hex::FromHex;
    use substrate_primitives::crypto::UncheckedInto;
    use substrate_primitives::H256;
    let addr: Vec<u8> = "5f423525f65a2b6eb5866a9159dee5edebb00c85e43147bf02ef1921590c4df1"
        .from_hex()
        .unwrap();
    let addr = H256::from_slice(&addr);

    {
        use substrate_primitives::ed25519::Public;

        let result = Extractor::<Public>::account_info(
            "5EDc5F7Dur9stLhvD1eBwv35NxyjkzMs9oJHFBUbzYaikccb".as_bytes(),
        );
        assert_eq!(result, Some((addr.unchecked_into(), b"".to_vec())));

        let result = Extractor::<Public>::account_info(
            "5EDc5F7Dur9stLhvD1eBwv35NxyjkzMs9oJHFBUbzYaikccb@channel1".as_bytes(),
        );
        assert_eq!(result, Some((addr.unchecked_into(), b"channel1".to_vec())));
    }
    {
        use substrate_primitives::sr25519::Public;

        let result = Extractor::<Public>::account_info(
            "5EDc5F7Dur9stLhvD1eBwv35NxyjkzMs9oJHFBUbzYaikccb".as_bytes(),
        );
        assert_eq!(result, Some((addr.unchecked_into(), b"".to_vec())));
    }
}
