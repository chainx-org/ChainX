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

use frame_support::{
    dispatch::{DispatchError, DispatchResult},
    ensure,
    log::{error, info},
};
use frame_system::{ensure_root, ensure_signed};
use sp_runtime::traits::StaticLookup;
use sp_std::{collections::btree_map::BTreeMap, convert::TryFrom, prelude::*};

use chainx_primitives::{AddrStr, AssetId, ChainAddress, Text};
use xp_runtime::Memo;
use xpallet_assets::{AssetRestrictions, BalanceOf, Chain, ChainT, WithdrawalLimit};
use xpallet_gateway_records::{WithdrawalRecordId, WithdrawalState};
use xpallet_support::traits::{MultisigAddressFor, Validator};

use self::traits::TrusteeForChain;
use self::types::{
    GenericTrusteeIntentionProps, GenericTrusteeSessionInfo, TrusteeInfoConfig,
    TrusteeIntentionProps,
};
pub use self::weights::WeightInfo;
pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    #[pallet::config]
    pub trait Config: frame_system::Config + xpallet_gateway_records::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
        type Validator: Validator<Self::AccountId>;

        type DetermineMultisigAddress: MultisigAddressFor<Self::AccountId>;

        // for bitcoin
        type Bitcoin: ChainT<BalanceOf<Self>>;
        type BitcoinTrustee: TrusteeForChain<
            Self::AccountId,
            trustees::bitcoin::BtcTrusteeType,
            trustees::bitcoin::BtcTrusteeAddrInfo,
        >;

        type WeightInfo: WeightInfo;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Create a withdrawal.
        /// Withdraws some balances of `asset_id` to address `addr` of target chain.
        ///
        /// WithdrawalRecord State: `Applying`
        ///
        /// NOTE: `ext` is for the compatibility purpose, e.g., EOS requires a memo when doing the transfer.
        #[pallet::weight(<T as Config>::WeightInfo::withdraw())]
        pub fn withdraw(
            origin: OriginFor<T>,
            #[pallet::compact] asset_id: AssetId,
            #[pallet::compact] value: BalanceOf<T>,
            addr: AddrStr,
            ext: Memo,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            ensure!(
                xpallet_assets::Pallet::<T>::can_do(&asset_id, AssetRestrictions::WITHDRAW),
                xpallet_assets::Error::<T>::ActionNotAllowed,
            );
            Self::verify_withdrawal(asset_id, value, &addr, &ext)?;

            xpallet_gateway_records::Pallet::<T>::withdraw(&who, asset_id, value, addr, ext)?;
            Ok(())
        }

        /// Cancel the withdrawal by the applicant.
        ///
        /// WithdrawalRecord State: `Applying` ==> `NormalCancel`
        #[pallet::weight(<T as Config>::WeightInfo::cancel_withdrawal())]
        pub fn cancel_withdrawal(origin: OriginFor<T>, id: WithdrawalRecordId) -> DispatchResult {
            let from = ensure_signed(origin)?;
            xpallet_gateway_records::Pallet::<T>::cancel_withdrawal(id, &from)
        }

        /// Setup the trustee.
        #[pallet::weight(<T as Config>::WeightInfo::setup_trustee())]
        pub fn setup_trustee(
            origin: OriginFor<T>,
            chain: Chain,
            about: Text,
            hot_entity: Vec<u8>,
            cold_entity: Vec<u8>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(T::Validator::is_validator(&who), Error::<T>::NotValidator);
            Self::setup_trustee_impl(who, chain, about, hot_entity, cold_entity)
        }

        /// Transition the trustee session.
        #[pallet::weight(<T as Config>::WeightInfo::transition_trustee_session(new_trustees.len() as u32))]
        pub fn transition_trustee_session(
            origin: OriginFor<T>,
            chain: Chain,
            new_trustees: Vec<T::AccountId>,
        ) -> DispatchResult {
            match ensure_signed(origin.clone()) {
                Ok(who) => {
                    if who != Self::trustee_multisig_addr(chain) {
                        return Err(Error::<T>::InvalidMultisig.into());
                    }
                }
                Err(_) => {
                    ensure_root(origin)?;
                }
            };

            info!(
                target: "runtime::gateway::common",
                "[transition_trustee_session] Try to transition trustees, chain:{:?}, new_trustees:{:?}",
                chain,
                new_trustees
            );
            Self::transition_trustee_session_impl(chain, new_trustees)
        }

        /// Set the state of withdraw record by the trustees.
        #[pallet::weight(<T as Config>::WeightInfo::set_withdrawal_state())]
        pub fn set_withdrawal_state(
            origin: OriginFor<T>,
            #[pallet::compact] id: WithdrawalRecordId,
            state: WithdrawalState,
        ) -> DispatchResult {
            let from = ensure_signed(origin)?;

            let map = Self::trustee_multisigs();
            let chain = map
                .into_iter()
                .find_map(|(chain, multisig)| if from == multisig { Some(chain) } else { None })
                .ok_or(Error::<T>::InvalidMultisig)?;

            xpallet_gateway_records::Pallet::<T>::set_withdrawal_state_by_trustees(id, chain, state)
        }

        /// Set the config of trustee information.
        ///
        /// This is a root-only operation.
        #[pallet::weight(<T as Config>::WeightInfo::set_trustee_info_config())]
        pub fn set_trustee_info_config(
            origin: OriginFor<T>,
            chain: Chain,
            config: TrusteeInfoConfig,
        ) -> DispatchResult {
            ensure_root(origin)?;
            TrusteeInfoConfigOf::<T>::insert(chain, config);
            Ok(())
        }

        /// Set the referral binding of corresponding chain and account.
        ///
        /// This is a root-only operation.
        #[pallet::weight(<T as Config>::WeightInfo::force_set_referral_binding())]
        pub fn force_set_referral_binding(
            origin: OriginFor<T>,
            chain: Chain,
            who: <T::Lookup as StaticLookup>::Source,
            referral: <T::Lookup as StaticLookup>::Source,
        ) -> DispatchResult {
            ensure_root(origin)?;
            let who = T::Lookup::lookup(who)?;
            let referral = T::Lookup::lookup(referral)?;
            Self::set_referral_binding(chain, who, referral);
            Ok(())
        }
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    #[pallet::metadata(T::AccountId = "AccountId")]
    pub enum Event<T: Config> {
        /// A (potential) trustee set the required properties. [who, chain, trustee_props]
        SetTrusteeProps(T::AccountId, Chain, GenericTrusteeIntentionProps),
        /// An account set its referral_account of some chain. [who, chain, referral_account]
        ReferralBinded(T::AccountId, Chain, T::AccountId),
        /// The trustee set of a chain was changed. [chain, session_number, session_info]
        TrusteeSetChanged(Chain, u32, GenericTrusteeSessionInfo<T::AccountId>),
    }

    /// Old name generated by `decl_event`.
    #[deprecated(note = "use `Event` instead")]
    pub type RawEvent<T> = Event<T>;

    #[pallet::error]
    pub enum Error<T> {
        /// the value of withdrawal less than than the minimum value
        InvalidWithdrawal,
        /// convert generic data into trustee session info error
        InvalidGenericData,
        /// trustee session info not found
        InvalidTrusteeSession,
        /// exceed the maximum length of the about field of trustess session info
        InvalidAboutLen,
        /// invalid multisig
        InvalidMultisig,
        /// Unsupported chain
        NotSupportedChain,
        /// existing duplicate account
        DuplicatedAccountId,
        /// not registered as trustee
        NotRegistered,
        /// just allow validator to register trustee
        NotValidator,
    }

    #[pallet::storage]
    #[pallet::getter(fn trustee_multisig_addr)]
    pub type TrusteeMultiSigAddr<T: Config> =
        StorageMap<_, Twox64Concat, Chain, T::AccountId, ValueQuery>;

    /// Trustee info config of the corresponding chain.
    #[pallet::storage]
    #[pallet::getter(fn trustee_info_config_of)]
    pub type TrusteeInfoConfigOf<T: Config> =
        StorageMap<_, Twox64Concat, Chain, TrusteeInfoConfig, ValueQuery>;

    #[pallet::type_value]
    pub fn DefaultForTrusteeSessionInfoLen() -> u32 {
        0
    }

    /// Next Trustee session info number of the chain.
    ///
    /// Auto generate a new session number (0) when generate new trustee of a chain.
    /// If the trustee of a chain is changed, the corresponding number will increase by 1.
    ///
    /// NOTE: The number can't be modified by users.
    #[pallet::storage]
    #[pallet::getter(fn trustee_session_info_len)]
    pub type TrusteeSessionInfoLen<T: Config> =
        StorageMap<_, Twox64Concat, Chain, u32, ValueQuery, DefaultForTrusteeSessionInfoLen>;

    /// Trustee session info of the corresponding chain and number.
    #[pallet::storage]
    #[pallet::getter(fn trustee_session_info_of)]
    pub type TrusteeSessionInfoOf<T: Config> = StorageDoubleMap<
        _,
        Twox64Concat,
        Chain,
        Twox64Concat,
        u32,
        GenericTrusteeSessionInfo<T::AccountId>,
    >;

    /// Trustee intention properties of the corresponding account and chain.
    #[pallet::storage]
    #[pallet::getter(fn trustee_intention_props_of)]
    pub type TrusteeIntentionPropertiesOf<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        Twox64Concat,
        Chain,
        GenericTrusteeIntentionProps,
    >;

    /// The account of the corresponding chain and chain address.
    #[pallet::storage]
    pub type AddressBindingOf<T: Config> =
        StorageDoubleMap<_, Twox64Concat, Chain, Blake2_128Concat, ChainAddress, T::AccountId>;

    /// The bound address of the corresponding account and chain.
    #[pallet::storage]
    pub type BoundAddressOf<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        Twox64Concat,
        Chain,
        Vec<ChainAddress>,
        ValueQuery,
    >;

    /// The referral account of the corresponding account and chain.
    #[pallet::storage]
    #[pallet::getter(fn referral_binding_of)]
    pub type ReferralBindingOf<T: Config> =
        StorageDoubleMap<_, Blake2_128Concat, T::AccountId, Twox64Concat, Chain, T::AccountId>;

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        pub trustees: Vec<(
            Chain,
            TrusteeInfoConfig,
            Vec<(T::AccountId, Text, Vec<u8>, Vec<u8>)>,
        )>,
    }

    #[cfg(feature = "std")]
    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            Self {
                trustees: Default::default(),
            }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
        fn build(&self) {
            let extra_genesis_builder: fn(&Self) = |config| {
                for (chain, info_config, trustee_infos) in config.trustees.iter() {
                    let mut trustees = Vec::with_capacity(trustee_infos.len());
                    for (who, about, hot, cold) in trustee_infos.iter() {
                        Pallet::<T>::setup_trustee_impl(
                            who.clone(),
                            *chain,
                            about.clone(),
                            hot.clone(),
                            cold.clone(),
                        )
                        .expect("setup trustee can not fail; qed");
                        trustees.push(who.clone());
                    }
                    TrusteeInfoConfigOf::<T>::insert(chain, info_config.clone());
                }
            };
            extra_genesis_builder(self);
        }
    }
}

