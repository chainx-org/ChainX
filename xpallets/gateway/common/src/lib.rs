// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

//! this module is for bridge common parts
//! define trait and type for
//! `trustees`, `crosschain binding` and something others

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::new_without_default, clippy::type_complexity)]

#[cfg(any(feature = "runtime-benchmarks", test))]
mod benchmarking;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

mod binding;
pub mod traits;
pub mod trustees;
pub mod types;
pub mod utils;
pub mod weights;

use sp_runtime::traits::StaticLookup;
use sp_std::{collections::btree_map::BTreeMap, convert::TryFrom, prelude::*};

use frame_support::{
    decl_error, decl_event, decl_module, decl_storage,
    dispatch::{DispatchError, DispatchResult},
    ensure, IterableStorageMap,
};
use frame_system::{ensure_root, ensure_signed};

use chainx_primitives::{AddrStr, AssetId, ChainAddress, Text};
use xp_logging::{error, info};
use xp_runtime::Memo;
use xpallet_assets::{AssetRestrictions, BalanceOf, Chain, ChainT, WithdrawalLimit};
use xpallet_gateway_records::WithdrawalState;
use xpallet_support::traits::{MultisigAddressFor, Validator};

use crate::traits::TrusteeForChain;
use crate::types::{
    GenericTrusteeIntentionProps, GenericTrusteeSessionInfo, TrusteeInfoConfig,
    TrusteeIntentionProps,
};
pub use crate::weights::WeightInfo;

pub trait Trait: xpallet_gateway_records::Trait {
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

    type Validator: Validator<Self::AccountId>;

    type DetermineMultisigAddress: MultisigAddressFor<Self::AccountId>;

    // for chain
    type Bitcoin: ChainT<BalanceOf<Self>>;

    type BitcoinTrustee: TrusteeForChain<
        Self::AccountId,
        trustees::bitcoin::BtcTrusteeType,
        trustees::bitcoin::BtcTrusteeAddrInfo,
    >;

    type WeightInfo: WeightInfo;
}

decl_event!(
    pub enum Event<T> where
        <T as frame_system::Trait>::AccountId,
    {
        /// A (potential) trustee set the required properties. [who, chain, trustee_props]
        SetTrusteeProps(AccountId, Chain, GenericTrusteeIntentionProps),
        /// An account set its referral_account of some chain. [who, chain, referral_account]
        ReferralBinded(AccountId, Chain, AccountId),
        /// The trustee set of a chain was changed. [chain, session_number, session_info]
        TrusteeSetChanged(Chain, u32, GenericTrusteeSessionInfo<AccountId>),
    }
);

