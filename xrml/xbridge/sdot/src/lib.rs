// Copyrgith 2019 Chainpool

#![cfg_attr(not(feature = "std"), no_std)]

// substrate core
use rstd::prelude::*;
#[cfg(feature = "std")]
use sr_primitives::traits::Zero;
// substrate runtime
use support::{
    decl_event, decl_module, decl_storage, dispatch::Result, ensure, StorageMap, StorageValue,
};
use system::ensure_signed;

// chainx core
use xr_primitives::generic::Extracter;
use xr_primitives::traits::Extractable;
// chainx runtime
use xassets::{Chain, ChainT};

use parity_codec::{Decode, Encode};
use tiny_keccak::keccak256;

pub type EthereumAddress = [u8; 20];

#[derive(Encode, Decode, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct EcdsaSignature(pub [u8; 32], pub [u8; 32], pub i8);

/// Configuration trait.
pub trait Trait: xaccounts::Trait + xassets::Trait + xrecords::Trait {
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

decl_event!(
    pub enum Event<T>
    where
        <T as system::Trait>::AccountId,
        <T as balances::Trait>::Balance
    {
        /// Someone claimed some DOTs.
        Claimed(AccountId, EthereumAddress, Balance),
    }
);

decl_storage! {
    trait Store for Module<T: Trait> as XBridgeOfSDOT {
        pub Claims get(claims) build(|config: &GenesisConfig<T>| {
            config.claims.iter().map(|(a, b)| (a.clone(), b.clone())).collect::<Vec<_>>()
        }): map EthereumAddress => Option<T::Balance>;
        pub Total get(total) build(|config: &GenesisConfig<T>| {
            config.claims.iter().fold(Zero::zero(), |acc: T::Balance, &(_, n)| acc + n)
        }): T::Balance;
    }
    add_extra_genesis {
        config(claims): Vec<(EthereumAddress, T::Balance)>;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        /// Deposit one of this module's events by using the default implementation.
        fn deposit_event<T>() = default;

        /// Make a claim.
        fn claim(origin, ethereum_signature: EcdsaSignature, sign_data: Vec<u8>, input_data: Vec<u8>) -> Result {
            // This is a public call, so we ensure that the origin is some signed account.
            let sender = ensure_signed(origin)?;

            let input = contains(sign_data.clone(), input_data).ok_or("sign_data not contains input_data")?;
            let (who, node_name) = Extracter::<T::AccountId>::new(input).account_info().ok_or("extracter account_id error")?;

            let signer = eth_recover(&ethereum_signature, &sign_data).ok_or("Invalid Ethereum signature")?;

            let balance_due = <Claims<T>>::take(&signer).ok_or("Ethereum address has no claim")?;

            let total = Self::total();
            ensure!(total >= balance_due, "Balance is less than remaining total of claims");
            <Total<T>>::mutate(|t| *t -= balance_due);

            deposit_token::<T>(&who, balance_due);

            xaccounts::apply_update_binding::<T>(who, (Chain::Ethereum, signer.to_vec()), node_name);

            // Let's deposit an event to let the outside world know this happened.
            Self::deposit_event(RawEvent::Claimed(sender, signer, balance_due));

            Ok(())
        }
    }
}

impl<T: Trait> ChainT for Module<T> {
    const TOKEN: &'static [u8] = b"SDOT";

    fn chain() -> Chain {
        Chain::Ethereum
    }

    fn check_addr(_addr: &[u8], _: &[u8]) -> Result {
        Ok(())
    }
}

fn ecdsa_recover(sig: &EcdsaSignature, msg: &[u8; 32]) -> Option<[u8; 64]> {
    let msg = secp256k1::Message::parse(msg);
    let signature = secp256k1::Signature::parse_slice(&(sig.0, sig.1).encode()).ok()?;
    let recovery_id = if sig.2 > 26 { sig.2 - 27 } else { sig.2 };
    let recovery_id = secp256k1::RecoveryId::parse(recovery_id as u8).ok()?;
    let pub_key = secp256k1::recover(&msg, &signature, &recovery_id).ok()?;
    let mut res = [0u8; 64];
    res.copy_from_slice(&pub_key.serialize()[1..65]);
    Some(res)
}

fn eth_recover(s: &EcdsaSignature, sign_data: &[u8]) -> Option<EthereumAddress> {
    let msg = keccak256(sign_data);
    let mut res = EthereumAddress::default();
    res.copy_from_slice(&keccak256(&ecdsa_recover(s, &msg)?[..])[12..]);
    Some(res)
}

fn contains(lvec: Vec<u8>, svec: Vec<u8>) -> Option<Vec<u8>> {
    let mut lvec = lvec;
    for _ in 0..lvec.len() - svec.len() {
        if lvec.starts_with(&svec) {
            return Some(svec);
        }
        lvec.remove(0);
    }
    None
}

fn deposit_token<T: Trait>(who: &T::AccountId, balance: T::Balance) {
    let token: xassets::Token = <Module<T> as xassets::ChainT>::TOKEN.to_vec();
    let _ = <xrecords::Module<T>>::deposit(&who, &token, balance);
}

#[cfg(test)]
mod tests {}
