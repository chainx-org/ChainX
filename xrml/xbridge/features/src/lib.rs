// Copyright 2018-2019 Chainpool.
//! this module is for bridge features
//! features know bridge-common and all bridge info(depends on bridge-common and  all spv bridge module)
//! get types from all bridge and impl trait for provider
//! include `trustees`, `crosschain binding` and others...
//! bridge-common(definition)
//!      |            \
//! bridge-bitcoin  bridge-ethereum (define trait type to accept data provider)
//!      |          /
//! bridge-features(implementation, data provider)

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod tests;

pub mod crosschain_binding;
pub mod trustees;

use rstd::collections::btree_map::BTreeMap;
use rstd::{marker::PhantomData, prelude::*, result};

use parity_codec::Encode;

use primitives::traits::Hash;
use substrate_primitives::crypto::UncheckedFrom;
use support::{decl_event, decl_module, decl_storage, dispatch::Result, ensure, StorageMap};
use system::ensure_signed;

// chainx runtime module
use xassets::Chain;
use xr_primitives::XString;
use xsupport::{error, info};

use xbridge_common::{
    traits::{TrusteeForChain, TrusteeMultiSig, TrusteeSession},
    types::{
        into_generic_all_info, GenericAllSessionInfo, GenericTrusteeIntentionProps,
        TrusteeInfoConfig,
    },
    utils::two_thirds_unsafe,
};

// bitcoin
pub use xbitcoin::H264;

pub use trustees::{
    BitcoinTrusteeAddrInfo, BitcoinTrusteeIntentionProps, BitcoinTrusteeMultiSig,
    BitcoinTrusteeSessionInfo,
};

pub use crosschain_binding::{BitcoinAddress, EthereumAddress};

pub trait TrusteeMultiSigFor<AccountId: Sized> {
    fn multi_sig_addr_for_trustees(chain: Chain, trustees: &Vec<AccountId>) -> AccountId;
}
/// Simple MultiSigIdFor struct
pub struct SimpleTrusteeMultiSigIdFor<T: Trait>(PhantomData<T>);
impl<T: Trait> TrusteeMultiSigFor<T::AccountId> for SimpleTrusteeMultiSigIdFor<T>
where
    T::AccountId: UncheckedFrom<T::Hash> + AsRef<[u8]>,
{
    fn multi_sig_addr_for_trustees(chain: Chain, trustees: &Vec<T::AccountId>) -> T::AccountId {
        let mut buf = Vec::<u8>::new();
        buf.extend_from_slice(&chain.encode());
        for trustee in trustees {
            buf.extend_from_slice(trustee.as_ref());
        }
        UncheckedFrom::unchecked_from(T::Hashing::hash(&buf[..]))
    }
}

pub trait Trait: system::Trait + xmultisig::Trait + xbitcoin::Trait {
    type TrusteeMultiSig: TrusteeMultiSigFor<Self::AccountId>;
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

decl_event!(
    pub enum Event<T> where
        <T as system::Trait>::AccountId
    {
        SetBitcoinTrusteeProps(AccountId, BitcoinTrusteeIntentionProps),
        BitcoinNewTrustees(u32, BitcoinTrusteeSessionInfo<AccountId>),
        //crosschain binding
        /// Record binding info for bitcoin addr(channel) and ChainX AccountId, params: new binding accountid, old binding accountid, crosschain addr, channel
        BitcoinBinding(AccountId, Option<AccountId>, BitcoinAddress, Option<AccountId>),
        /// Record binding info for ethereum addr(channel) and ChainX AccountId
        EthereumBinding(AccountId, Option<AccountId>, EthereumAddress, Option<AccountId>),
    }
);

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event<T>() = default;

        // trustees
        pub fn setup_bitcoin_trustee(origin, about: XString, hot_entity: H264, cold_entity: H264) -> Result {
            let who = ensure_signed(origin)?;
            Self::setup_bitcoin_trustee_impl(who, about, hot_entity, cold_entity)
        }

        /// use for trustee multisig addr
        pub fn transition_trustee_session(origin, chain: Chain, new_trustees: Vec<T::AccountId>) -> Result {
            let who = ensure_signed(origin)?;
            // judge current addr
            BitcoinTrusteeMultiSig::<T>::check_multisig(&who)?;
            info!("[transition_trustee_session]|try to transition trustee|from multisig addr:{:?}|chain:{:?}|new_trustees:{:?}", who, chain, new_trustees);
            Self::transition_trustee_session_impl(chain, new_trustees)
        }

        pub fn transition_trustee_session_by_root(chain: Chain, new_trustees: Vec<T::AccountId>) -> Result {
            info!("[transition_trustee_session_by_root]|try to transition trustee|chain:{:?}|new_trustees:{:?}", chain, new_trustees);
            Self::transition_trustee_session_impl(chain, new_trustees)
        }

