// Copyright 2018-2019 Chainpool.
//! this module is for bridge common parts
//! define trait and type for
//! `trustees`, `crosschain binding` and something others

#![cfg_attr(not(feature = "std"), no_std)]

pub mod extractor;
pub mod traits;
pub mod trustees;
pub mod types;
pub mod utils;

use frame_support::{
    decl_error, decl_event, decl_module, decl_storage,
    dispatch::{DispatchError, DispatchResult},
};
use frame_system::{self as system, ensure_root, ensure_signed};
use sp_std::{convert::TryFrom, prelude::*, result};

use chainx_primitives::{AssetId, Name, Text};
use xpallet_assets::Chain;
use xpallet_support::{error, info, traits::MultiSig};

use crate::traits::{ChannelBinding, TrusteeForChain};
use crate::trustees::{ChainContext, TrusteeMultisigProvider};
use crate::types::{
    GenericTrusteeIntentionProps, GenericTrusteeSessionInfo, TrusteeInfoConfig,
    TrusteeIntentionProps,
};

pub trait Trait: system::Trait {
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
    // for chain
    type BitcoinTrustee: TrusteeForChain<
        Self::AccountId,
        trustees::bitcoin::BTCTrusteeType,
        trustees::bitcoin::BTCTrusteeAddrInfo,
    >;
}

decl_event!(
    pub enum Event<T> where
        <T as system::Trait>::AccountId,
        GenericTrusteeSessionInfo = GenericTrusteeSessionInfo<<T as system::Trait>::AccountId> {
        SetTrusteeProps(AccountId, Chain, GenericTrusteeIntentionProps),
        NewTrustees(Chain, u32, GenericTrusteeSessionInfo),
        ChannelBinding(AssetId, AccountId, AccountId),
    }
);

decl_error! {
    /// Error for the This Module
    pub enum Error for Module<T: Trait> {
        ///
        InvalidGenericData,
        ///
        InvalidTrusteeSession,
        ///
        InvalidAboutLen,
        ///
        InvalidMultisig,
        ///
        NotSupportedForTrustee,
        /// existing duplicate account
        DuplicatedAccountId,
        /// not registered as trustee
        NotRegistered,
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event() = default;

        // trustees
        #[weight = 0]
        pub fn setup_trustee(origin, chain: Chain, about: Text, hot_entity: Vec<u8>, cold_entity: Vec<u8>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            Self::setup_trustee_impl(who, chain, about, hot_entity, cold_entity)
        }

        /// use for trustee multisig addr
        #[weight = 0]
        pub fn transition_trustee_session(origin, chain: Chain, new_trustees: Vec<T::AccountId>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            // judge current addr
            let _c = ChainContext::<T>::new(chain);
            if !TrusteeMultisigProvider::<T, ChainContext::<T>>::check_multisig(&who) {
                Err(Error::<T>::InvalidMultisig)?;
            }
            Self::transition_trustee_session_impl(chain, new_trustees)
        }

        #[weight = 0]
        pub fn transition_trustee_session_by_root(origin, chain: Chain, new_trustees: Vec<T::AccountId>) -> DispatchResult {
            ensure_root(origin)?;
            info!("[transition_trustee_session_by_root]|try to transition trustee|chain:{:?}|new_trustees:{:?}", chain, new_trustees);
            Self::transition_trustee_session_impl(chain, new_trustees)
        }

        #[weight = 0]
        pub fn set_trustee_info_config(origin, chain: Chain, config: TrusteeInfoConfig) -> DispatchResult {
            ensure_root(origin)?;
            TrusteeInfoConfigOf::insert(chain, config);
            Ok(())
        }

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
        // for trustee
        pub TrusteeMultiSigAddr get(fn trustee_multisig_addr): map hasher(twox_64_concat) Chain => T::AccountId;

        /// trustee basal info config
        pub TrusteeInfoConfigOf get(fn trustee_info_config) config(): map hasher(twox_64_concat) Chain => TrusteeInfoConfig;
        /// when generate trustee, auto generate a new session number, increase the newest trustee addr, can't modify by user
        pub TrusteeSessionInfoLen get(fn trustee_session_info_len): map hasher(twox_64_concat) Chain => u32 = 0;

        pub TrusteeSessionInfoOf get(fn trustee_session_info_of):
            double_map hasher(twox_64_concat) Chain, hasher(twox_64_concat) u32 => Option<GenericTrusteeSessionInfo<T::AccountId>>;

        pub TrusteeIntentionPropertiesOf get(fn trustee_intention_props_of):
            double_map hasher(blake2_128_concat) T::AccountId, hasher(twox_64_concat) Chain => Option<GenericTrusteeIntentionProps>;

        pub ChannelBindingOf get(fn channel_binding_of):
            double_map hasher(blake2_128_concat) T::AccountId, hasher(twox_64_concat) AssetId => Option<T::AccountId>;

        TmpChain get(fn tmp_chain): Chain;
    }
}

