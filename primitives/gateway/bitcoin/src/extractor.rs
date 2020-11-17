// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use sp_core::crypto::AccountId32;
use sp_std::prelude::Vec;

use chainx_primitives::ReferralId;
use xp_gateway_common::from_ss58_check;
use xp_logging::{debug, error};

pub use xp_gateway_common::AccountExtractor;

/// A helper struct that implements the `AccountExtractor` trait for Bitcoin OP_RETURN data.
///
/// OP_RETURN data format:
/// - `account`, e.g. 5VEW3R1T4LR3kDhYwXeeCnYrHRwRaH7E9V1KprypBe68XmY4
/// - `account@referral`, e.g. 5VEW3R1T4LR3kDhYwXeeCnYrHRwRaH7E9V1KprypBe68XmY4@referral1
#[derive(PartialEq, Eq, Clone)]
pub struct OpReturnExtractor;

impl AccountExtractor<AccountId32, ReferralId> for OpReturnExtractor {
    fn extract_account(data: &[u8]) -> Option<(AccountId32, Option<ReferralId>)> {
        let account_and_referral = data
            .split(|x| *x == b'@')
            .map(|d| d.to_vec())
            .collect::<Vec<_>>();

        if account_and_referral.is_empty() {
            error!(
                "[extract_account] Can't extract account from data:{:?}",
                hex::encode(data)
            );
            return None;
        }

        let account = from_ss58_check(account_and_referral[0].as_slice())?;
        let referral = if account_and_referral.len() > 1 {
            Some(account_and_referral[1].to_vec())
        } else {
            None
        };

        debug!(
            "[extract_account] account:{:?}, referral:{:?}",
            account, referral
        );
        Some((account, referral))
    }
}

#[test]
fn test_opreturn_extractor() {
    use sp_core::{
        crypto::{set_default_ss58_version, Ss58AddressFormat, UncheckedInto},
        H256,
    };

    let addr = "f778a69d4166401048acb0f7b2625e9680609f8859c78e3d28e2549f84f0269a"
        .parse::<H256>()
        .unwrap();
    let mainnet = Ss58AddressFormat::ChainXAccount;
    let testnet = Ss58AddressFormat::SubstrateAccount;

    {
        set_default_ss58_version(mainnet);

        // test for account
        let result = OpReturnExtractor::extract_account(
            "5VEW3R1T4LR3kDhYwXeeCnYrHRwRaH7E9V1KprypBe68XmY4".as_bytes(),
        );
        assert_eq!(result, Some((addr.unchecked_into(), None)));

        // test for account and referral
        let result = OpReturnExtractor::extract_account(
            "5VEW3R1T4LR3kDhYwXeeCnYrHRwRaH7E9V1KprypBe68XmY4@referral1".as_bytes(),
        );
        assert_eq!(
            result,
            Some((addr.unchecked_into(), Some(b"referral1".to_vec())))
        );
    }
    {
        set_default_ss58_version(testnet);

        // test for version
        let result = OpReturnExtractor::extract_account(
            "5VEW3R1T4LR3kDhYwXeeCnYrHRwRaH7E9V1KprypBe68XmY4".as_bytes(),
        );
        #[cfg(feature = "ss58check")]
        assert_eq!(result, None);
        #[cfg(not(feature = "ss58check"))]
        assert_eq!(result, Some((addr.unchecked_into(), None)));
    }
    {
        // test for checksum
        set_default_ss58_version(testnet);

        let addr = "00308187439ac204df9e299e1e54a00000000bf348e03dad679737c91871dc53"
            .parse::<H256>()
            .unwrap();

        // old checksum
        let result = OpReturnExtractor::extract_account(
            "5C4xGQZwoNEM5mdk2U3vJbFZPr6ZKFSiqWnc9JRDcJ3w2x5D".as_bytes(),
        );

        // would check ss58version
        #[cfg(feature = "ss58check")]
        assert_eq!(result, None);
        // would not check ss58 version and hash checksum
        #[cfg(not(feature = "ss58check"))]
        assert_eq!(result, Some((addr.unchecked_into(), None)));

        // new checksum
        let result = OpReturnExtractor::extract_account(
            "5C4xGQZwoNEM5mdk2U3vJbFZPr6ZKFSiqWnc9JRDcJ3w334p".as_bytes(),
        );
        assert_eq!(result, Some((addr.unchecked_into(), None)));
    }
}
