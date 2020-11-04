// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};

use sp_core::crypto::AccountId32;
use sp_runtime_interface::runtime_interface;

#[derive(Encode, Decode, Clone, Copy, sp_runtime::RuntimeDebug)]
pub enum Ss58CheckError {
    /// Bad alphabet.
    BadBase58,
    /// Bad length.
    BadLength,
    /// Unknown version.
    UnknownVersion,
    /// Unknown version.
    UnmatchedVersion,
    /// Invalid checksum.
    InvalidChecksum,
    /// Invalid format.
    InvalidFormat,
    /// Unknown.
    Unknown,
}

#[runtime_interface]
pub trait Ss58Codec {
    fn from_ss58check(addr: &[u8]) -> Result<AccountId32, Ss58CheckError> {
        use sp_core::crypto::{PublicError, Ss58AddressFormat, Ss58Codec};
        let s = String::from_utf8_lossy(addr).into_owned();
        AccountId32::from_ss58check_with_version(&s)
            .map_err(|e| match e {
                PublicError::BadBase58 => Ss58CheckError::BadBase58,
                PublicError::BadLength => Ss58CheckError::BadLength,
                PublicError::UnknownVersion => Ss58CheckError::UnknownVersion,
                PublicError::InvalidChecksum => Ss58CheckError::InvalidChecksum,
                PublicError::InvalidFormat => Ss58CheckError::InvalidFormat,
                PublicError::InvalidPath => Ss58CheckError::Unknown,
            })
            .and_then(|(accountid, ver)| {
                let ver_u8: u8 = ver.into();
                let default_u8: u8 = Ss58AddressFormat::default().into();
                if ver_u8 != default_u8 {
                    Err(Ss58CheckError::UnmatchedVersion)
                } else {
                    Ok(accountid)
                }
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ss58_check() {
        use sp_core::crypto::{set_default_ss58_version, Ss58AddressFormat};
        let format = Ss58AddressFormat::Custom(42);
        set_default_ss58_version(format);
        let pubkey =
            hex::decode("072ec6e199a69a1a38f0299afc083b2b6c85899bdad56d250b2ec39a9788b7a2")
                .expect("must success");
        let addr42 = b"5CE864FPj1Z48qrvdCAQ48iTfkcBFMoUWt2UAnR4Np22kZFM";
        let addr44 = b"5PoSc3LCVbJWSxfrSFvSowFJxitmMj4Wtm8jQ9hfJXD1K5vF";
        let accountid = ss_58_codec::from_ss58check(addr42).expect("parse must success");
        assert_eq!(AsRef::<[u8]>::as_ref(&accountid), pubkey.as_slice());

        assert!(ss_58_codec::from_ss58check(addr44).is_err());
    }
}
