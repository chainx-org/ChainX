// Copyright 2018-2019 Chainpool.
//! Staking manager: Periodically determines the best set of validators.

#![cfg_attr(not(feature = "std"), no_std)]

// Substrate
use substrate_primitives::H512;

use rstd::prelude::*;
use rstd::result;
use support::{decl_event, decl_module, decl_storage, dispatch::Result, StorageMap, StorageValue};
use system::ensure_signed;

// ChainX
use xsupport::{debug, ensure_with_errorlog, error, info, warn};
#[cfg(feature = "std")]
use xsupport::{u8array_to_hex, who};

pub trait Trait: xstaking::Trait {
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
    type CheckHeader: CheckHeader<
        <Self as consensus::Trait>::SessionKey,
        <Self as system::Trait>::BlockNumber,
    >;
}

pub trait CheckHeader<SessionKey, BlockNumber: Default> {
    /// Check if the header is signed by the given signer.
    fn check_header(
        signer: &SessionKey,
        first: &(RawHeader, u64, H512),
        second: &(RawHeader, u64, H512),
    ) -> result::Result<(BlockNumber, BlockNumber), &'static str>;
}

impl<SessionKey, BlockNumber: Default> CheckHeader<SessionKey, BlockNumber> for () {
    fn check_header(
        _signer: &SessionKey,
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
            double_signer: T::SessionKey,
            fst_header: (RawHeader, u64, H512),
            snd_header: (RawHeader, u64, H512)
        ) -> Result {
            let who = ensure_signed(origin)?;
            ensure_with_errorlog!(
                Self::fishermen().contains(&who),
                "Only the fisherman can report the double signer.",
                "Only the fisherman can report the double signer|current fishermen:{:?}|sender{:?}", Self::fishermen(), who
            );

            debug!("report double signer|signer:{:?}|first:({:?}, {:}, {:?})|existed:{:?}|second:({:?}, {:}, {:?})|existed:{:?}",
                double_signer,
                u8array_to_hex(&fst_header.0), fst_header.1, fst_header.2, <Reported<T>>::get(&fst_header.2).is_none(),
                u8array_to_hex(&snd_header.0), snd_header.1, snd_header.2, <Reported<T>>::get(&snd_header.2).is_none(),
            );

            ensure_with_errorlog!(
                <Reported<T>>::get(&fst_header.2).is_none() || <Reported<T>>::get(&snd_header.2).is_none(),
                "The double signer at this height has been reported already.",
                "The double signer at this height has been reported already|fst_sig:{:?}|snd_sig:{:?}", fst_header.2, snd_header.2
            );

            let (fst_height, snd_height) = T::CheckHeader::check_header(&double_signer, &fst_header, &snd_header)?;

            Self::slash(&double_signer, fst_height, snd_height, fst_header.1);

            <Reported<T>>::insert(&fst_header.2, ());
            <Reported<T>>::insert(&snd_header.2, ());
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
        pub Reported get(reported): map H512 => Option<()>;

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
    /// Actually slash the double signer.
    fn apply_slash(who: &T::AccountId) -> T::Balance {
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

        slashed
    }

    fn slash(
        double_signed_key: &T::SessionKey,
        fst_height: T::BlockNumber,
        snd_height: T::BlockNumber,
        slot: u64,
    ) {
        if let Some(who) = xsession::Module::<T>::account_id_for(double_signed_key) {
            if !xstaking::Module::<T>::is_intention(&who) {
                warn!("[slash] Try to slash only to find that it is not an intention|session_key:{:?}|accountid:{:?}", double_signed_key, who);
                return;
            }

            let slashed = Self::apply_slash(&who);

            Self::deposit_event(RawEvent::SlashDoubleSigner(
                fst_height, snd_height, slot, who, slashed,
            ));
        } else {
            error!("[slash] Cannot find the account id given the double signed session key|session_key:{:?}", double_signed_key);
        }
    }
}