decl_error! {
    /// Error for the This Module
    pub enum Error for Module<T: Trait> {
        ///
        InvalidWithdrawal,
        ///
        InvalidGenericData,
        ///
        InvalidTrusteeSession,
        ///
        InvalidAboutLen,
        ///
        InvalidMultisig,
        ///
        NotSupportedChain,
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
        type Error = Error<T>;
        fn deposit_event() = default;

        /// Withdraws some balances of `asset_id` to address `addr` of target chain.
        ///
        /// NOTE: `ext` is for the compatiblity purpose, e.g., EOS requires a memo when doing the transfer.
        #[weight = <T as Trait>::WeightInfo::withdraw()]
        pub fn withdraw(
            origin,
            #[compact] asset_id: AssetId,
            #[compact] value: BalanceOf<T>,
            addr: AddrStr,
            ext: Memo
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            ensure!(
                xpallet_assets::Module::<T>::can_do(&asset_id, AssetRestrictions::WITHDRAW),
                xpallet_assets::Error::<T>::ActionNotAllowed,
            );
            Self::verify_withdrawal(asset_id, value, &addr, &ext)?;

            xpallet_gateway_records::Module::<T>::withdraw(&who, asset_id, value, addr, ext)?;
            Ok(())
        }

        #[weight = <T as Trait>::WeightInfo::withdraw()]
        pub fn revoke_withdraw(origin, id: u32) -> DispatchResult {
            let from = ensure_signed(origin)?;
            xpallet_gateway_records::Module::<T>::cancel_withdrawal(id, &from)
        }

        // trustees
        #[weight = <T as Trait>::WeightInfo::setup_trustee()]
        pub fn setup_trustee(
            origin,
            chain: Chain,
            about: Text,
            hot_entity: Vec<u8>,
            cold_entity: Vec<u8>
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(T::Validator::is_validator(&who), Error::<T>::NotValidator);
            Self::setup_trustee_impl(who, chain, about, hot_entity, cold_entity)
        }

        /// use for trustee multisig addr
        #[weight = <T as Trait>::WeightInfo::transition_trustee_session(new_trustees.len() as u32)]
        pub fn transition_trustee_session(
            origin,
            chain: Chain,
            new_trustees: Vec<T::AccountId>
        ) -> DispatchResult {
            match ensure_signed(origin.clone()) {
                Ok(who) => {
                    if who != Self::trustee_multisig_addr(chain) {
                        return Err(Error::<T>::InvalidMultisig.into());
                    }
                },
                Err(_) => {
                    ensure_root(origin)?;
                },
            };

            info!(
                "[transition_trustee_session_by_root] Try to transition trustees, chain:{:?}, new_trustees:{:?}",
                chain,
                new_trustees
            );
            Self::transition_trustee_session_impl(chain, new_trustees)
        }

        #[weight = <T as Trait>::WeightInfo::set_withdrawal_state()]
        pub fn set_withdrawal_state(
            origin,
            #[compact] withdrawal_id: u32,
            state: WithdrawalState
        ) -> DispatchResult {
            let from = ensure_signed(origin)?;

            let map = Self::trustee_multisigs();
            let chain = map
                .into_iter()
                .find_map(|(chain, multisig)| if from == multisig { Some(chain) } else { None })
                .ok_or(Error::<T>::InvalidMultisig)?;

            xpallet_gateway_records::Module::<T>::set_withdrawal_state_by_trustees(withdrawal_id, chain, state)
        }

        #[weight = <T as Trait>::WeightInfo::set_trustee_info_config()]
        pub fn set_trustee_info_config(origin, chain: Chain, config: TrusteeInfoConfig) -> DispatchResult {
            ensure_root(origin)?;
            TrusteeInfoConfigOf::insert(chain, config);
            Ok(())
        }

        #[weight = 0]
        pub fn force_set_binding(
            origin,
            chain: Chain,
            who: <T::Lookup as StaticLookup>::Source,
            referral: <T::Lookup as StaticLookup>::Source
        ) -> DispatchResult {
            ensure_root(origin)?;
            let who = T::Lookup::lookup(who)?;
            let referral = T::Lookup::lookup(referral)?;
            Self::set_referral_binding(chain, who, referral);
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
            double_map hasher(twox_64_concat) Chain, hasher(twox_64_concat) u32
            => Option<GenericTrusteeSessionInfo<T::AccountId>>;

        pub TrusteeIntentionPropertiesOf get(fn trustee_intention_props_of):
            double_map hasher(blake2_128_concat) T::AccountId, hasher(twox_64_concat) Chain
            => Option<GenericTrusteeIntentionProps>;

        pub AddressBinding:
            double_map hasher(twox_64_concat) Chain, hasher(blake2_128_concat) ChainAddress
            => Option<T::AccountId>;

        pub BoundAddressOf:
            double_map hasher(blake2_128_concat) T::AccountId, hasher(twox_64_concat) Chain
            => Vec<ChainAddress>;

        pub ChannelBindingOf get(fn channel_binding_of):
            double_map hasher(blake2_128_concat) T::AccountId, hasher(twox_64_concat) Chain
            => Option<T::AccountId>;
    }
    add_extra_genesis {
        config(trustees): Vec<(Chain, TrusteeInfoConfig, Vec<(T::AccountId, Text, Vec<u8>, Vec<u8>)>)>;
        build(|config| {
            for (chain, info_config, trustee_infos) in config.trustees.iter() {
                let mut trustees = Vec::with_capacity(trustee_infos.len());
                for (who, about, hot, cold) in trustee_infos.iter() {
                    Module::<T>::setup_trustee_impl(
                        who.clone(),
                        *chain,
                        about.clone(),
                        hot.clone(),
                        cold.clone(),
                    )
                    .expect("setup trustee can not fail; qed");
                    trustees.push(who.clone());
                }
                // config set should before transition
                TrusteeInfoConfigOf::insert(chain, info_config.clone());
                // trustee init should happen in chain module like gateway_bitcoin/gateway_ethereum.
            }
        })
    }
}

// withdraw
impl<T: Trait> Module<T> {
    pub fn withdrawal_limit(
        asset_id: &AssetId,
    ) -> Result<WithdrawalLimit<BalanceOf<T>>, DispatchError> {
        let chain = xpallet_assets_registrar::Module::<T>::chain_of(asset_id)?;
        match chain {
            Chain::Bitcoin => T::Bitcoin::withdrawal_limit(&asset_id),
            _ => Err(Error::<T>::NotSupportedChain.into()),
        }
    }

    pub fn verify_withdrawal(
        asset_id: AssetId,
        value: BalanceOf<T>,
        addr: &[u8],
        ext: &Memo,
    ) -> DispatchResult {
        ext.check_validity()?;

        let chain = xpallet_assets_registrar::Module::<T>::chain_of(&asset_id)?;
        match chain {
            Chain::Bitcoin => {
                // bitcoin do not need memo
                T::Bitcoin::check_addr(&addr, b"")?;
            }
            _ => return Err(Error::<T>::NotSupportedChain.into()),
        };
        // we could only split withdrawal limit due to a runtime-api would call `withdrawal_limit`
        // to export `WithdrawalLimit` for an asset.
        let limit = Self::withdrawal_limit(&asset_id)?;
        // withdrawal value should larger than minimal_withdrawal, allow equal
        if value < limit.minimal_withdrawal {
            return Err(Error::<T>::InvalidWithdrawal.into());
        }
        Ok(())
    }
}

pub fn is_valid_about<T: Trait>(about: &[u8]) -> DispatchResult {
    // TODO
    if about.len() > 128 {
        return Err(Error::<T>::InvalidAboutLen.into());
    }

    xp_runtime::xss_check(about)
}

// trustees
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
            _ => return Err(Error::<T>::NotSupportedChain.into()),
        };

        let props = GenericTrusteeIntentionProps(TrusteeIntentionProps::<Vec<u8>> {
            about,
            hot_entity: hot,
            cold_entity: cold,
        });

        TrusteeIntentionPropertiesOf::<T>::insert(&who, chain, props.clone());
        Self::deposit_event(Event::<T>::SetTrusteeProps(who, chain, props));
        Ok(())
    }

    pub fn try_generate_session_info(
        chain: Chain,
        new_trustees: Vec<T::AccountId>,
    ) -> Result<GenericTrusteeSessionInfo<T::AccountId>, DispatchError> {
        let config = Self::trustee_info_config(chain);
        let has_duplicate =
            (1..new_trustees.len()).any(|i| new_trustees[i..].contains(&new_trustees[i - 1]));
        if has_duplicate {
            error!(
                "[try_generate_session_info] Duplicate account, candidates:{:?}",
                new_trustees
            );
            return Err(Error::<T>::DuplicatedAccountId.into());
        }
        let mut props = Vec::with_capacity(new_trustees.len());
        for accountid in new_trustees.into_iter() {
            let p = Self::trustee_intention_props_of(&accountid, chain).ok_or_else(|| {
                error!(
                    "[transition_trustee_session] Candidate {:?} has not registered as a trustee",
                    accountid
                );
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
            _ => return Err(Error::<T>::NotSupportedChain.into()),
        };
        Ok(info)
    }

    fn transition_trustee_session_impl(
        chain: Chain,
        new_trustees: Vec<T::AccountId>,
    ) -> DispatchResult {
        let info = Self::try_generate_session_info(chain, new_trustees)?;
        let multi_addr = Self::generate_multisig_addr(chain, &info)?;

        let session_number = Self::trustee_session_info_len(chain);
        // FIXME: rethink about the overflow case.
        let next_number = session_number.checked_add(1).unwrap_or(0u32);

        TrusteeSessionInfoLen::insert(chain, next_number);
        TrusteeSessionInfoOf::<T>::insert(chain, session_number, info.clone());
        TrusteeMultiSigAddr::<T>::insert(chain, multi_addr);

        Self::deposit_event(Event::<T>::TrusteeSetChanged(chain, session_number, info));
        Ok(())
    }

    pub fn generate_multisig_addr(
        chain: Chain,
        info: &GenericTrusteeSessionInfo<T::AccountId>,
    ) -> Result<T::AccountId, DispatchError> {
        let multi_addr =
            T::DetermineMultisigAddress::calc_multisig(&info.trustee_list, info.threshold);

        // Each chain must have a distinct multisig address,
        // duplicated multisig address is not allowed.
        let find_duplicated = Self::trustee_multisigs()
            .into_iter()
            .any(|(c, multisig)| multi_addr == multisig && c == chain);
        if find_duplicated {
            return Err(Error::<T>::InvalidMultisig.into());
        }
        Ok(multi_addr)
    }

    fn set_referral_binding(chain: Chain, who: T::AccountId, referral: T::AccountId) {
        ChannelBindingOf::<T>::insert(&who, &chain, referral.clone());
        Self::deposit_event(Event::<T>::ReferralBinded(who, chain, referral))
    }
}

impl<T: Trait> Module<T> {
    pub fn trustee_multisigs() -> BTreeMap<Chain, T::AccountId> {
        TrusteeMultiSigAddr::<T>::iter().collect()
    }
}
