// Copyright 2018-2019 Chainpool.
//! Staking manager: Periodically determines the best set of validators.

#![cfg_attr(not(feature = "std"), no_std)]

// Substrate
use primitives::traits::{Lookup, StaticLookup};
use substrate_primitives::H512;

use rstd::prelude::*;
use rstd::result;
use support::{decl_event, decl_module, decl_storage, dispatch::Result, StorageMap, StorageValue};
use system::ensure_signed;

// ChainX
use xsupport::{debug, ensure_with_errorlog, info};
#[cfg(feature = "std")]
use xsupport::{u8array_to_string, who};

pub trait Trait: xstaking::Trait {
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
    type CheckHeader: CheckHeader<
        <Self as system::Trait>::AccountId,
        <Self as system::Trait>::BlockNumber,
    >;
}

pub trait CheckHeader<AccountId, BlockNumber: Default> {
    /// Check if the header is signed by the given signer.
    fn check_header(
        signer: &AccountId,
        first: &(RawHeader, u64, H512),
        second: &(RawHeader, u64, H512),
    ) -> result::Result<(BlockNumber, BlockNumber), &'static str>;
}

impl<AccountId, BlockNumber: Default> CheckHeader<AccountId, BlockNumber> for () {
    fn check_header(
        _signer: &AccountId,
        _first: &(RawHeader, u64, H512),
        _second: &(RawHeader, u64, H512),
    ) -> result::Result<(BlockNumber, BlockNumber), &'static str> {
        Ok((Default::default(), Default::default()))
    }
}

pub type RawHeader = Vec<u8>;

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event<T>() = default;

        /// Report the double sign misbehavior by fisherman.
        /// the header is tuple of (pre_header(Vec<u8>), signature(64Bytes), slot(u64))
        fn report_double_signer(
            origin,
            double_signer: <T::Lookup as StaticLookup>::Source,
            fst_header: (RawHeader, u64, H512),
            snd_header: (RawHeader, u64, H512)
        ) -> Result {
            let who = ensure_signed(origin)?;
            ensure_with_errorlog!(
                Self::fishermen().contains(&who),
                "Only the fisherman can report the double signer.",
                "Only the fisherman can report the double signer|current fishermen:{:?}|sender{:?}", Self::fishermen(), who
            );

            let double_signer = system::ChainContext::<T>::default().lookup(double_signer)?;
            debug!("report double signer|signer:{:?}|first:({:?}, {:}, {:?})|second:({:?}, {:}, {:?})",
                double_signer,
                u8array_to_string(&fst_header.0), fst_header.1, fst_header.2,
                u8array_to_string(&snd_header.0), snd_header.1, snd_header.2,
            );

            let (fst_height, snd_height) = T::CheckHeader::check_header(&who, &fst_header, &snd_header)?;

            let reported_key1 = (fst_height, double_signer.clone());
            let reported_key2 = (snd_height, double_signer.clone());
            ensure_with_errorlog!(
                <Reported<T>>::get(&reported_key1).is_none() && <Reported<T>>::get(&reported_key2).is_none(),
                "The double signer at this height has been reported already.",
                "The double signer at this height has been reported already|header1_key:{:?}|header2_key:{:?}", reported_key1, reported_key2
            );

            Self::slash(&double_signer, fst_height, snd_height, fst_header.1);

            <Reported<T>>::insert(&reported_key1, ());
            <Reported<T>>::insert(&reported_key2, ());
            Ok(())
        }

        /// Add a new fisherman.
        fn register_fisherman(who: T::AccountId) {
            let mut fishermen = Self::fishermen();
            if !fishermen.contains(&who) {
                fishermen.push(who);
            }
            info!("add fisher|current fishermen:{:?}", fishermen);
            <Fishermen<T>>::put(fishermen);
        }

        /// Remove a fisherman.
        fn remove_fisherman(who: T::AccountId) {
            let mut fishermen = Self::fishermen();
            fishermen.retain(|x| *x != who);
            info!("remove fisher|current fishermen:{:?}|remove:{:?}", fishermen, who);
            <Fishermen<T>>::put(fishermen);
        }
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as XFisher {
        /// If the validator has been reported the double sign misbehavior at a certain height.
        pub Reported get(reported): map (T::BlockNumber, T::AccountId) => Option<()>;

        /// Qualified accounts to report the double signer.
        pub Fishermen get(fishermen): Vec<T::AccountId>;
    }
}

decl_event!(
    pub enum Event<T> where
    <T as system::Trait>::BlockNumber,
    <T as xassets::Trait>::Balance,
    <T as system::Trait>::AccountId
    {
        SlashDoubleSigner(BlockNumber, BlockNumber, u64, AccountId, Balance),
    }
);

impl<T: Trait> Module<T> {
    /// Try removing the double signer from the validator set.
    fn try_reset_validators_given_double_signer(who: &T::AccountId) {
        let mut validators = <xsession::Module<T>>::validators()
            .into_iter()
            .map(|(v, _)| v)
            .collect::<Vec<_>>();

        if validators.contains(who)
            && validators.len() > xstaking::Module::<T>::minimum_validator_count() as usize
        {
            validators.retain(|x| *x != *who);
            info!(
                "[slash_double_signer] {:?} has been removed from the validator set, the latest validator set: {:?}",
                who!(who),
                validators.clone()
            );
            xstaking::Module::<T>::set_validators_on_non_era(validators);
        }
    }

    fn slash(
        who: &T::AccountId,
        fst_height: T::BlockNumber,
        snd_height: T::BlockNumber,
        slot: u64,
    ) {
        // Slash the whole jackpot of double signer.
        let council = xaccounts::Module::<T>::council_account();
        let jackpot = xstaking::Module::<T>::jackpot_accountid_for(who);

        let slashed = <xassets::Module<T>>::pcx_free_balance(&jackpot);
        let _ = <xassets::Module<T>>::pcx_move_free_balance(&jackpot, &council, slashed);
        info!(
            "[slash_double_signer] {:?} is slashed: {:?}",
            who!(who),
            slashed
        );

        // Force the double signer to be inactive.
        <xaccounts::IntentionPropertiesOf<T>>::mutate(who, |props| {
            props.is_active = false;
            props.last_inactive_since = <system::Module<T>>::block_number();
            info!("[slash_double_signer] force {:?} to be inactive", who!(who));
        });

        Self::try_reset_validators_given_double_signer(who);

        Self::deposit_event(RawEvent::SlashDoubleSigner(
            fst_height,
            snd_height,
            slot,
            who.clone(),
            slashed,
        ));
    }
}