pub fn is_valid_about<T: Trait>(about: &[u8]) -> DispatchResult {
    // TODO
    if about.len() > 128 {
        Err(Error::<T>::InvalidAboutLen)?;
    }

    xpallet_support::xss_check(about)
}

impl<T: Trait> Module<T> {
    pub fn setup_trustee_impl(
        who: T::AccountId,
        chain: Chain,
        about: Text,
        hot_entity: Vec<u8>,
        cold_entity: Vec<u8>,
    ) -> DispatchResult {
        // todo validate is intention
        // ensure!(
        //     xaccounts::Module::<T>::is_intention(&who),
        //     "Transactor is not an intention."
        // );
        is_valid_about::<T>(&about)?;

        let (hot, cold) = match chain {
            Chain::Bitcoin => {
                let hot = T::BitcoinTrustee::check_trustee_entity(&hot_entity)?;
                let cold = T::BitcoinTrustee::check_trustee_entity(&cold_entity)?;
                (hot.into(), cold.into())
            }
            _ => Err(Error::<T>::NotSupportedForTrustee)?,
        };

        let props = GenericTrusteeIntentionProps(TrusteeIntentionProps::<Vec<u8>> {
            about,
            hot_entity: hot,
            cold_entity: cold,
        });

        TrusteeIntentionPropertiesOf::<T>::insert(&who, chain, props.clone());
        Self::deposit_event(RawEvent::SetTrusteeProps(who, chain, props));
        Ok(())
    }

    pub fn try_generate_session_info(
        chain: Chain,
        new_trustees: Vec<T::AccountId>,
    ) -> result::Result<GenericTrusteeSessionInfo<T::AccountId>, DispatchError> {
        let config = Self::trustee_info_config(chain);
        let has_duplicate =
            (1..new_trustees.len()).any(|i| new_trustees[i..].contains(&new_trustees[i - 1]));
        if has_duplicate {
            error!(
                "[try_generate_session_info]|existing duplicate account|candidates:{:?}",
                new_trustees
            );
            Err(Error::<T>::DuplicatedAccountId)?;
        }
        let mut props = vec![];
        for accountid in new_trustees.into_iter() {
            let p = Self::trustee_intention_props_of(&accountid, chain).ok_or_else(|| {
                error!("[transition_trustee_session]|not all candidate has registered as a trustee yet|who:{:?}",  accountid);
                Error::<T>::NotRegistered
            })?;
            props.push((accountid, p));
        }
        let info = match chain {
            Chain::Bitcoin => {
                let props = props
                    .into_iter()
                    .map(|(id, prop)| {
                        (
                            id,
                            TrusteeIntentionProps::<_>::try_from(prop)
                                .expect("must decode succss from storage data"),
                        )
                    })
                    .collect();
                let session_info = T::BitcoinTrustee::generate_trustee_session_info(props, config)?;
                session_info.into()
            }
            _ => Err(Error::<T>::NotSupportedForTrustee)?,
        };
        Ok(info)
    }

    fn transition_trustee_session_impl(
        chain: Chain,
        new_trustees: Vec<T::AccountId>,
    ) -> DispatchResult {
        let info = Self::try_generate_session_info(chain, new_trustees)?;

        let session_number = Self::trustee_session_info_len(chain);
        let next_number = match session_number.checked_add(1) {
            Some(n) => n,
            None => 0_u32,
        };
        TrusteeSessionInfoLen::insert(chain, next_number);
        TrusteeSessionInfoOf::<T>::insert(chain, session_number, info);

        // TODO generic new multisig addr
        // Self::deploy_trustee_addr_unsafe(chain, trustees);
        // TrusteeMultiSigAddr::<T>::insert(chain, addr)
        Ok(())
    }

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
