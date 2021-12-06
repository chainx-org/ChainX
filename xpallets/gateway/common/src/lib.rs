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

use frame_support::traits::{ChangeMembers, Get};
use frame_support::{
    dispatch::{DispatchError, DispatchResult},
    ensure,
    log::{error, info},
    weights::Weight,
};
use frame_system::{ensure_root, ensure_signed};
use sp_runtime::traits::{StaticLookup, Zero};
use sp_std::{collections::btree_map::BTreeMap, convert::TryFrom, prelude::*};

use chainx_primitives::{AddrStr, AssetId, ChainAddress, Text};
use xp_runtime::Memo;
use xpallet_assets::{AssetRestrictions, BalanceOf, Chain, ChainT, WithdrawalLimit};
use xpallet_gateway_records::{WithdrawalRecordId, WithdrawalState};
use xpallet_support::traits::{MultisigAddressFor, Validator};

use self::traits::{TrusteeForChain, TrusteeSession};
use self::types::{
    GenericTrusteeIntentionProps, GenericTrusteeSessionInfo, TrusteeInfoConfig,
    TrusteeIntentionProps,
};
pub use self::weights::WeightInfo;
use crate::types::ScriptInfo;
pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    #[pallet::config]
    pub trait Config:
        frame_system::Config + xpallet_gateway_records::Config + pallet_elections_phragmen::Config
    {
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
        type BitcoinTrusteeSessionProvider: TrusteeSession<
            Self::AccountId,
            trustees::bitcoin::BtcTrusteeAddrInfo,
        >;

        type WeightInfo: WeightInfo;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub (super) trait Store)]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        /// What to do at the end of each block.
        ///
        /// Checks if an trustee transition needs to happen or not.
        fn on_initialize(n: T::BlockNumber) -> Weight {
            let term_duration = Self::trustee_transition_duration();
            if !term_duration.is_zero() && (n % term_duration).is_zero() {
                Self::do_trustee_election()
            } else {
                0
            }
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Create a withdrawal.
        /// Withdraws some balances of `asset_id` to address `addr` of target chain.
        ///
        /// WithdrawalRecord State: `Applying`
        ///
        /// NOTE: `ext` is for the compatibility purpose, e.g., EOS requires a memo when doing the transfer.
        #[pallet::weight(< T as Config >::WeightInfo::withdraw())]
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
        #[pallet::weight(< T as Config >::WeightInfo::cancel_withdrawal())]
        pub fn cancel_withdrawal(origin: OriginFor<T>, id: WithdrawalRecordId) -> DispatchResult {
            let from = ensure_signed(origin)?;
            xpallet_gateway_records::Pallet::<T>::cancel_withdrawal(id, &from)
        }

        /// Setup the trustee info.
        #[pallet::weight(< T as Config >::WeightInfo::setup_trustee())]
        pub fn setup_trustee(
            origin: OriginFor<T>,
            proxy_account: Option<T::AccountId>,
            chain: Chain,
            about: Text,
            hot_entity: Vec<u8>,
            cold_entity: Vec<u8>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            // make sure this person is a pre-selected trustee
            // or the trustee is in little black house
            ensure!(
                Self::generate_trustee_pool().contains(&who)
                    || Self::little_black_house().contains(&who),
                Error::<T>::NotTrusteePreselectedMember
            );
            Self::setup_trustee_impl(who, proxy_account, chain, about, hot_entity, cold_entity)
        }

        /// Transition the trustee session.
        #[pallet::weight(< T as Config >::WeightInfo::transition_trustee_session())]
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

        /// Move a current trust into a small black room.
        ///
        /// This is to allow for timely replacement in the event of a problem with a particular trust.
        /// The trustee will be moved into the small black room.
        ///
        /// This is called by the trustee admin and root.
        /// # <weight>
        /// Since this is a root call and will go into trustee election, we assume full block for now.
        /// # </weight>
        #[pallet::weight(T::BlockWeights::get().max_block)]
        pub fn move_trust_to_black_room(
            origin: OriginFor<T>,
            trustees: Option<Vec<T::AccountId>>,
        ) -> DispatchResult {
            match ensure_signed(origin.clone()) {
                Ok(who) => {
                    if who != Self::trustee_admin() {
                        return Err(Error::<T>::NotTrusteeAdmin.into());
                    }
                }
                Err(_) => {
                    ensure_root(origin)?;
                }
            };

            info!(
                target: "runtime::gateway::common",
                "[move_trust_to_black_room] Try to move a trust to black room, trustee:{:?}",
                trustees
            );

            if let Some(trustees) = trustees {
                for trustee in trustees {
                    LittleBlackHouse::<T>::append(trustee);
                }
            }

            Self::do_trustee_election();
            Ok(())
        }

        /// Set the state of withdraw record by the trustees.
        #[pallet::weight(< T as Config >::WeightInfo::set_withdrawal_state())]
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
        #[pallet::weight(< T as Config >::WeightInfo::set_trustee_info_config())]
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
        #[pallet::weight(< T as Config >::WeightInfo::force_set_referral_binding())]
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

        /// Dangerous! Be careful to set TrusteeTransitionDuration
        #[pallet::weight(< T as Config >::WeightInfo::change_trustee_transition_duration())]
        pub fn change_trustee_transition_duration(
            origin: OriginFor<T>,
            duration: T::BlockNumber,
        ) -> DispatchResult {
            ensure_root(origin)?;
            TrusteeTransitionDuration::<T>::put(duration);
            Ok(())
        }

        /// Set the trustee admin.
        ///
        /// This is a root-only operation.
        /// The trustee admin is the account who can change the trustee list.
        #[pallet::weight(< T as Config >::WeightInfo::set_trustee_admin())]
        pub fn set_trustee_admin(
            origin: OriginFor<T>,
            admin: T::AccountId,
            chain: Chain,
        ) -> DispatchResult {
            ensure_root(origin)?;
            Self::trustee_intention_props_of(&admin, chain).ok_or_else::<DispatchError, _>(
                || {
                    error!(
                        target: "runtime::gateway::common",
                        "[set_trustee_admin] admin {:?} has not in TrusteeIntentionPropertiesOf",
                        admin
                    );
                    Error::<T>::NotRegistered.into()
                },
            )?;
            TrusteeAdmin::<T>::put(admin);
            Ok(())
        }
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub (super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A (potential) trustee set the required properties. [who, chain, trustee_props]
        SetTrusteeProps(
            T::AccountId,
            Chain,
            GenericTrusteeIntentionProps<T::AccountId>,
        ),
        /// An account set its referral_account of some chain. [who, chain, referral_account]
        ReferralBinded(T::AccountId, Chain, T::AccountId),
        /// The trustee set of a chain was changed. [chain, session_number, session_info, script_info]
        TrusteeSetChanged(
            Chain,
            u32,
            GenericTrusteeSessionInfo<T::AccountId>,
            ScriptInfo<T::AccountId>,
        ),
        /// The last trust transition was not completed.
        TrusteeTransitionNotCompleted,
        /// The trust members was not changed.
        TrusteeMembersNotChanged,
        /// The trust transition was failed.
        TrusteeTransitionFail,
    }

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
        /// unsupported chain
        NotSupportedChain,
        /// existing duplicate account
        DuplicatedAccountId,
        /// not registered as trustee
        NotRegistered,
        /// just allow validator to register trustee
        NotValidator,
        /// just allow trustee admin to remove trustee
        NotTrusteeAdmin,
        /// just allow trust preselected members to set their trust information
        NotTrusteePreselectedMember,
        /// invalid public key
        InvalidPublicKey,
    }

    #[pallet::storage]
    #[pallet::getter(fn trustee_multisig_addr)]
    pub type TrusteeMultiSigAddr<T: Config> =
        StorageMap<_, Twox64Concat, Chain, T::AccountId, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn trustee_admin)]
    pub type TrusteeAdmin<T: Config> = StorageValue<_, T::AccountId, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn agg_pubkey_info)]
    pub type AggPubkeyInfo<T: Config> =
        StorageMap<_, Twox64Concat, Vec<u8>, Vec<T::AccountId>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn trustee_sig_record)]
    pub type TrusteeSigRecord<T: Config> =
        StorageMap<_, Twox64Concat, T::AccountId, u32, ValueQuery>;

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
        GenericTrusteeIntentionProps<T::AccountId>,
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

    /// How long each trustee is kept. This defines the next block number at which an
    /// trustee transition will happen. If set to zero, no trustee transition are ever triggered.
    #[pallet::storage]
    #[pallet::getter(fn trustee_transition_duration)]
    pub type TrusteeTransitionDuration<T: Config> = StorageValue<_, T::BlockNumber, ValueQuery>;

    /// The status of the of the trustee transition
    #[pallet::storage]
    #[pallet::getter(fn trustee_transition_status)]
    pub type TrusteeTransitionStatus<T: Config> = StorageValue<_, bool, ValueQuery>;

    /// Members not participating in trust elections.
    ///
    /// The current trust members did not conduct multiple signings and put the members in the
    /// little black room. Filter out the member in the next trust election
    #[pallet::storage]
    #[pallet::getter(fn little_black_house)]
    pub type LittleBlackHouse<T: Config> = StorageValue<_, Vec<T::AccountId>, ValueQuery>;

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        pub trustees: Vec<(
            Chain,
            TrusteeInfoConfig,
            Vec<(T::AccountId, Text, Vec<u8>, Vec<u8>)>,
        )>,
        pub genesis_trustee_transition_duration: T::BlockNumber,
        pub genesis_trustee_transition_status: bool,
    }

    #[cfg(feature = "std")]
    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            Self {
                trustees: Default::default(),
                genesis_trustee_transition_duration: Default::default(),
                genesis_trustee_transition_status: Default::default(),
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
                            None,
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
                TrusteeTransitionDuration::<T>::put(config.genesis_trustee_transition_duration);
                TrusteeTransitionStatus::<T>::put(&config.genesis_trustee_transition_status);
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
            Chain::Bitcoin => T::Bitcoin::withdrawal_limit(asset_id),
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
                T::Bitcoin::check_addr(addr, b"")?;
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

    pub fn generate_trustee_pool() -> Vec<T::AccountId> {
        let members = pallet_elections_phragmen::Pallet::<T>::members()
            .iter()
            .map(|m| m.who.clone())
            .collect::<Vec<T::AccountId>>();
        let runnersup = pallet_elections_phragmen::Pallet::<T>::runners_up()
            .iter()
            .map(|m| m.who.clone())
            .collect::<Vec<T::AccountId>>();
        [members, runnersup].concat()
    }

    pub fn do_trustee_election() -> Weight {
        if Self::trustee_transition_status() {
            Self::deposit_event(Event::TrusteeTransitionNotCompleted);
            return T::DbWeight::get().reads(1);
        }

        // Current trust list
        let old_trustee_candidate: Vec<T::AccountId> =
            if let Ok(info) = T::BitcoinTrusteeSessionProvider::current_trustee_session() {
                info.trustee_list
            } else {
                vec![]
            };

        let multi_count_0 = old_trustee_candidate
            .iter()
            .filter_map(|acc| match Self::trustee_sig_record(acc) {
                0 => Some(acc.clone()),
                _ => None,
            })
            .collect::<Vec<T::AccountId>>();

        let filter_members: Vec<T::AccountId> =
            [Self::little_black_house(), multi_count_0].concat();

        let new_trustee_pool: Vec<T::AccountId> = Self::generate_trustee_pool()
            .iter()
            .filter_map(|who| match filter_members.contains(who) {
                true => None,
                false => Some(who.clone()),
            })
            .collect::<Vec<T::AccountId>>();

        let remain_filter_members = filter_members
            .iter()
            .filter_map(|who| match new_trustee_pool.contains(who) {
                true => Some(who.clone()),
                false => None,
            })
            .collect::<Vec<_>>();

        LittleBlackHouse::<T>::put(remain_filter_members);

        let desired_members =
            <T as pallet_elections_phragmen::Config>::DesiredMembers::get() as usize;

        if new_trustee_pool.len() < desired_members {
            Self::deposit_event(Event::TrusteeMembersNotChanged);
            return 0u64
                .saturating_add(T::DbWeight::get().writes(1))
                .saturating_add(T::DbWeight::get().reads(7));
        }

        let new_trustee_candidate = new_trustee_pool[..desired_members].to_vec();
        let mut new_trustee_candidate_sorted = new_trustee_candidate.clone();
        new_trustee_candidate_sorted.sort_unstable();

        let mut old_trustee_candidate_sorted = old_trustee_candidate;
        old_trustee_candidate_sorted.sort();
        let (incoming, outgoing) =
            <T as pallet_elections_phragmen::Config>::ChangeMembers::compute_members_diff_sorted(
                &old_trustee_candidate_sorted,
                &new_trustee_candidate_sorted,
            );
        if incoming.is_empty() && outgoing.is_empty() {
            Self::deposit_event(Event::TrusteeMembersNotChanged);
            return 0u64
                .saturating_add(T::DbWeight::get().writes(1))
                .saturating_add(T::DbWeight::get().reads(7));
        }
        if Self::transition_trustee_session_impl(Chain::Bitcoin, new_trustee_candidate).is_err() {
            Self::deposit_event(Event::TrusteeTransitionFail);
            return 0u64
                .saturating_add(T::DbWeight::get().writes(1))
                .saturating_add(T::DbWeight::get().reads(7))
                .saturating_add(<T as Config>::WeightInfo::transition_trustee_session());
        }

        TrusteeTransitionStatus::<T>::put(true);

        0u64.saturating_add(T::DbWeight::get().writes(2))
            .saturating_add(T::DbWeight::get().reads(7))
            .saturating_add(<T as Config>::WeightInfo::transition_trustee_session())
    }
}

