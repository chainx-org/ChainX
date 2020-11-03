// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

//! Common concepts with regard to the ChainX gateway system.

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(missing_docs)]

use chainx_primitives::ReferralId;

/// Trait for extracting the account from the external world data (e.g. like btc op_return).
pub trait AccountExtractor<AccountId: Default + AsMut<[u8]> + AsRef<[u8]>> {
    /// Returns the target deposit account and possible referral id.
    fn account_info(data: &[u8]) -> Option<(AccountId, Option<ReferralId>)>;

    /// Verify if the data is a properly encoded SS58Check address.
    fn from_ss58_check(data: &[u8]) -> Option<AccountId> {
        // use custom runtime-interface to provide ss58check from outside of runtime. but this feature
        // could not be used in parachain
        #[cfg(feature = "ss58check")]
        {
            xp_io::ss_58_codec::from_ss58check(data)
                .map_err(|err| {
                    // error!(
                    //     "[from_ss58_check] parse data:{?} into account error:{:?}",
                    //     data, err
                    // );
                    err
                })
                .ok()
        }

        // due to current parachain do not allow custom runtime-interface,
        // thus we just could impl address parse in runtime, and ignore address version check.
        // same to `substrate/core/primitives/src/crypto.rs:trait Ss58Codec`
        #[cfg(not(feature = "ss58check"))]
        {
            let mut res = AccountId::default();
            let len = res.as_mut().len();
            let d = bs58::decode(data)
                .into_vec()
                .map_err(|err| {
                    // error!("[from_ss58_check] Base58 decode {:?} error:{}", data, err);
                    err
                })
                .ok()?; // failure here would be invalid encoding.
            if d.len() != len + 3 {
                // Invalid length.
                // error!(
                //     "[from_ss58_check] Bad length, data len:{}, len:{}",
                //     d.len(),
                //     len
                // );
                return None;
            }

            // ignore checksum checking, since we can't calc blake512 in runtime

            res.as_mut().copy_from_slice(&d[1..len + 1]);
            Some(res)
        }
    }
}

impl<AccountId: Default + AsMut<[u8]> + AsRef<[u8]>> AccountExtractor<AccountId> for () {
    fn account_info(_data: &[u8]) -> Option<(AccountId, Option<ReferralId>)> {
        None
    }

    fn from_ss58_check(_data: &[u8]) -> Option<AccountId> {
        None
    }
}
