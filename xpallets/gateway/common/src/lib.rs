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

use sp_std::{collections::btree_map::BTreeMap, convert::TryFrom, prelude::*, result};

use frame_support::{
    decl_error, decl_event, decl_module, decl_storage,
    dispatch::{DispatchError, DispatchResult},
    ensure, IterableStorageMap,
};
use frame_system::{self as system, ensure_root, ensure_signed};

use chainx_primitives::{AssetId, Name, Text};
use xpallet_assets::Chain;
use xpallet_support::{
    error, info,
    traits::{MultiSig, Validator},
};

use crate::traits::{ChannelBinding, TrusteeForChain};
use crate::trustees::{ChainContext, TrusteeMultisigProvider};
use crate::types::{
    GenericTrusteeIntentionProps, GenericTrusteeSessionInfo, TrusteeInfoConfig,
    TrusteeIntentionProps,
};

pub trait Trait: system::Trait + pallet_multisig::Trait {
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;

    type Validator: Validator<Self::AccountId>;
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
        /// just allow validator to register trustee
        NotValidator,
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event() = default;

        // trustees
        #[weight = 0]
        pub fn setup_trustee(origin, chain: Chain, about: Text, hot_entity: Vec<u8>, cold_entity: Vec<u8>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(T::Validator::is_validator(&who), Error::<T>::NotValidator);
            Self::setup_trustee_impl(who, chain, about, hot_entity, cold_entity)
        }

        /// use for trustee multisig addr
        #[weight = 0]
        pub fn transition_trustee_session(origin, chain: Chain, new_trustees: Vec<T::AccountId>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            // judge current addr
            let _c = ChainContext::new(chain);
            if !TrusteeMultisigProvider::<T, ChainContext>::check_multisig(&who) {
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
        pub TrusteeInfoConfigOf get(fn trustee_info_config): map hasher(twox_64_concat) Chain => TrusteeInfoConfig;
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
    add_extra_genesis {
        config(trustees): Vec<(Chain, TrusteeInfoConfig, Vec<(T::AccountId, Text, Vec<u8>, Vec<u8>)>)>;
        build(|config| {
            for (chain, info_config, trustee_infos) in config.trustees.iter() {
                let mut trustees = vec![];
                for (who, about, hot, cold) in trustee_infos.iter() {
                    Module::<T>::setup_trustee_impl(who.clone(), *chain, about.clone(), hot.clone(), cold.clone()).expect("must success");
                    trustees.push(who.clone());
                }
                // config set should before transitino
                TrusteeInfoConfigOf::insert(chain, info_config.clone());
                Module::<T>::transition_trustee_session_impl(*chain, trustees).expect("must success in genesis");
            }
        })
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
                let mut session_info =
                    T::BitcoinTrustee::generate_trustee_session_info(props, config)?;

                // sort account list to make sure generate a stable multisig addr(addr is related with accounts sequence)
                session_info.trustee_list.sort();
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

        let multi_addr =
            pallet_multisig::Module::<T>::multi_account_id(&info.trustee_list, info.threshold);

        TrusteeSessionInfoLen::insert(chain, next_number);
        TrusteeSessionInfoOf::<T>::insert(chain, session_number, info);

        TrusteeMultiSigAddr::<T>::insert(chain, multi_addr);
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

impl<T: Trait> Module<T> {
    pub fn trustee_multisigs() -> BTreeMap<Chain, T::AccountId> {
        TrusteeMultiSigAddr::<T>::iter().collect()
    }
}
