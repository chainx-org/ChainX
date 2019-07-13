// Copyright 2018-2019 Chainpool.
//! this module is for bridge common parts
//! define trait and type for
//! `trustees`, `crosschain binding` and something others

#![cfg_attr(not(feature = "std"), no_std)]

pub mod extractor;
pub mod traits;
pub mod types;
pub mod utils;

mod trustees;

use primitives::traits::{As, CheckedSub};
use support::{decl_event, decl_module, decl_storage, StorageMap};

use xassets::TokenJackpotAccountIdFor;
use xr_primitives::{Name, Token};
use xsupport::{debug, error, warn};
#[cfg(feature = "std")]
use xsupport::{token, try_hex_or_str};

use crate::traits::CrossChainBindingV2;

pub trait Trait: system::Trait + xaccounts::Trait + xassets::Trait + xfee_manager::Trait {
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

decl_event!(
    pub enum Event<T> where
        <T as system::Trait>::AccountId {
        ChannelBinding(Token, AccountId, AccountId),
    }
);

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event<T>() = default;
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as XBridgeCommon {
        pub CrossChainBinding get(crosschain_binding): map (Token, T::AccountId) => Option<T::AccountId>;
    }
}

impl<T: Trait> Module<T> {
    pub fn reward_from_jackpot(token: &Token, who: &T::AccountId, value: T::Balance) {
        if let Some(addr) = T::DetermineTokenJackpotAccountId::accountid_for_safe(token) {
            let now = xassets::Module::<T>::pcx_free_balance(&addr);
            if let None = now.checked_sub(&value) {
                return;
            }

            if let Err(_e) = xassets::Module::<T>::pcx_move_free_balance(&addr, who, value) {
                error!(
                    "[reward_from_jackpot]|reward from jackpot err|e:{:}",
                    _e.info()
                );
            }
        } else {
            error!(
                "[reward_from_jackpot]|this token do not have jackpot|token:{:}",
                token!(token)
            );
        }
    }

    pub fn reward_relayer(token: &Token, who: &T::AccountId, power: u64, tx_len: u64) {
        // todo may use a storage to adjust `Acceleration`
        let acc = 1_u64;
        // version(1) + addr + sig + index(compact) + era(relay usually use `Immortal`) + acc(compact)
        let extrinsic_len = 1 + (1 + 32) + 64 + 1 + 1 + 1;
        // module + call + tx_source
        let func_len = 1 + 1 + tx_len;

        let len = extrinsic_len + func_len;

        let value = xfee_manager::Module::<T>::transaction_fee(power, len) * As::sa(acc);

        Self::reward_from_jackpot(token, who, value);
        debug!(
            "[reward_relayer]|token:{:}|relayer:{:?}|back fee:{:}",
            token!(token),
            who,
            value
        );
    }
}

impl<T: Trait> CrossChainBindingV2<T::AccountId> for Module<T> {
    fn update_binding(token: &Token, who: &T::AccountId, channel_name: Option<Name>) {
        if let Some(name) = channel_name {
            if let Some(channel) = xaccounts::Module::<T>::intention_of(&name) {
                match Self::get_binding_info(token, who) {
                    None => {
                        // set to storage
                        let key = (token.clone(), who.clone());
                        CrossChainBinding::<T>::insert(&key, channel.clone());

                        Self::deposit_event(RawEvent::ChannelBinding(
                            token.clone(),
                            who.clone(),
                            channel,
                        ));
                    }
                    Some(_channel) => {
                        debug!("[update_binding]|already has binding, do nothing|token:{:}|who:{:?}|channel:{:?}", token!(token), who, _channel);
                    }
                }
            } else {
                warn!(
                    "[update_binding]|channel not exist, do not set binding|name:{:?}",
                    try_hex_or_str(&name)
                );
            };
        };
    }
    fn get_binding_info(token: &Token, who: &T::AccountId) -> Option<T::AccountId> {
        Self::crosschain_binding(&(token.clone(), who.clone()))
    }
}
