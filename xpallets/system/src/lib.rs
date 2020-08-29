#![cfg_attr(not(feature = "std"), no_std)]

use sp_runtime::traits::StaticLookup;
use sp_std::{collections::btree_map::BTreeMap, prelude::*};

use frame_support::{
    decl_error, decl_event, decl_module, decl_storage,
    dispatch::{CallMetadata, DispatchResult},
};
use frame_system::ensure_root;

use xpallet_protocol::NetworkType;

pub trait Trait: frame_system::Trait {
    /// Event
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
}

decl_error! {
    /// Error for the System Module
    pub enum Error for Module<T: Trait> {

    }
}

decl_event!(
    pub enum Event<T> where
        <T as frame_system::Trait>::AccountId
    {
        BlockAccount(AccountId),
        RevokeBlockedAccounts(AccountId),
    }
);

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        type Error = Error<T>;
        fn deposit_event() = default;

        #[weight = 0]
        pub fn modify_paused(origin, pallet: Vec<u8>, call: Option<Vec<u8>>, paused: bool) -> DispatchResult {
            ensure_root(origin)?;
            let mut sub_paused = Self::paused(&pallet);

            if paused {
                if let Some(c) = call {
                    sub_paused.insert(c, ());
                } else {
                    sub_paused.insert(PALLET_MARK.to_vec(), ());
                }
            } else {
                if let Some(c) = call {
                    sub_paused.remove(&c[..]);
                } else {
                    sub_paused.remove(&PALLET_MARK[..]);
                }
            }

            if sub_paused.is_empty() {
                Paused::remove(&pallet);
            } else {
                Paused::insert(pallet, sub_paused);
            }
            Ok(())
        }

        #[weight = 0]
        fn modify_blocked_list(origin, who: <T::Lookup as StaticLookup>::Source, block: bool) -> DispatchResult {
            ensure_root(origin)?;
            let who = T::Lookup::lookup(who)?;
            if block {
                BlockedAccounts::<T>::insert(who.clone(), ());
                Self::deposit_event(RawEvent::BlockAccount(who))
            } else {
                BlockedAccounts::<T>::remove(&who);
                Self::deposit_event(RawEvent::RevokeBlockedAccounts(who));
            }
            Ok(())
        }
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as XSystem {
        pub NetworkProps get(fn network_props) config(): NetworkType;

        pub Paused get(fn paused): map hasher(twox_64_concat) Vec<u8> => BTreeMap<Vec<u8>, ()>;

        pub BlockedAccounts get(fn blocked_accounts): map hasher(blake2_128_concat) T::AccountId => Option<()>;
    }
}

const ALWAYS_ALLOW: [&str; 1] = ["Sudo"];
const PALLET_MARK: &[u8; 1] = b"#";

impl<T: Trait> Module<T> {
    pub fn is_paused(metadata: CallMetadata) -> bool {
        if ALWAYS_ALLOW.contains(&metadata.pallet_name) {
            return false;
        }
        let p = Self::paused(metadata.pallet_name.as_bytes());
        // check whether this pallet has been paused
        if p.get(&PALLET_MARK[..]).is_some() {
            return true;
        }
        // check whether this pallet call has been paused
        if p.get(metadata.function_name.as_bytes()).is_some() {
            return true;
        }
        // no pause
        false
    }

    pub fn blocked_list() -> Vec<T::AccountId> {
        use frame_support::IterableStorageMap;
        BlockedAccounts::<T>::iter().map(|(a, _)| a).collect()
    }
}
