// Copyright 2019-2022 ChainX Project Authors. Licensed under GPL-3.0.

#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};

use sp_core::crypto::AccountId32;
use sp_runtime::RuntimeDebug;
use sp_runtime_interface::runtime_interface;

#[derive(Clone, Copy, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
pub enum Ss58CheckError {
    /// Bad alphabet.
    BadBase58,
    /// Bad length.
    BadLength,
    /// Unknown ss58 address format.
    UnknownSs58AddressFormat,
    /// Invalid checksum.
    InvalidChecksum,
    /// Invalid prefix
    InvalidPrefix,
    /// Invalid format.
    InvalidFormat,
    /// Invalid derivation path.
    InvalidPath,
    /// Mismatch version.
    MismatchVersion,
    /// Disallowed SS58 Address Format for this datatype.
    FormatNotAllowed,
}

#[runtime_interface]
pub trait Ss58Codec {
    fn from_ss58check(addr: &[u8]) -> Result<AccountId32, Ss58CheckError> {
        use sp_core::crypto::{PublicError, Ss58Codec};
        let s = String::from_utf8_lossy(addr).into_owned();
        AccountId32::from_ss58check_with_version(&s)
            .map(|(account, _)| {
                // https://github.com/paritytech/substrate/blob/polkadot-v0.9.18/primitives/core/src/crypto.rs#L310
                // Support all ss58 versions.
                account
            })
            .map_err(|err| match err {
                PublicError::BadBase58 => Ss58CheckError::BadBase58,
                PublicError::BadLength => Ss58CheckError::BadLength,
                PublicError::UnknownSs58AddressFormat(_) => {
                    Ss58CheckError::UnknownSs58AddressFormat
                }
                PublicError::InvalidChecksum => Ss58CheckError::InvalidChecksum,
                PublicError::InvalidPrefix => Ss58CheckError::InvalidPrefix,
                PublicError::InvalidFormat => Ss58CheckError::InvalidFormat,
                PublicError::InvalidPath => Ss58CheckError::InvalidPath,
                PublicError::FormatNotAllowed => Ss58CheckError::FormatNotAllowed,
            })
    }
}

#[test]
fn ss58_check() {
    let addr42 = b"5CE864FPj1Z48qrvdCAQ48iTfkcBFMoUWt2UAnR4Np22kZFM";
    let addr44 = b"5PoSc3LCVbJWSxfrSFvSowFJxitmMj4Wtm8jQ9hfJXD1K5vF";
    let pubkey =
        hex::decode("072ec6e199a69a1a38f0299afc083b2b6c85899bdad56d250b2ec39a9788b7a2").unwrap();

    set_default_ss58_version(Ss58AddressFormat::from(44u16));
    let account = ss_58_codec::from_ss58check(addr44).unwrap();
    assert_eq!(AsRef::<[u8]>::as_ref(&account), pubkey.as_slice());
    assert!(ss_58_codec::from_ss58check(addr42).is_ok());

    set_default_ss58_version(Ss58AddressFormat::from(42u16));
    let account = ss_58_codec::from_ss58check(addr42).unwrap();
    assert_eq!(AsRef::<[u8]>::as_ref(&account), pubkey.as_slice());
    assert!(ss_58_codec::from_ss58check(addr44).is_ok());
}
