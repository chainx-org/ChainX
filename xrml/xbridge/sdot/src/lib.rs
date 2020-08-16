// Copyrgith 2019 Chainpool

#![cfg_attr(not(feature = "std"), no_std)]

mod tests;
pub mod types;

use parity_codec::Encode;
use tiny_keccak::keccak256;

// Substrate
#[cfg(feature = "std")]
use primitives::traits::Zero;
use rstd::prelude::*;
use support::{
    decl_event, decl_module, decl_storage, dispatch::Result, ensure, StorageMap, StorageValue,
};
use system::ensure_signed;

// ChainX
use xassets::{Chain, ChainT};
use xbridge_common::traits::{CrossChainBinding, Extractable};
use xr_primitives::Name;
use xsupport::{error, warn};

pub use self::types::{EcdsaSignature, EthereumAddress};

/// Configuration trait.
pub trait Trait: xsystem::Trait + xassets::Trait + xrecords::Trait {
    type AccountExtractor: Extractable<Self::AccountId>;
    type CrossChainProvider: CrossChainBinding<Self::AccountId, EthereumAddress>;
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

decl_event!(
    pub enum Event<T>
    where
        <T as system::Trait>::AccountId,
        <T as xassets::Trait>::Balance
    {
        /// Someone claimed some DOTs.
        Claimed(AccountId, EthereumAddress, Balance),
    }
);

decl_storage! {
    trait Store for Module<T: Trait> as XBridgeOfSDOT {
        pub Claims get(claims) build(|config: &GenesisConfig<T>| {
            config.claims.iter().map(|(a, b)| (*a, *b)).collect::<Vec<_>>()
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

            // Recover the Ethereum address from signature.
            let signer = match recover_eth_address(&ethereum_signature, &sign_data, &input_data) {
                Some(eth_address) => eth_address,
                None => {
                    error!("[sdot_claim]|Invalid Ethereum transaction signature|signature:{:?}|raw:{:?}|data:{:?}", ethereum_signature, sign_data, input_data);
                    return Err("Invalid Ethereum transaction signature");
                }
            };

            let addr_type = xsystem::Module::<T>::address_type();
            let (account_id, channel_name) = handle_input_data::<T>(&input_data, addr_type).ok_or("Extract account info error")?;

            let balance = match <Claims<T>>::take(&signer) {
                Some(balance) => balance,
                None => {
                    warn!("[sdot_claim]|The Ethereum address `{:?}` has no SDOT claims", signer);
                    return Err("The Ethereum address has no SDOT claims");
                }
            };

            let total = Self::total();
            ensure!(total >= balance, "Balance is less than the total amount of SDOT");
            <Total<T>>::mutate(|t| *t -= balance);

            deposit_token::<T>(&account_id, balance);

            update_binding::<T>(&account_id, signer, channel_name);

            // Let's deposit an event to let the outside world know this happened.
            Self::deposit_event(RawEvent::Claimed(sender, signer, balance));

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

// Recover Ethereum address from signature, return the Ethereum address.
fn recover_eth_address(
    signature: &EcdsaSignature,
    raw: &[u8],
    data: &[u8],
) -> Option<EthereumAddress> {
    if !contains(raw, data) {
        return None;
    }
    eth_recover(signature, raw)
}

fn contains(seq: &[u8], sub_seq: &[u8]) -> bool {
    seq.windows(sub_seq.len()).any(|window| window == sub_seq)
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

/// Try updating the binding address, remove pending deposit if the updating goes well.
/// return this account id and validator name
fn handle_input_data<T: Trait>(
    input: &[u8],
    addr_type: u8,
) -> Option<(T::AccountId, Option<Name>)> {
    T::AccountExtractor::account_info(input, addr_type)
}

fn deposit_token<T: Trait>(who: &T::AccountId, balance: T::Balance) {
    let token: xassets::Token = <Module<T> as xassets::ChainT>::TOKEN.to_vec();
    let _ = <xrecords::Module<T>>::deposit(&who, &token, balance).map_err(|e| {
        error!(
            "call xrecords to deposit error!, must use root to fix this error. reason:{:?}",
            e
        );
        e
    });
}

/// bind account
fn update_binding<T: Trait>(
    who: &T::AccountId,
    input_addr: EthereumAddress,
    channel_name: Option<Name>,
) {
    T::CrossChainProvider::update_binding(who, input_addr, channel_name)
}