// withdraw
impl<T: Config> Pallet<T> {
    pub fn withdrawal_limit(
        asset_id: &AssetId,
    ) -> Result<WithdrawalLimit<BalanceOf<T>>, DispatchError> {
        let chain = xpallet_assets_registrar::Pallet::<T>::chain_of(asset_id)?;
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

        let chain = xpallet_assets_registrar::Pallet::<T>::chain_of(&asset_id)?;
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

pub fn is_valid_about<T: Config>(about: &[u8]) -> DispatchResult {
    if about.len() > 128 {
        return Err(Error::<T>::InvalidAboutLen.into());
    }

    xp_runtime::xss_check(about)
}

// trustees
impl<T: Config> Pallet<T> {
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
        let config = Self::trustee_info_config_of(chain);
        let has_duplicate =
            (1..new_trustees.len()).any(|i| new_trustees[i..].contains(&new_trustees[i - 1]));
        if has_duplicate {
            error!(
                target: "runtime::gateway::common",
                "[try_generate_session_info] Duplicate account, candidates:{:?}",
                new_trustees
            );
            return Err(Error::<T>::DuplicatedAccountId.into());
        }
        let mut props = Vec::with_capacity(new_trustees.len());
        for accountid in new_trustees.into_iter() {
            let p = Self::trustee_intention_props_of(&accountid, chain).ok_or_else(|| {
                error!(
                    target: "runtime::gateway::common",
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

        TrusteeSessionInfoLen::<T>::insert(chain, next_number);
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
            T::DetermineMultisigAddress::calc_multisig(&info.0.trustee_list, info.0.threshold);

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
        ReferralBindingOf::<T>::insert(&who, &chain, referral.clone());
        Self::deposit_event(Event::<T>::ReferralBinded(who, chain, referral))
    }
}

impl<T: Config> Pallet<T> {
    pub fn trustee_multisigs() -> BTreeMap<Chain, T::AccountId> {
        TrusteeMultiSigAddr::<T>::iter().collect()
    }
}
