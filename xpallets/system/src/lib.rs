// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

#![cfg_attr(not(feature = "std"), no_std)]

pub use self::pallet::*;

const PALLET_MARK: &[u8; 1] = b"#";
const ALWAYS_ALLOW: [&str; 1] = ["Sudo"];

/// The module's config trait.
///
/// `frame_system::Config` should always be included in our implied traits.
#[frame_support::pallet]
pub mod pallet {
    use sp_std::{collections::btree_map::BTreeMap, marker::PhantomData, vec::Vec};

    #[cfg(feature = "std")]
    use frame_support::traits::GenesisBuild;
    use frame_support::{
        dispatch::{CallMetadata, DispatchResultWithPostInfo},
        pallet_prelude::{StorageMap, StorageValue, ValueQuery},
        traits::{Currency, IsType},
        Blake2_128Concat, Twox64Concat,
    };
    use frame_system::pallet_prelude::{ensure_root, OriginFor};

    use sp_runtime::traits::StaticLookup;

    use xp_protocol::NetworkType;

    use super::*;

    #[pallet::pallet]
    #[pallet::generate_store(pub(crate) trait Store)]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching event type.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// The currency mechanism.
        type Currency: Currency<Self::AccountId>;
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Modify the paused status of the given pallet call.
        ///
        /// This is a root-only operation.
        #[pallet::weight(0)]
        pub fn modify_paused(
            origin: OriginFor<T>,
            pallet: Vec<u8>,
            call: Option<Vec<u8>>,
            should_paused: bool,
        ) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;

            let mut paused = Self::paused(&pallet);

            if should_paused {
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
                Paused::<T>::remove(&pallet);
            } else {
                Paused::<T>::insert(pallet, paused);
            }
            Ok(().into())
        }

        /// Toggle the blacklist status of the given account id.
        ///
        /// This is a root-only operation.
        #[pallet::weight(0)]
        fn toggle_blacklist(
            origin: OriginFor<T>,
            who: <T::Lookup as StaticLookup>::Source,
            should_blacklist: bool,
        ) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;

            let who = T::Lookup::lookup(who)?;
            if should_blacklist {
                Blacklist::<T>::insert(who.clone(), true);
                Self::deposit_event(Event::<T>::Blacklisted(who))
            } else {
                Blacklist::<T>::remove(&who);
                Self::deposit_event(Event::<T>::Unblacklisted(who));
            }
            Ok(().into())
        }
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(crate) fn deposit_event)]
    #[pallet::metadata(T::AccountId = "AccountId")]
    /// Event for the XSystem Module
    pub enum Event<T: Config> {
        /// An account was added to the blacklist. [who]
        Blacklisted(T::AccountId),
        /// An account was removed from the blacklist. [who]
        Unblacklisted(T::AccountId),
    }

    #[pallet::storage]
    #[pallet::getter(fn network_props)]
    /// Network property (Mainnet / Testnet).
    pub type NetworkProps<T> = StorageValue<_, NetworkType, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn paused)]
    /// Paused pallet call.
    pub type Paused<T> = StorageMap<_, Twox64Concat, Vec<u8>, BTreeMap<Vec<u8>, ()>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn blacklist)]
    /// The accounts that are blocked.
    pub type Blacklist<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, bool, ValueQuery>;

    #[pallet::genesis_config]
    pub struct GenesisConfig {
        pub network_props: NetworkType,
    }

    #[cfg(feature = "std")]
    impl Default for GenesisConfig {
        fn default() -> Self {
            Self {
                network_props: Default::default(),
            }
        }
    }

    #[pallet::genesis_build]
    #[cfg(feature = "std")]
    impl<T: Config> GenesisBuild<T> for GenesisConfig {
        fn build(&self) {
            NetworkProps::<T>::put(self.network_props);
        }
    }

    impl<T: Config> Pallet<T> {
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
        pub fn get_blacklist() -> Vec<T::AccountId> {
            Blacklist::<T>::iter()
                .filter_map(|(account, blocked)| if blocked { Some(account) } else { None })
                .collect()
        }
    }
}
