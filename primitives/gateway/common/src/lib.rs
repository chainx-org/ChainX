// Copyright 2019-2022 ChainX Project Authors. Licensed under GPL-3.0.

//! Common concepts with regard to the ChainX gateway system.

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(missing_docs)]
use codec::{Decode, Encode};
use scale_info::TypeInfo;
use sp_core::{crypto::AccountId32, RuntimeDebug, H160};

use frame_support::log::error;

/// OpReturn supports evm and substrate addresses
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum OpReturnAccount<AccountId> {
    /// Evm address
    Evm(H160),
    /// Wasm address
    Wasm(AccountId),
}

/// Trait for extracting the account and possible extra data (e.g. referral) from
/// the external world data (e.g. btc op_return).
pub trait AccountExtractor<Account, Extra: AsRef<[u8]>> {
    /// Extract the account and possible extra from the data.
    fn extract_account(data: &[u8]) -> Option<(OpReturnAccount<Account>, Option<Extra>)>;
}

impl<Account, Extra: AsRef<[u8]>> AccountExtractor<Account, Extra> for () {
    fn extract_account(_data: &[u8]) -> Option<(OpReturnAccount<Account>, Option<Extra>)> {
        None
    }
}

/// Transfer slice into unchecked evm address
pub fn transfer_evm_uncheck(raw_account: &[u8]) -> Option<H160> {
    let data = if raw_account.len() == 20 {
        raw_account.to_vec()
    } else if raw_account.len() == 40 {
        hex::decode(raw_account).ok()?
    } else if raw_account.len() == 42 {
        let mut key = [0u8; 40];
        // remove 0x prefix
        key.copy_from_slice(&raw_account[2..42]);
        hex::decode(key).ok()?
    } else {
        return None;
    };

    let mut key = [0u8; 20];
    key.copy_from_slice(&data);
    H160::try_from(key).ok()
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
    // impl ss58 check in runtime, and ignore ss58 address version and hash checksum checking.
    // Same as `substrate/primitives/core/src/crypto.rs:trait Ss58Codec`
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

        // ignore the ss58 address version and hash checksum checking

        res.as_mut().copy_from_slice(&d[1..len + 1]);
        Some(res.into())
    }
}
