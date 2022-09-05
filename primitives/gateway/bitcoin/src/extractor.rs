// Copyright 2019-2022 ChainX Project Authors. Licensed under GPL-3.0.

use crate::types::OpReturnAccount;
use frame_support::log::{debug, error};
use sp_core::crypto::AccountId32;
use sp_std::prelude::Vec;

use chainx_primitives::ReferralId;
use xp_gateway_common::{
    from_ss58_check, transfer_aptos_uncheck, transfer_evm_uncheck, transfer_named_uncheck,
};

pub use xp_gateway_common::AccountExtractor;

/// A helper struct that implements the `AccountExtractor` trait for Bitcoin OP_RETURN data.
///
/// OP_RETURN data format:
/// - `account`, e.g. 5VEW3R1T4LR3kDhYwXeeCnYrHRwRaH7E9V1KprypBe68XmY4
/// - `account@referral`, e.g. 5VEW3R1T4LR3kDhYwXeeCnYrHRwRaH7E9V1KprypBe68XmY4@referral1
#[derive(PartialEq, Eq, Clone)]
pub struct OpReturnExtractor;

impl AccountExtractor<AccountId32, ReferralId> for OpReturnExtractor {
    fn extract_account(data: &[u8]) -> Option<(OpReturnAccount<AccountId32>, Option<ReferralId>)> {
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

        let wasm_account = from_ss58_check(account_and_referral[0].as_slice());

        let account = if let Some(v) = wasm_account {
            OpReturnAccount::Wasm(v)
        } else if let Some(v) = transfer_evm_uncheck(account_and_referral[0].as_slice()) {
            OpReturnAccount::Evm(v)
        } else if let Some(v) = transfer_aptos_uncheck(account_and_referral[0].as_slice()) {
            OpReturnAccount::Aptos(v)
        } else {
            let data = transfer_named_uncheck(account_and_referral[0].as_slice())?;
            OpReturnAccount::Named(data.0, data.1)
        };

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
        crypto::{set_default_ss58_version, Ss58AddressFormatRegistry, UncheckedInto},
        H160, H256,
    };

    let addr = "f778a69d4166401048acb0f7b2625e9680609f8859c78e3d28e2549f84f0269a"
        .parse::<H256>()
        .unwrap();
    let mainnet = Ss58AddressFormatRegistry::ChainxAccount.into();
    let testnet = Ss58AddressFormatRegistry::SubstrateAccount.into();

    {
        set_default_ss58_version(mainnet);

        // test for account
        let result = OpReturnExtractor::extract_account(
            "5VEW3R1T4LR3kDhYwXeeCnYrHRwRaH7E9V1KprypBe68XmY4".as_bytes(),
        );
        assert_eq!(
            result,
            Some((OpReturnAccount::Wasm(addr.unchecked_into()), None))
        );

        // test for account and referral
        let result = OpReturnExtractor::extract_account(
            "5VEW3R1T4LR3kDhYwXeeCnYrHRwRaH7E9V1KprypBe68XmY4@referral1".as_bytes(),
        );
        assert_eq!(
            result,
            Some((
                OpReturnAccount::Wasm(addr.unchecked_into()),
                Some(b"referral1".to_vec())
            ))
        );

        let mut key = [0u8; 20];
        key.copy_from_slice(&hex::decode("3800501939F9385CB044F9FB992b97442Cc45e47").unwrap());
        let evm_addr = H160::try_from(key).unwrap();

        let result = OpReturnExtractor::extract_account(
            "0x3800501939F9385CB044F9FB992b97442Cc45e47@referral1".as_bytes(),
        );
        assert_eq!(
            result,
            Some((OpReturnAccount::Evm(evm_addr), Some(b"referral1".to_vec())))
        );

        let result = OpReturnExtractor::extract_account(
            "3800501939F9385CB044F9FB992b97442Cc45e47@referral1".as_bytes(),
        );
        assert_eq!(
            result,
            Some((OpReturnAccount::Evm(evm_addr), Some(b"referral1".to_vec())))
        );

        let mut key = [0u8; 32];
        key.copy_from_slice(
            &hex::decode("eeff357ea5c1a4e7bc11b2b17ff2dc2dcca69750bfef1e1ebcaccf8c8018175b")
                .unwrap(),
        );
        let aptos_addr = H256::try_from(key).unwrap();

        let result = OpReturnExtractor::extract_account(
            "0xeeff357ea5c1a4e7bc11b2b17ff2dc2dcca69750bfef1e1ebcaccf8c8018175b@referral1"
                .as_bytes(),
        );
        assert_eq!(
            result,
            Some((
                OpReturnAccount::Aptos(aptos_addr),
                Some(b"referral1".to_vec())
            ))
        );

        let result = OpReturnExtractor::extract_account(
            "eeff357ea5c1a4e7bc11b2b17ff2dc2dcca69750bfef1e1ebcaccf8c8018175b@referral1".as_bytes(),
        );
        assert_eq!(
            result,
            Some((
                OpReturnAccount::Aptos(aptos_addr),
                Some(b"referral1".to_vec())
            ))
        );

        let name = vec![b's', b'u', b'i'];
        let addr = hex::decode("1dcba11f07596152cf96a9bd358b675d5d5f9506").unwrap();

        let result = OpReturnExtractor::extract_account(
            "sui:0x1dcba11f07596152cf96a9bd358b675d5d5f9506@referral1".as_bytes(),
        );
        assert_eq!(
            result,
            Some((
                OpReturnAccount::Named(name.clone(), addr.clone()),
                Some(b"referral1".to_vec())
            ))
        );

        let result = OpReturnExtractor::extract_account(
            "sui:1dcba11f07596152cf96a9bd358b675d5d5f9506@referral1".as_bytes(),
        );
        assert_eq!(
            result,
            Some((
                OpReturnAccount::Named(name, addr),
                Some(b"referral1".to_vec())
            ))
        );
    }
    {
        set_default_ss58_version(testnet);

        // test for version
        let result = OpReturnExtractor::extract_account(
            "5VEW3R1T4LR3kDhYwXeeCnYrHRwRaH7E9V1KprypBe68XmY4".as_bytes(),
        );
        #[cfg(feature = "ss58check")]
        assert_eq!(
            result,
            Some((OpReturnAccount::Wasm(addr.unchecked_into()), None))
        );
        #[cfg(not(feature = "ss58check"))]
        assert_eq!(
            result,
            Some((OpReturnAccount::Wasm(addr.unchecked_into()), None))
        );
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
        assert_eq!(
            result,
            Some((OpReturnAccount::Wasm(addr.unchecked_into()), None))
        );

        // new checksum
        let result = OpReturnExtractor::extract_account(
            "5C4xGQZwoNEM5mdk2U3vJbFZPr6ZKFSiqWnc9JRDcJ3w334p".as_bytes(),
        );
        assert_eq!(
            result,
            Some((OpReturnAccount::Wasm(addr.unchecked_into()), None))
        );
    }
}
