// Copyrgith 2019 Chainpool

#![cfg_attr(not(feature = "std"), no_std)]
extern crate secp256k1;
extern crate tiny_keccak;
#[macro_use]
extern crate srml_support;
extern crate srml_system as system;
#[macro_use]
extern crate parity_codec_derive;
extern crate parity_codec as codec;
extern crate sr_primitives;
extern crate sr_std as rstd;
extern crate srml_balances as balances;
extern crate xrml_xaccounts as xaccounts;
extern crate xrml_xassets_assets as xassets;
extern crate xrml_xassets_records as xrecords;
#[cfg(test)]
#[macro_use]
extern crate hex_literal;
extern crate xr_primitives;

use codec::Encode;
use rstd::prelude::*;
#[cfg(feature = "std")]
use sr_primitives::traits::Zero;
use srml_support::dispatch::Result;
use srml_support::{StorageMap, StorageValue};
use system::ensure_signed;
use tiny_keccak::keccak256;
use xassets::{Chain as ChainDef, ChainT};
use xr_primitives::generic::Extracter;
use xr_primitives::traits::Extractable;

impl<T: Trait> ChainT for Module<T> {
    const TOKEN: &'static [u8] = b"XDOT";

    fn chain() -> ChainDef {
        ChainDef::Ethereum
    }

    fn check_addr(_addr: &[u8], _: &[u8]) -> Result {
        Ok(())
    }
}

/// Configuration trait.
pub trait Trait: xassets::Trait + xrecords::Trait + xaccounts::Trait {
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

pub type EthereumAddress = [u8; 20];

#[derive(Encode, Decode, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct EcdsaSignature(pub [u8; 32], pub [u8; 32], pub i8);

/// An event in this module.
decl_event!(
    pub enum Event<T>
    where
        <T as balances::Trait>::Balance,
        <T as system::Trait>::AccountId
    {
        /// Someone claimed some DOTs.
        Claimed(AccountId, EthereumAddress, Balance),
    }
);

decl_storage! {
    // A macro for the Storage trait, and its implementation, for this module.
    // This allows for type-safe usage of the Substrate storage database, so you can
    // keep things around between blocks.
    trait Store for Module<T: Trait> as Claims {
        Claims get(claims) build(|config: &GenesisConfig<T>| {
            config.claims.iter().map(|(a, b)| (a.clone(), b.clone())).collect::<Vec<_>>()
        }): map EthereumAddress => Option<T::Balance>;
        Total get(total) build(|config: &GenesisConfig<T>| {
            config.claims.iter().fold(Zero::zero(), |acc: T::Balance, &(_, n)| acc + n)
        }): T::Balance;
    }
    add_extra_genesis {
        config(claims): Vec<(EthereumAddress, T::Balance)>;
    }
}

fn ecdsa_recover(sig: &EcdsaSignature, msg: &[u8; 32]) -> Option<[u8; 64]> {
    let v = secp256k1::RecoveryId::parse(if sig.2 > 26 { sig.2 - 27 } else { sig.2 } as u8).ok()?;
    let rs = (sig.0, sig.1)
        .using_encoded(secp256k1::Signature::parse_slice)
        .ok()?;
    let pubkey = secp256k1::recover(&secp256k1::Message::parse(msg), &rs, &v).ok()?;
    let mut res = [0u8; 64];
    res.copy_from_slice(&pubkey.serialize()[1..65]);
    Some(res)
}

fn eth_recover(s: &EcdsaSignature, sign_data: &[u8]) -> Option<EthereumAddress> {
    let msg = keccak256(sign_data);
    let mut res = EthereumAddress::default();
    res.copy_from_slice(&keccak256(&ecdsa_recover(s, &msg)?[..])[12..]);
    Some(res)
}

fn contains(lvec: Vec<u8>, svec: Vec<u8>) -> Option<Vec<u8>> {
    let llen = lvec.len();
    let slen = svec.len();
    let mut op_vec = lvec;
    for _ in 0..llen - slen {
        if op_vec.starts_with(&svec) {
            return Some(svec);
        }
        op_vec.remove(0);
    }
    None
}

fn deposit_token<T: Trait>(who: &T::AccountId, balance: T::Balance) {
    let token: xassets::Token = <Module<T> as xassets::ChainT>::TOKEN.to_vec();
    let _ = <xrecords::Module<T>>::deposit(&who, &token, balance);
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        /// Deposit one of this module's events by using the default implementation.
        fn deposit_event<T>() = default;

        /// Make a claim.
        fn claim(origin, ethereum_signature: EcdsaSignature, sign_data: Vec<u8>, input_data: Vec<u8>) {
            // This is a public call, so we ensure that the origin is some signed account.
            let sender = ensure_signed(origin)?;

            let input = contains(sign_data.clone(), input_data).ok_or("sign_data not contains input_data")?;
            let (node_name, who) = Extracter::<T::AccountId>::new(input).account_info().ok_or("extracter account_id error")?;

            let signer = eth_recover(&ethereum_signature, &sign_data).ok_or("Invalid Ethereum signature")?;

            let balance_due = <Claims<T>>::take(&signer)
                .ok_or("Ethereum address has no claim")?;

            <Total<T>>::mutate(|t| if *t < balance_due {
                panic!("Logic error: Pot less than the total of claims!")
            } else {
                *t -= balance_due
            });
            deposit_token::<T>(&who, balance_due);

            xaccounts::apply_update_binding::<T>(who, signer.to_vec(), node_name, ChainDef::Ethereum);
            // Let's deposit an event to let the outside world know this happened.
            Self::deposit_event(RawEvent::Claimed(sender, signer, balance_due));
        }
    }
}

#[cfg(test)]
mod tests {}
