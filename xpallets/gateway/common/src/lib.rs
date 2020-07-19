// Copyright 2018-2019 Chainpool.
//! this module is for bridge common parts
//! define trait and type for
//! `trustees`, `crosschain binding` and something others

#![cfg_attr(not(feature = "std"), no_std)]

pub mod extractor;
pub mod traits;
pub mod types;
pub mod utils;

use frame_support::{decl_error, decl_event, decl_module, decl_storage, dispatch::DispatchResult};
use frame_system::{self as system, ensure_root};

// use xassets::AssetIdJackpotAccountIdFor;
// use xr_primitives::{Name, AssetId};
use chainx_primitives::{AssetId, Name};

use crate::traits::ChannelBinding;

pub trait Trait: system::Trait {
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

decl_event!(
    pub enum Event<T> where
        <T as system::Trait>::AccountId {
        ChannelBinding(AssetId, AccountId, AccountId),
    }
);

decl_error! {
    /// Error for the This Module
    pub enum Error for Module<T: Trait> {
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event() = default;

        #[weight = 0]
        pub fn force_set_binding(origin, #[compact] asset_id: AssetId, who: T::AccountId, binded: T::AccountId) -> DispatchResult {
            ensure_root(origin)?;
            Self::set_binding(asset_id, who, binded);
            Ok(())
        }
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as XGatewayCommon {
        pub ChannelBindingOf get(fn channel_binding_of):
            double_map hasher(blake2_128_concat) T::AccountId, hasher(twox_64_concat) AssetId => Option<T::AccountId>;
    }
}

impl<T: Trait> Module<T> {
    fn set_binding(asset_id: AssetId, who: T::AccountId, binded: T::AccountId) {
        ChannelBindingOf::<T>::insert(&who, &asset_id, binded.clone());

        Self::deposit_event(RawEvent::ChannelBinding(asset_id, who, binded))
    }
}

impl<T: Trait> ChannelBinding<T::AccountId> for Module<T> {
    fn update_binding(assert_id: &AssetId, who: &T::AccountId, channel_name: Option<Name>) {
        if let Some(name) = channel_name {
            // TODO relate name to an accountid
            // Self::set_binding(asset_id, who, binded);
            /*
            if let Some(channel) = xaccounts::Module::<T>::intention_of(&name) {
                match Self::get_binding_info(assert_id, who) {
                    None => {
                        // set to storage
                        let key = (assert_id.clone(), who.clone());
                        ChannelBindingOf::<T>::insert(&key, channel.clone());

                        Self::deposit_event(RawEvent::ChannelBinding(
                            assert_id.clone(),
                            who.clone(),
                            channel,
                        ));
                    }
                    Some(_channel) => {
                        debug!("[update_binding]|already has binding, do nothing|assert_id:{:}|who:{:?}|channel:{:?}", assert_id!(assert_id), who, _channel);
                    }
                }
            } else {
                warn!(
                    "[update_binding]|channel not exist, do not set binding|name:{:?}",
                    str!(&name)
                );
            };
            */
        };
    }
    fn get_binding_info(assert_id: &AssetId, who: &T::AccountId) -> Option<T::AccountId> {
        Self::channel_binding_of(who, assert_id)
    }
}
