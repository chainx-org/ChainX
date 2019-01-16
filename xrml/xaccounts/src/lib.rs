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
extern crate srml_timestamp as timestamp;

extern crate xr_primitives;

use rstd::prelude::*;
use runtime_support::dispatch::Result;
use runtime_support::{StorageMap, StorageValue};

use xr_primitives::XString;

mod tests;

pub type Name = XString;
pub type URL = XString;

pub trait Trait: system::Trait + timestamp::Trait {
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

/// Cert immutable properties
#[derive(PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct CertImmutableProps<BlockNumber: Default, Moment: Default> {
    pub issued_at: (BlockNumber, Moment),
    pub frozen_duration: u32,
}

/// Intention Immutable properties
#[derive(PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct IntentionImmutableProps<Moment> {
    pub name: Name,
    pub activator: Name,
    pub initial_shares: u32,
    pub registered_at: Moment,
}

/// Intention mutable properties
#[derive(PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct IntentionProps {
    pub url: URL,
    pub is_active: bool,
    pub about: XString,
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event<T>() = default;
    }
}

/// An event in this module.
decl_event!(
    pub enum Event<T> where <T as system::Trait>::AccountId {
        /// A cert has been issued.
        Issue(Name, u32, AccountId),
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
        pub CertOwnerOf get(cert_owner_of): map Name => Option<T::AccountId>;

        pub CertImmutablePropertiesOf get(cert_immutable_props_of): map Name => CertImmutableProps<T::BlockNumber, T::Moment>;

        pub RemainingSharesOf get(remaining_shares_of): map Name => u32;

        pub CertNamesOf get(cert_names_of): map T::AccountId => Vec<Name>;

        /// intention name => intention
        pub IntentionOf get(intention_of): map Name => Option<T::AccountId>;

        pub IntentionImmutablePropertiesOf get(intention_immutable_props_of): map T::AccountId => Option<IntentionImmutableProps<T::Moment>>;

        pub IntentionPropertiesOf get(intention_props_of): map T::AccountId => IntentionProps;

    }

    add_extra_genesis {
        config(cert_owner): T::AccountId;

        build(|storage: &mut runtime_primitives::StorageMap, _: &mut runtime_primitives::ChildrenStorageMap, config: &GenesisConfig<T>| {
            use runtime_io::with_externalities;
            use substrate_primitives::Blake2Hasher;
            use runtime_primitives::StorageMap;

            let s = storage.clone().build_storage().unwrap().0;
            let mut init: runtime_io::TestExternalities<Blake2Hasher> = s.into();
            with_externalities(&mut init, || {
                let cert_name = b"genesis_cert".to_vec();
                let frozen_duration = 1u32;
                let cert_owner = config.cert_owner.clone();
                Module::<T>::issue(cert_name, frozen_duration, cert_owner).unwrap();
            });
            let init: StorageMap = init.into();
            storage.extend(init);
        });
    }
}

impl<T: Trait> Module<T> {
    /// Issue new cert triggered by relayed transaction.
    pub fn issue(cert_name: Name, frozen_duration: u32, cert_owner: T::AccountId) -> Result {
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
            cert.issued_at = (
                <system::Module<T>>::block_number(),
                <timestamp::Module<T>>::now(),
            );
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

pub fn is_valid_about<T: Trait>(about: &[u8]) -> Result {
    if about.len() > 128 {
        return Err("The length of about must be in range [0, 128].");
    }

    Ok(())
}

pub fn is_valid_url<T: Trait>(url: &[u8]) -> Result {
    if url.len() > 24 || url.len() < 4 {
        return Err("The length of url must be in range [4, 24].");
    }
    // number, capital letter, lowercase letter, .
    let is_valid = |n: &u8| -> bool {
        *n >= 0x30 && *n <= 0x39
            || *n >= 0x41 && *n <= 0x5A
            || *n >= 0x61 && *n <= 0x7A
            || *n == 0x2E
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
