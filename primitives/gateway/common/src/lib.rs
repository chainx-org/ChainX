// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

//! Common concepts with regard to the ChainX gateway system.

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(missing_docs)]

use sp_core::crypto::AccountId32;

use xp_logging::error;

/// Trait for extracting the account and possible extra (e.g. like referral) from
/// the external world data (e.g. like btc op_return).
pub trait AccountExtractor<Account, Extra: AsRef<[u8]>> {
    /// Extract the account and possible extra from the data.
    fn extract_account(data: &[u8]) -> Option<(Account, Option<Extra>)>;
}

impl<Account, Extra: AsRef<[u8]>> AccountExtractor<Account, Extra> for () {
    fn extract_account(_data: &[u8]) -> Option<(Account, Option<Extra>)> {
        None
    }
}

/// Verify if the raw account is a properly encoded SS58Check address.
pub fn from_ss58_check(raw_account: &[u8]) -> Option<AccountId32> {
    // Use custom runtime-interface to provide ss58check from outside of runtime.
    // But this feature could not be used in parachain.
    #[cfg(feature = "ss58check")]
    {
        xp_io::ss_58_codec::from_ss58check(raw_account)
            .map_err(|err| {
                error!(
                    "[from_ss58_check] Parse data:{:?} into account error:{:?}",
                    hex::encode(raw_account),
                    err
                );
                err
            })
            .ok()
    }

    // Due to current parachain do not allow custom runtime-interface, thus we just could
    // impl ss58 check in runtime, and ignore address version and hash checksum check.
    // Same as `substrate/core/primitives/src/crypto.rs:trait Ss58Codec`
    #[cfg(not(feature = "ss58check"))]
    {
        let mut res: [u8; 32] = Default::default();
        let len = res.as_mut().len();
        let d = bs58::decode(raw_account)
            .into_vec()
            .map_err(|err| {
                error!(
                    "[from_ss58_check] Base58 decode {} error:{}",
                    hex::encode(raw_account),
                    err
                );
                err
            })
            .ok()?;
        if d.len() != len + 3 {
            // Invalid length.
            error!(
                "[from_ss58_check] Bad length, data len:{}, len:{}",
                d.len(),
                len
            );
            return None;
        }

        // ignore the ss58 address version checking and hash checksum checking

        res.as_mut().copy_from_slice(&d[1..len + 1]);
        Some(res.into())
    }
}
