// Copyright 2018-2019 Chainpool.

use sp_runtime::AccountId32;
use sp_std::prelude::Vec;

use chainx_primitives::Name;
use xp_io::ss_58_codec::from_ss58check;
use xpallet_support::{debug, error, str};

use crate::traits::Extractable;

/// Definition of something that the external world might want to say; its
/// existence implies that it has been checked and is good, particularly with
/// regards to the signature.
#[derive(PartialEq, Eq, Clone)]
pub struct Extractor;

fn split(data: &[u8]) -> Vec<Vec<u8>> {
    data.split(|x| *x == b'@').map(|d| d.to_vec()).collect()
}

pub fn parse_account_info(data: &[u8]) -> Option<(AccountId32, Option<Name>)> {
    let v = split(data);
    if v.len() < 1 {
        error!("[account_info]|can't parse data|data:{:?}", str!(data));
        return None;
    }

    let op = &v[0];
    let res = from_ss58check(&op[..])
        .map_err(|e| {
            error!(
                "[parse_account_info]|parse account error|src:{:?}|reason:{:?}",
                str!(&op[..]),
                e
            );
            e
        })
        .ok()?;

    // channel is a validator
    let channel_name = if v.len() > 1 {
        Some(v[1].to_vec())
    } else {
        None
    };

    debug!(
        "[account_info]|parse account info success!|who:{:?}|channel:{:?}",
        res, channel_name
    );
    Some((res, channel_name))
}

impl Extractable<AccountId32> for Extractor {
    /// same to `substrate/core/primitives/src/crypto.rs:trait Ss58Codec`
    fn account_info(data: &[u8]) -> Option<(AccountId32, Option<Name>)> {
        parse_account_info(data)
    }
}

#[test]
fn test_extractor() {
    use sp_core::{
        crypto::{set_default_ss58_version, Ss58AddressFormat, UncheckedInto},
        ed25519::Public,
        H256,
    };
    let addr: Vec<u8> =
        hex::decode("f778a69d4166401048acb0f7b2625e9680609f8859c78e3d28e2549f84f0269a")
            .expect("must be valid hex");
    let addr = H256::from_slice(&addr);
    let mainnet = Ss58AddressFormat::Custom(44); // todo change this when update substrate
    let testnet = Ss58AddressFormat::SubstrateAccount;
    {
        // test for ed25519 and channel
        set_default_ss58_version(mainnet);
        let result = Extractor::<Public>::account_info(
            "5VEW3R1T4LR3kDhYwXeeCnYrHRwRaH7E9V1KprypBe68XmY4".as_bytes(),
        );
        assert_eq!(result, Some((addr.unchecked_into(), None)));

        let result = Extractor::<Public>::account_info(
            "5VEW3R1T4LR3kDhYwXeeCnYrHRwRaH7E9V1KprypBe68XmY4@channel1".as_bytes(),
        );
        assert_eq!(
            result,
            Some((addr.unchecked_into(), Some(b"channel1".to_vec())))
        );
    }
    {
        // test for sr25519
        use sp_core::sr25519::Public;
        set_default_ss58_version(mainnet);
        let result = Extractor::<Public>::account_info(
            "5VEW3R1T4LR3kDhYwXeeCnYrHRwRaH7E9V1KprypBe68XmY4".as_bytes(),
        );
        assert_eq!(result, Some((addr.unchecked_into(), None)));
    }
    {
        // test for checksum
        set_default_ss58_version(testnet);
        // old checksum
        let addr: Vec<u8> =
            hex::decode("00308187439ac204df9e299e1e54a00000000bf348e03dad679737c91871dc53")
                .expect("must be valid hex");
        let addr = H256::from_slice(&addr);
        let result = Extractor::<Public>::account_info(
            "5C4xGQZwoNEM5mdk2U3vJbFZPr6ZKFSiqWnc9JRDcJ3w2x5D".as_bytes(),
        );
        assert_eq!(result, None);
        // new checksum
        let result = Extractor::<Public>::account_info(
            "5C4xGQZwoNEM5mdk2U3vJbFZPr6ZKFSiqWnc9JRDcJ3w334p".as_bytes(),
        );
        assert_eq!(result, Some((addr.unchecked_into(), None)));
    }
    {
        // test for version
        set_default_ss58_version(testnet);
        let result = Extractor::<Public>::account_info(
            "5VEW3R1T4LR3kDhYwXeeCnYrHRwRaH7E9V1KprypBe68XmY4".as_bytes(),
        );
        assert_eq!(result, None);
    }
}