        pub fn set_trustee_info_config(chain: Chain, config: TrusteeInfoConfig) -> Result {
            TrusteeInfoConfigOf::<T>::insert(chain, config);
            Ok(())
        }
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as XBridgeFeatures {
        // for trustee
        pub TrusteeMultiSigAddr get(trustee_multisig_addr): map Chain => T::AccountId;

        /// trustee basal info config
        pub TrusteeInfoConfigOf get(trustee_info_config) config(): map Chain => TrusteeInfoConfig;
        /// when generate trustee, auto generate a new session number, increase the newest trustee addr, can't modify by user
        pub TrusteeSessionInfoLen get(trustee_session_info_len): map Chain => u32;

        // for bitcoin
        /// all bitcoin session trustee addr
        pub BitcoinTrusteeSessionInfoOf get(bitcoin_trustee_session_info_of): map u32 => Option<BitcoinTrusteeSessionInfo<T::AccountId>>;
        /// properties for bitcoin trustees
        pub BitcoinTrusteeIntentionPropertiesOf get(bitcoin_trustee_intention_props_of): map T::AccountId => Option<BitcoinTrusteeIntentionProps>;
        // for other chain

        // for crosschain
        // for bitcoin
        /// account deposit accountid, chain => multi deposit addr
        pub BitcoinCrossChainBinding: map T::AccountId => Vec<BitcoinAddress>;
        /// account deposit addr => (accountid, option(channel accountid))  (channel is a validator)
        pub BitcoinCrossChainOf: map BitcoinAddress => Option<(T::AccountId, Option<T::AccountId>)>;
        // for sdot
        pub EthereumCrossChainBinding: map T::AccountId => Vec<EthereumAddress>;
        pub EthereumCrossChainOf: map EthereumAddress => Option<(T::AccountId, Option<T::AccountId>)>;

    }
}

// for trustees
impl<T: Trait> Module<T> {
    pub fn setup_bitcoin_trustee_impl(
        who: T::AccountId,
        about: XString,
        hot_entity: H264,
        cold_entity: H264,
    ) -> Result {
        ensure!(
            xaccounts::Module::<T>::is_intention(&who),
            "Transactor is not an intention."
        );
        xaccounts::is_valid_about::<T>(&about)?;

        let hot_pubkey = xbitcoin::Module::<T>::check_trustee_entity(hot_entity.as_ref())?;
        let cold_pubkey = xbitcoin::Module::<T>::check_trustee_entity(cold_entity.as_ref())?;

        let props = BitcoinTrusteeIntentionProps {
            about,
            hot_entity: hot_pubkey,
            cold_entity: cold_pubkey,
        };

        BitcoinTrusteeIntentionPropertiesOf::<T>::insert(&who, props.clone());
        Self::deposit_event(RawEvent::SetBitcoinTrusteeProps(who, props));
        Ok(())
    }

    #[inline]
    pub fn current_session_number(chain: Chain) -> u32 {
        match Self::trustee_session_info_len(chain).checked_sub(1) {
            Some(r) => r,
            None => u32::max_value(),
        }
    }
    #[inline]
    pub fn last_session_number(chain: Chain) -> u32 {
        match Self::current_session_number(chain).checked_sub(1) {
            Some(r) => r,
            None => u32::max_value(),
        }
    }

    fn new_trustee_session<F: FnOnce(u32)>(chain: Chain, func_setstorage: F) {
        let session_number = Self::trustee_session_info_len(chain);
        let number = match session_number.checked_add(1) {
            Some(n) => n,
            None => 0_u32,
        };
        func_setstorage(session_number);
        TrusteeSessionInfoLen::<T>::insert(chain, number);
    }

    fn try_generate_session_info<F1, Props, F2, SessionInfo: Clone>(
        new_trustees: Vec<T::AccountId>,
        config: TrusteeInfoConfig,
        get_trustee_props: F1,
        generate_session_info: F2,
    ) -> result::Result<SessionInfo, &'static str>
    where
        F1: Fn(&T::AccountId) -> Option<Props>,
        F2: FnOnce(
            Vec<(T::AccountId, Props)>,
            TrusteeInfoConfig,
        ) -> result::Result<SessionInfo, &'static str>,
    {
        // check duplicate
        let has_duplicate =
            (1..new_trustees.len()).any(|i| new_trustees[i..].contains(&new_trustees[i - 1]));
        if has_duplicate {
            error!(
                "[try_generate_session_info]|existing duplicate account|candidates:{:?}",
                new_trustees
            );
            return Err("existing duplicate account");
        }

        // read storage
        let mut props = vec![];
        for accountid in new_trustees.into_iter() {
            let p = get_trustee_props(&accountid).ok_or_else(|| {
                error!("[transition_trustee_session]|not all candidate has registered as a trustee yet|who:{:?}",  accountid);
                "not all candidate has registered as a trustee yet"
            })?;
            props.push((accountid, p));
        }

        // handle
        generate_session_info(props, config)
    }