pub fn is_valid_about<T: Config>(about: &[u8]) -> DispatchResult {
    // TODO
    if about.len() > 128 {
        return Err(Error::<T>::InvalidAboutLen.into());
    }

    xp_runtime::xss_check(about)
}

// trustees
impl<T: Config> Pallet<T> {
    pub fn setup_trustee_impl(
        who: T::AccountId,
        proxy_account: Option<T::AccountId>,
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

        let proxy_account = if let Some(addr) = proxy_account {
            Some(addr)
        } else {
            Some(who.clone())
        };

        let props = GenericTrusteeIntentionProps::<T::AccountId>(TrusteeIntentionProps::<
            T::AccountId,
            Vec<u8>,
        > {
            proxy_account,
            about,
            hot_entity: hot,
            cold_entity: cold,
        });

        if TrusteeIntentionPropertiesOf::<T>::contains_key(&who, chain) {
            if Self::little_black_house().contains(&who) {
                LittleBlackHouse::<T>::mutate(|house| house.retain(|a| *a != who));
            }
            TrusteeIntentionPropertiesOf::<T>::mutate(&who, chain, |t| *t = Some(props.clone()));
        } else {
            TrusteeIntentionPropertiesOf::<T>::insert(&who, chain, props.clone());
        }
        Self::deposit_event(Event::<T>::SetTrusteeProps(who, chain, props));
        Ok(())
    }

