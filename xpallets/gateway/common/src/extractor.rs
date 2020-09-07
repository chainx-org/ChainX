// Copyright 2018-2019 Chainpool.

use sp_runtime::AccountId32;
use sp_std::prelude::Vec;

use chainx_primitives::ReferralId;
use xpallet_support::{debug, error, str};

use crate::traits::Extractable;

/// use custom runtime-interface to provide ss58check from outside of runtime. but this feature
/// could not be used in parachain
#[cfg(feature = "ss58check")]
pub fn parse_address(data: &[u8]) -> Option<AccountId32> {
    use xp_io::ss_58_codec::from_ss58check;
    from_ss58check(data)
        .map_err(|e| {
            error!(
                "[parse_address]|parse account error|src:{:?}|reason:{:?}",
                str!(data),
                e
            );
            e
        })
        .ok()
}

/// due to current parachain do not allow custom runtime-interface, thus we just could
/// impl address parse in runtime, and ignore address version check.
/// same to `substrate/core/primitives/src/crypto.rs:trait Ss58Codec`
#[cfg(not(feature = "ss58check"))]
pub fn parse_address(data: &[u8]) -> Option<AccountId32> {
    let mut res: [u8; 32] = Default::default();
    let len = res.len();
    // parse data from base58 to raw
    let d = bs58::decode(data)
        .into_vec()
        .map_err(|e| {
            error!(
                "[parse_address]|parse base58 err|e:{:?}|data:{:?}",
                e,
                str!(data)
            );
            e
        })
        .ok()?;
    if d.len() != len + 3 {
        // Invalid length.
        error!(
            "[parse_address]|bad length|data len:{:}|len:{:}",
            d.len(),
            len
        );
        return None;
    }
    // ignore address version check, for can't calc blake512 in runtime
    res.copy_from_slice(&d[1..len + 1]);
    Some(res.into())
}

fn split(data: &[u8]) -> Vec<Vec<u8>> {
    data.split(|x| *x == b'@').map(|d| d.to_vec()).collect()
}

/// Definition of something that the external world might want to say; its
/// existence implies that it has been checked and is good, particularly with
/// regards to the signature.
#[derive(PartialEq, Eq, Clone)]
pub struct Extractor;

impl Extractable<AccountId32> for Extractor {
    /// Extracts the target deposit AccountId in ChainX and possible referral
    /// from the slice in the format of "AccountId@ReferralId".
    ///
    /// NOTE: `@` is used to be the separator. `ReferralId` is an essential
    /// attribute of Staking validator.
    fn account_info(data: &[u8]) -> Option<(AccountId32, Option<ReferralId>)> {
        let v = split(data);
        if v.is_empty() {
            error!("[account_info]|can't parse data|data:{:?}", str!(data));
            return None;
        }

        let target_account = parse_address(&v[0][..])?;

        let referral_id = if v.len() > 1 {
            Some(v[1].to_vec())
        } else {
            None
        };

        debug!(
            "[extract_account_info]||target_account:{:?}|referral_id:{:?}",
            target_account, referral_id
        );
        Some((target_account, referral_id))
    }
}

#[test]
fn test_extractor() {
    use sp_core::{
        crypto::{set_default_ss58_version, Ss58AddressFormat, UncheckedInto},
        H256,
    };
    let addr: Vec<u8> =
        hex::decode("f778a69d4166401048acb0f7b2625e9680609f8859c78e3d28e2549f84f0269a")
            .expect("must be valid hex");
    let addr = H256::from_slice(&addr);
    let mainnet = Ss58AddressFormat::ChainXAccount;
    let testnet = Ss58AddressFormat::SubstrateAccount;
    {
        // test for ed25519 and channel
        set_default_ss58_version(mainnet);
        let result =
            Extractor::account_info("5VEW3R1T4LR3kDhYwXeeCnYrHRwRaH7E9V1KprypBe68XmY4".as_bytes());
        assert_eq!(result, Some((addr.unchecked_into(), None)));

        let result = Extractor::account_info(
            "5VEW3R1T4LR3kDhYwXeeCnYrHRwRaH7E9V1KprypBe68XmY4@channel1".as_bytes(),
        );
        assert_eq!(
            result,
            Some((addr.unchecked_into(), Some(b"channel1".to_vec())))
        );
    }
    {
        // test for sr25519
        set_default_ss58_version(mainnet);
        let result =
            Extractor::account_info("5VEW3R1T4LR3kDhYwXeeCnYrHRwRaH7E9V1KprypBe68XmY4".as_bytes());
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
        let result =
            Extractor::account_info("5C4xGQZwoNEM5mdk2U3vJbFZPr6ZKFSiqWnc9JRDcJ3w2x5D".as_bytes());

        #[cfg(feature = "ss58check")]
        {
            // in ss58check feature, would check ss58version
            assert_eq!(result, None);
        }
        #[cfg(not(feature = "ss58check"))]
        {
            // not in ss58check feature, would not check ss58 version
            assert_eq!(result, Some((addr.unchecked_into(), None)));
        }

        // new checksum
        let result =
            Extractor::account_info("5C4xGQZwoNEM5mdk2U3vJbFZPr6ZKFSiqWnc9JRDcJ3w334p".as_bytes());
        assert_eq!(result, Some((addr.unchecked_into(), None)));
    }
    {
        // test for version
        set_default_ss58_version(testnet);
        let result =
            Extractor::account_info("5VEW3R1T4LR3kDhYwXeeCnYrHRwRaH7E9V1KprypBe68XmY4".as_bytes());
        #[cfg(feature = "ss58check")]
        {
            assert_eq!(result, None);
        }
        #[cfg(not(feature = "ss58check"))]
        {
            assert_eq!(result, Some((addr.unchecked_into(), None)));
        }
    }
}
