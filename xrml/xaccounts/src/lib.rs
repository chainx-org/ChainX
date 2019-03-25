// Copyright 2019 Chainpool.

//! this module is for chainx accounts

#![cfg_attr(not(feature = "std"), no_std)]

mod tests;
mod types;
use substrate_primitives::crypto::UncheckedFrom;

use primitives::traits::Hash;
use rstd::prelude::*;
use support::dispatch::Result;
use support::{decl_event, decl_module, decl_storage, StorageMap};
use xassets::Chain;
use xr_primitives::XString;

pub use self::types::{
    IntentionProps, TrusteeEntity, TrusteeInfoConfig, TrusteeIntentionProps, TrusteeSessionInfo,
};

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
    T::AccountId: UncheckedFrom<T::Hash>,
{
    fn accountid_for(origin: &T::AccountId) -> T::AccountId {
        let name = Module::<T>::intention_name_of(origin)
            .expect("The original account must be an existing intention.");
        // name
        UncheckedFrom::unchecked_from(T::Hashing::hash(&name))
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event<T>() = default;
    }
}

decl_event!(
    pub enum Event<T> where <T as system::Trait>::AccountId {
        /// New Trustees for chain, chain, session number, accountid, hot_addr, cold_addr
        NewTrustees(Chain, u32, Vec<AccountId>, Vec<u8>, Vec<u8>),
    }
);

decl_storage! {
    trait Store for Module<T: Trait> as XAccounts {
        /// intention name => intention
        pub IntentionOf get(intention_of): map Name => Option<T::AccountId>;

        /// intention => intention name
        pub IntentionNameOf get(intention_name_of): map T::AccountId => Option<Name>;

        pub IntentionPropertiesOf get(intention_props_of): map T::AccountId => IntentionProps<T::SessionKey>;

        /// account deposit addr(chain, addr bytes) => (accountid, option(channel accountid))  (channel is a validator)
        pub CrossChainAddressMapOf get(address_map): map (Chain, Vec<u8>) => Option<(T::AccountId, Option<T::AccountId>)>;
        /// account deposit accountid, chain => multi deposit addr
        pub CrossChainBindOf get(account_map): map (Chain, T::AccountId) => Vec<Vec<u8>>;

        /// when generate trustee, auto generate a new session number, increase the newest trustee addr, can't modify by user
        pub TrusteeSessionInfoLen get(trustee_session_info_len): map Chain => u32;
        /// all session trustee addr
        pub TrusteeSessionInfoOf get(trustee_session_info_of): map (Chain, u32) => Option<TrusteeSessionInfo<T::AccountId>>;
        /// trustee basal info config
        pub TrusteeInfoConfigOf get(trustee_info_config) config(): map Chain => TrusteeInfoConfig;
        /// trustee property of a accountid and chain
        pub TrusteeIntentionPropertiesOf get(trustee_intention_props_of): map (T::AccountId, Chain) => Option<TrusteeIntentionProps>;
    }
}

impl<T: Trait> Module<T> {
    #[inline]
    pub fn current_session_number(chain: Chain) -> u32 {
        match Self::trustee_session_info_len(chain).checked_sub(1) {
            Some(r) => r,
            None => u32::max_value(),
        }
    }

    pub fn trustee_session_info(chain: Chain) -> Option<TrusteeSessionInfo<T::AccountId>> {
        let current_session = Self::current_session_number(chain);
        Self::trustee_session_info_of((chain, current_session))
    }

    pub fn trustee_address(chain: Chain) -> Option<(Vec<u8>, Vec<u8>)> {
        Self::trustee_session_info(chain).map(|info| (info.hot_address, info.cold_address))
    }

    pub fn trustee_list(chain: Chain) -> Option<Vec<T::AccountId>> {
        Self::trustee_session_info(chain).map(|info| info.trustee_list)
    }

    pub fn trustee_address_of(chain: Chain, session_number: u32) -> Option<(Vec<u8>, Vec<u8>)> {
        Self::trustee_session_info_of((chain, session_number))
            .map(|info| (info.hot_address, info.cold_address))
    }

    pub fn new_trustee_session(
        chain: Chain,
        trustee_list: Vec<T::AccountId>,
        hot_address: Vec<u8>,
        cold_address: Vec<u8>,
    ) {
        let session_number = TrusteeSessionInfoLen::<T>::get(chain);
        TrusteeSessionInfoOf::<T>::insert(
            (chain, session_number),
            TrusteeSessionInfo::<T::AccountId> {
                trustee_list: trustee_list.clone(),
                hot_address: hot_address.clone(),
                cold_address: cold_address.clone(),
            },
        );

        let number = match session_number.checked_add(1) {
            Some(n) => n,
            None => 0_u32,
        };
        TrusteeSessionInfoLen::<T>::insert(chain, number);

        Self::deposit_event(RawEvent::NewTrustees(
            chain,
            session_number,
            trustee_list,
            hot_address,
            cold_address,
        ));
    }
}

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