    pub fn try_generate_session_info(
        chain: Chain,
        new_trustees: Vec<T::AccountId>,
    ) -> Result<
        (
            GenericTrusteeSessionInfo<T::AccountId>,
            ScriptInfo<T::AccountId>,
        ),
        DispatchError,
    > {
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
                            TrusteeIntentionProps::<T::AccountId, _>::try_from(prop)
                                .expect("must decode succss from storage data"),
                        )
                    })
                    .collect();
                let session_info = T::BitcoinTrustee::generate_trustee_session_info(props, config)?;

                (session_info.0.into(), session_info.1)
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
        let multi_addr = Self::generate_multisig_addr(chain, &info.0)?;

        let session_number = Self::trustee_session_info_len(chain);
        // FIXME: rethink about the overflow case.
        let next_number = session_number.checked_add(1).unwrap_or(0u32);

        TrusteeSessionInfoLen::<T>::insert(chain, next_number);
        TrusteeSessionInfoOf::<T>::insert(chain, session_number, info.0.clone());
        TrusteeMultiSigAddr::<T>::insert(chain, multi_addr);

        for index in 0..info.1.agg_pubkeys.len() {
            AggPubkeyInfo::<T>::insert(
                &info.1.agg_pubkeys[index],
                info.1.personal_accounts[index].clone(),
            );
        }

        Self::deposit_event(Event::<T>::TrusteeSetChanged(
            chain,
            session_number,
            info.0,
            info.1,
        ));
        Ok(())
    }

    pub fn generate_multisig_addr(
        chain: Chain,
        session_info: &GenericTrusteeSessionInfo<T::AccountId>,
    ) -> Result<T::AccountId, DispatchError> {
        // If there is a proxy account, choose a proxy account
        let mut acc_list: Vec<T::AccountId> = vec![];
        for acc in session_info.0.trustee_list.iter() {
            let acc = Self::trustee_intention_props_of(acc, chain)
                .ok_or_else::<DispatchError, _>(|| {
                    error!(
                        target: "runtime::gateway::common",
                        "[generate_multisig_addr] acc {:?} has not in TrusteeIntentionPropertiesOf",
                        acc
                    );
                    Error::<T>::NotRegistered.into()
                })?
                .0
                .proxy_account
                .unwrap_or_else(|| acc.clone());
            acc_list.push(acc);
        }

        let multi_addr =
            T::DetermineMultisigAddress::calc_multisig(&acc_list, session_info.0.threshold);

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
