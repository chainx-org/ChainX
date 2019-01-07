// Copyright 2018 Chainpool.

//! this module is for chainx accounts

#![cfg_attr(not(feature = "std"), no_std)]

#[macro_use]
extern crate parity_codec_derive;
extern crate parity_codec as codec;

// for substrate
#[cfg(feature = "std")]
extern crate substrate_primitives;

// for substrate runtime
// map!, vec! marco.
extern crate sr_std as rstd;

#[cfg(feature = "std")]
extern crate sr_io as runtime_io;
extern crate sr_primitives as runtime_primitives;
// for substrate runtime module lib
// Needed for type-safe access to storage DB.
#[macro_use]
extern crate srml_support as runtime_support;
#[cfg(test)]
extern crate srml_balances as balances;
#[cfg(test)]
extern crate srml_consensus as consensus;
#[cfg(test)]
extern crate srml_session as session;
extern crate srml_system as system;
#[cfg(test)]
extern crate srml_timestamp as timestamp;

use rstd::prelude::*;
use runtime_support::dispatch::Result;
use runtime_support::{StorageMap, StorageValue};

mod tests;

pub trait Trait: system::Trait {
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

/// Cert immutable properties
#[derive(PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub struct CertImmutableProps<BlockNumber: Default> {
    pub issued_at: BlockNumber,
    pub frozen_duration: u32,
}

/// Intention Immutable properties
#[derive(PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub struct IntentionImmutableProps {
    pub name: Vec<u8>,
    pub activator: Vec<u8>,
    pub initial_shares: u32,
}

/// Intention mutable properties
#[derive(PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub struct IntentionProps {
    pub url: Vec<u8>,
    pub is_active: bool,
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event() = default;
    }
}

/// An event in this module.
decl_event!(
    pub enum Event<T> where <T as system::Trait>::AccountId {
        /// A cert has been issued.
        Issue(Vec<u8>, u32, AccountId),
    }
);

decl_storage! {
    trait Store for Module<T: Trait> as XAccounts {
        /// Shares per cert.
        pub SharesPerCert get(shares_per_cert) config(): u32;

        pub ActivationPerShare get(activation_per_share) config(): u32;

        pub MaximumCertCount get(maximum_cert_count) config(): u32;

        pub TotalIssued get(total_issued) config(): u32;

        /// cert name => cert owner
        pub CertOwnerOf get(cert_owner_of): map Vec<u8> => Option<T::AccountId>;

        pub CertImmutablePropertiesOf get(cert_immutable_props_of): map Vec<u8> => CertImmutableProps<T::BlockNumber>;

        pub RemainingSharesOf get(remaining_shares_of): map Vec<u8> => u32;

        pub CertNamesOf get(cert_names_of): map T::AccountId => Vec<Vec<u8>>;

        /// intention name => intention
        pub IntentionOf get(intention_of): map Vec<u8> => Option<T::AccountId>;

        pub IntentionImmutablePropertiesOf get(intention_immutable_props_of): map T::AccountId => Option<IntentionImmutableProps>;

        pub IntentionPropertiesOf get(intention_props_of): map T::AccountId => IntentionProps;
    }
}

impl<T: Trait> Module<T> {
    /// Issue new cert triggered by relayed transaction.
    pub fn issue(cert_name: Vec<u8>, frozen_duration: u32, cert_owner: T::AccountId) -> Result {
        is_valid_name::<T>(&cert_name)?;

        ensure!(
            Self::cert_owner_of(&cert_name).is_none(),
            "Cannot issue if this cert name already exists."
        );

        ensure!(
            Self::total_issued() < Self::maximum_cert_count(),
            "Cannot issue when there are too many certs."
        );

        ensure!(
            frozen_duration <= 365,
            "Cannot issue if frozen duration out of range."
        );

        <CertOwnerOf<T>>::insert(&cert_name, cert_owner.clone());

        <CertImmutablePropertiesOf<T>>::mutate(&cert_name, |cert| {
            cert.issued_at = <system::Module<T>>::block_number();
            cert.frozen_duration = frozen_duration;
        });

        <RemainingSharesOf<T>>::insert(&cert_name, Self::shares_per_cert());

        <CertNamesOf<T>>::mutate(&cert_owner, |names| names.push(cert_name.clone()));
        <TotalIssued<T>>::put(Self::total_issued() + 1);

        Self::deposit_event(RawEvent::Issue(cert_name, frozen_duration, cert_owner));

        Ok(())
    }
}

pub fn is_valid_name<T: Trait>(name: &[u8]) -> Result {
    if name.len() > 12 || name.len() < 2 {
        return Err("The length of name must be in range [2, 12].");
    }

    Ok(())
}

pub fn is_valid_url<T: Trait>(url: &[u8]) -> Result {
    if url.len() > 24 || url.len() < 4 {
        return Err("The length of url must be in range [4, 24].");
    }
    let is_valid = |n: &u8| -> bool {
        // number, capital letter, lowercase letter, .
        if *n >= 0x30 && *n <= 0x39
            || *n >= 0x41 && *n <= 0x5A
            || *n >= 0x61 && *n <= 0x7A
            || *n == 0x2E
        {
            return true;
        }
        return false;
    };

    if url
        .into_iter()
        .filter(|n| !is_valid(n))
        .collect::<Vec<_>>()
        .len()
        > 0
    {
        return Err("Only numbers, letters and . are allowed.");
    }
    Ok(())
}