    fn transition_new_session<F1, Props, F2, SessionInfo: Clone, F3>(
        chain: Chain,
        new_trustees: Vec<T::AccountId>,
        config: TrusteeInfoConfig,
        get_trustee_props: F1,
        generate_session_info: F2,
        set_new_session_info: F3,
    ) -> result::Result<SessionInfo, &'static str>
    where
        F1: Fn(&T::AccountId) -> Option<Props>,
        F2: FnOnce(
            Vec<(T::AccountId, Props)>,
            TrusteeInfoConfig,
        ) -> result::Result<SessionInfo, &'static str>,
        F3: FnOnce(u32, SessionInfo),
    {
        let session_info = Self::try_generate_session_info(
            new_trustees,
            config,
            get_trustee_props,
            generate_session_info,
        )?;
        // set to storage
        let f = |num: u32| {
            set_new_session_info(num, session_info.clone());
        };
        Self::new_trustee_session(chain, f);
        Ok(session_info)
    }

    fn transition_trustee_session_impl(chain: Chain, new_trustees: Vec<T::AccountId>) -> Result {
        let config = Self::trustee_info_config(chain);
        let trustees = match chain {
            Chain::Bitcoin => {
                let session_info = Self::transition_new_session(
                    chain,
                    new_trustees,
                    config,
                    |accountid: &T::AccountId| Self::bitcoin_trustee_intention_props_of(accountid),
                    xbitcoin::Module::<T>::generate_trustee_session_info,
                    |session_number, session_info| {
                        BitcoinTrusteeSessionInfoOf::<T>::insert(session_number, &session_info);
                        Module::<T>::deposit_event(RawEvent::BitcoinNewTrustees(
                            session_number,
                            session_info.clone(),
                        ));
                    },
                )?;
                session_info.trustee_list
            }
            _ => return Err("no transition trustee support for this chain"),
        };

        Self::deploy_trustee_addr_unsafe(chain, trustees);
        Ok(())
    }

    pub fn mock_trustee_session_impl(
        chain: Chain,
        new_trustees: Vec<T::AccountId>,
    ) -> result::Result<GenericAllSessionInfo<T::AccountId>, &'static str> {
        let config = Self::trustee_info_config(chain);
        let generic_all_info = match chain {
            Chain::Bitcoin => {
                let session_info = Self::try_generate_session_info(
                    new_trustees,
                    config,
                    |accountid: &T::AccountId| Self::bitcoin_trustee_intention_props_of(accountid),
                    xbitcoin::Module::<T>::generate_trustee_session_info,
                )?;
                into_generic_all_info(session_info, |accountid: &T::AccountId| {
                    Self::bitcoin_trustee_intention_props_of(accountid)
                })
            }
            _ => return Err("no transition trustee support for this chain"),
        };

        Ok(generic_all_info)
    }

    pub fn current_trustee_session_info_for(
        chain: Chain,
    ) -> Option<GenericAllSessionInfo<T::AccountId>> {
        match chain {
            Chain::Bitcoin => {
                let session_info = <Self as TrusteeSession<T::AccountId, BitcoinTrusteeAddrInfo>>::current_trustee_session().ok();
                session_info.map(|info| {
                    into_generic_all_info(info, |accountid: &T::AccountId| {
                        Self::bitcoin_trustee_intention_props_of(accountid)
                    })
                })
            }
            _ => None,
        }
    }

    pub fn trustee_props_for(who: &T::AccountId) -> BTreeMap<Chain, GenericTrusteeIntentionProps> {
        let mut m = BTreeMap::new();

        if let Some(props) = Self::bitcoin_trustee_intention_props_of(who) {
            m.insert(Chain::Bitcoin, props.into());
        }
        m
    }

    fn deploy_trustee_addr_unsafe(chain: Chain, trustee_list: Vec<T::AccountId>) {
        // generate new addr
        let addr = T::TrusteeMultiSig::multi_sig_addr_for_trustees(chain, &trustee_list);
        let deployer = trustee_list
            .get(0)
            .expect("the trustee_list len must large than 1")
            .clone();
        // calc required num
        let required_num = two_thirds_unsafe(trustee_list.len() as u32);

        let trustee_list = trustee_list
            .into_iter()
            .map(|accountid| (accountid, true))
            .collect::<Vec<_>>();
        xmultisig::Module::<T>::deploy_impl_unsafe(
            xmultisig::AddrType::Trustee,
            &addr,
            &deployer,
            trustee_list,
            required_num,
        );
        // change TrusteeMultiSigAddr
        TrusteeMultiSigAddr::<T>::insert(chain, addr);
    }

    #[cfg(feature = "std")]
    pub fn deploy_trustee_in_genesis(trustees: Vec<(Chain, Vec<T::AccountId>)>) -> Result {
        // deploy trustee
        for (chain, trustee_list) in trustees {
            Self::transition_trustee_session_impl(chain, trustee_list)?;
        }
        Ok(())
    }
}
