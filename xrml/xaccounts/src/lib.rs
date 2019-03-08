// Copyright 2019 Chainpool.

//! this module is for chainx accounts

#![cfg_attr(not(feature = "std"), no_std)]

use parity_codec_derive::{Decode, Encode};
#[cfg(feature = "std")]
use serde_derive::{Deserialize, Serialize};

use primitives::traits::Hash;
use rstd::prelude::*;
use support::dispatch::Result;
use support::{decl_event, decl_module, decl_storage, StorageMap};
use xassets::Chain;
use xr_primitives::XString;

mod tests;

pub type Name = XString;
pub type URL = XString;

pub trait Trait: system::Trait + consensus::Trait {
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;

    /// Generate virtual AccountId for each (psedu) intention
    type DetermineIntentionJackpotAccountId: IntentionJackpotAccountIdFor<Self::AccountId>;
}

pub trait IntentionJackpotAccountIdFor<AccountId: Sized> {
    fn accountid_for(origin: &AccountId) -> AccountId;
}

pub struct SimpleAccountIdDeterminator<T: Trait>(::rstd::marker::PhantomData<T>);

impl<T: Trait> IntentionJackpotAccountIdFor<T::AccountId> for SimpleAccountIdDeterminator<T>
where
    T::AccountId: From<T::Hash> + AsRef<[u8]>,
{
    fn accountid_for(origin: &T::AccountId) -> T::AccountId {
        let name = Module::<T>::intention_name_of(origin)
            .expect("The original account must be an existing intention.");
        // name
        T::Hashing::hash(&name).into()
    }
}

/// Intention mutable properties
#[derive(PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct IntentionProps<SessionKey> {
    pub url: URL,
    pub is_active: bool,
    pub about: XString,
    pub session_key: Option<SessionKey>,
}

// TrusteeEntity could be a pubkey or an address depending on the different chain.
#[derive(PartialEq, PartialOrd, Ord, Eq, Clone, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub enum TrusteeEntity {
    Bitcoin(Vec<u8>),
}

impl Default for TrusteeEntity {
    fn default() -> Self {
        TrusteeEntity::Bitcoin(Vec::default())
    }
}

#[derive(PartialEq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct TrusteeIntentionProps {
    pub about: XString,
    pub hot_entity: TrusteeEntity,
    pub cold_entity: TrusteeEntity,
}

#[derive(PartialEq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct TrusteeAddressPair {
    pub hot_address: Vec<u8>,
    pub cold_address: Vec<u8>,
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
        /// intention name => intention
        pub IntentionOf get(intention_of): map Name => Option<T::AccountId>;

        /// intention => intention name
        pub IntentionNameOf get(intention_name_of): map T::AccountId => Option<Name>;

        pub IntentionPropertiesOf get(intention_props_of): map T::AccountId => IntentionProps<T::SessionKey>;

        pub TrusteeIntentions get(trustee_intentions): Vec<T::AccountId>;

        pub TrusteeIntentionPropertiesOf get(trustee_intention_props_of): map (T::AccountId, Chain) => Option<TrusteeIntentionProps>;
        /// account deposit addr(chain, addr bytes) => (accountid, option(channel accountid))  (channel is a validator)
        pub CrossChainAddressMapOf get(address_map): map (Chain, Vec<u8>) => Option<(T::AccountId, Option<T::AccountId>)>;
        /// account deposit accountid, chain => multi deposit addr
        pub CrossChainBindOf get(account_map): map (Chain, T::AccountId) => Vec<Vec<u8>>;

        pub TrusteeAddress get(trustee_address): map Chain => Option<TrusteeAddressPair>;
    }
}

impl<T: Trait> Module<T> {}

impl<T: Trait> xsystem::Validator<T::AccountId> for Module<T> {
    fn get_validator_by_name(name: &[u8]) -> Option<T::AccountId> {
        Self::intention_of(name.to_vec())
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

    if url.iter().filter(|n| !is_valid(n)).count() > 0 {
        return Err("Only numbers, letters and . are allowed.");
    }
    Ok(())
}

/// Actually update the binding address of original transactor.
pub fn apply_update_binding<T: Trait>(
    who: T::AccountId,
    address_info: (Chain, Vec<u8>),
    channel_name: Vec<u8>,
) {
    let chain = address_info.0;
    if let Some((accountid, _)) = Module::<T>::address_map(&address_info) {
        if accountid != who {
            // old accountid is not equal to new accountid, means should change this addr bind to new account
            // remove this addr for old accounid's CrossChainBindOf
            CrossChainBindOf::<T>::mutate(&(chain, accountid), |addr_list| {
                addr_list.retain(|addr| addr != &address_info.1); // remove addr for this accountid bind
            });
        }
    }
    // insert or override binding relationship
    CrossChainBindOf::<T>::mutate(&(chain, who.clone()), |addr_list| {
        if !addr_list.contains(&address_info.1) {
            addr_list.push(address_info.1.clone());
        }
    });
    let channel_accountid = <IntentionOf<T>>::get(channel_name);
    CrossChainAddressMapOf::<T>::insert(&address_info, (who.clone(), channel_accountid));
}
