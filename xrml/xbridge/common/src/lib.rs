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
#[cfg(feature = "std")]
use xsupport::token;
use xsupport::{debug, error};

use crate::traits::CrossChainBindingV2;

pub trait Trait: system::Trait + xaccounts::Trait + xassets::Trait + xfee_manager::Trait {
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

decl_event!(
    pub enum Event<T> where
        <T as system::Trait>::AccountId {
        ChannelBinding(Token, AccountId, Option<AccountId>),
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
        // version(1) + addr + sig + index + era(relay usually use `Immortal`) + acc
        let extrinsic_len = 1 + (1 + 32) + 64 + 8 + 1 + 4;
        // module + call + accountid + tx_source
        let func_len = 1 + 1 + 32 + tx_len;

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
        let channel_accountid = channel_name
            .and_then(|name| xaccounts::Module::<T>::intention_of(name))
            .map(|accountid| {
                if Self::get_binding_info(token, who).is_none() {
                    // set to storage
                    let key = (token.clone(), who.clone());
                    CrossChainBinding::<T>::insert(&key, accountid.clone());
                }
                accountid
            });

        Self::deposit_event(RawEvent::ChannelBinding(
            token.clone(),
            who.clone(),
            channel_accountid,
        ));
    }
    fn get_binding_info(token: &Token, who: &T::AccountId) -> Option<T::AccountId> {
        Self::crosschain_binding(&(token.clone(), who.clone()))
    }
}
