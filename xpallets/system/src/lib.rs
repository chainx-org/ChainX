// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

#![cfg_attr(not(feature = "std"), no_std)]

use sp_std::{collections::btree_map::BTreeMap, prelude::*};

use frame_support::{
    decl_error, decl_event, decl_module, decl_storage,
    dispatch::{CallMetadata, DispatchResult},
    traits::Currency,
    IterableStorageMap,
};
use frame_system::ensure_root;
use sp_runtime::traits::StaticLookup;

use xpallet_protocol::NetworkType;

const PALLET_MARK: &[u8; 1] = b"#";
const ALWAYS_ALLOW: [&str; 1] = ["Sudo"];

type BalanceOf<T> =
    <<T as Trait>::Currency as Currency<<T as frame_system::Trait>::AccountId>>::Balance;

/// The module's config trait.
///
/// `frame_system::Trait` should always be included in our implied traits.
pub trait Trait: frame_system::Trait {
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

    /// The currency mechanism.
    type Currency: Currency<Self::AccountId>;
}

decl_error! {
    /// Error for the XSystem Module
    pub enum Error for Module<T: Trait> {}
}

decl_event!(
    /// Event for the XSystem Module
    pub enum Event<T>
    where
        Balance = BalanceOf<T>,
        <T as frame_system::Trait>::AccountId,
    {
        /// An account is blocked.
        BlockAccount(AccountId),
        /// The blocked status of an account is revoked.
        RevokeBlockedAccount(AccountId),
        /// Transaction fee is paid to the block author and its reward pot in 10:90.
        /// [author, author_fee, reward_pot, reward_pot_fee]
        TransactionFeePaid(AccountId, Balance, AccountId, Balance),
    }
);

decl_storage! {
    trait Store for Module<T: Trait> as XSystem {
        /// Network property (Mainnet / Testnet).
        pub NetworkProps get(fn network_props) config(): NetworkType;

        /// Paused pallet call.
        pub Paused get(fn paused): map hasher(twox_64_concat) Vec<u8> => BTreeMap<Vec<u8>, ()>;

        /// Blocked account ids.
        pub BlockedAccounts get(fn blocked_accounts): map hasher(blake2_128_concat) T::AccountId => Option<()>;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        type Error = Error<T>;

        fn deposit_event() = default;

        /// Modify the paused status of the given pallet call.
        ///
        /// This is a root-only operation.
        #[weight = 0]
        pub fn modify_paused(origin, pallet: Vec<u8>, call: Option<Vec<u8>>, is_paused: bool) -> DispatchResult {
            ensure_root(origin)?;

            let mut paused = Self::paused(&pallet);

            if is_paused {
                if let Some(c) = call {
                    // pause the call of the pallet
                    paused.insert(c, ());
                } else {
                    // pause the whole calls of the pallet
                    paused.insert(PALLET_MARK.to_vec(), ());
                }
            } else {
                if let Some(c) = call {
                    // revoke the paused status of the call in the pallet
                    paused.remove(&c[..]);
                } else {
                    // revoke the paused status of the whole calls in the pallet.
                    paused.remove(&PALLET_MARK[..]);
                }
            }

            if paused.is_empty() {
                Paused::remove(&pallet);
            } else {
                Paused::insert(pallet, paused);
            }
            Ok(())
        }

        /// Modify the blocked status of the given account id.
        ///
        /// This is a root-only operation.
        #[weight = 0]
        fn modify_blocked_list(origin, who: <T::Lookup as StaticLookup>::Source, is_block: bool) -> DispatchResult {
            ensure_root(origin)?;

            let who = T::Lookup::lookup(who)?;
            if is_block {
                BlockedAccounts::<T>::insert(who.clone(), ());
                Self::deposit_event(Event::<T>::BlockAccount(who))
            } else {
                BlockedAccounts::<T>::remove(&who);
                Self::deposit_event(Event::<T>::RevokeBlockedAccount(who));
            }
            Ok(())
        }
    }
}

impl<T: Trait> Module<T> {
    /// Returns true if the given pallet call has been paused.
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

    /// Returns the blocked account id list.
    pub fn blocked_list() -> Vec<T::AccountId> {
        BlockedAccounts::<T>::iter().map(|(a, _)| a).collect()
    }
}
