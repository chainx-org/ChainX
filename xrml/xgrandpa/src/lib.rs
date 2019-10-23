// Copyright 2017-2018 Parity Technologies (UK) Ltd.
// Copyright 2018-2019 Chainpool.
// This file is part of Substrate.

// Substrate is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Substrate is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Substrate.  If not, see <http://www.gnu.org/licenses/>.

//! GRANDPA Consensus module for runtime.
//!
//! This manages the GRANDPA authority set ready for the native code.
//! These authorities are only for GRANDPA finality, not for consensus overall.
//!
//! In the future, it will also handle misbehavior reports, and on-chain
//! finality notifications.
//!
//! For full integration with GRANDPA, the `GrandpaApi` should be implemented.
//! The necessary items are re-exported via the `fg_primitives` crate.

#![cfg_attr(not(feature = "std"), no_std)]

// Substrate
// re-export since this is necessary for `impl_apis` in runtime.
pub use fg_primitives;
use fg_primitives::ScheduledChange;
use parity_codec::{Encode, KeyedVec};
use primitives::traits::{Convert, CurrentHeight, SaturatedConversion};
use rstd::prelude::*;
use substrate_primitives::ed25519::Public as AuthorityId;
use substrate_primitives::storage::well_known_keys;
use support::storage::unhashed::StorageVec;
use support::{decl_event, decl_module, decl_storage, dispatch::Result, storage, StorageValue};
use system::ensure_signed;

// ChainX
use xsupport::{debug, warn};

mod mock;
mod tests;
pub mod types;

pub use self::types::{Log, OldStoredPendingChange, RawLog, StoredPendingChange};

struct AuthorityStorageVec<S: parity_codec::Codec + Default>(rstd::marker::PhantomData<S>);

impl<S: parity_codec::Codec + Default> StorageVec for AuthorityStorageVec<S> {
    type Item = (S, u64);
    const PREFIX: &'static [u8] = crate::fg_primitives::well_known_keys::AUTHORITY_PREFIX;
}

/// Logs which can be scanned by GRANDPA for authorities change events.
pub trait GrandpaChangeSignal<N> {
    /// Try to cast the log entry as a contained signal.
    fn as_signal(&self) -> Option<ScheduledChange<N>>;
    /// Try to cast the log entry as a contained forced signal.
    fn as_forced_signal(&self) -> Option<(N, ScheduledChange<N>)>;
}

impl<N, SessionKey> GrandpaChangeSignal<N> for RawLog<N, SessionKey>
where
    N: Clone,
    SessionKey: Clone + Into<AuthorityId>,
{
    fn as_signal(&self) -> Option<ScheduledChange<N>> {
        RawLog::as_signal(self).map(|(delay, next_authorities)| ScheduledChange {
            delay,
            next_authorities: next_authorities
                .iter()
                .cloned()
                .map(|(k, w)| (k.into(), w))
                .collect(),
        })
    }

    fn as_forced_signal(&self) -> Option<(N, ScheduledChange<N>)> {
        RawLog::as_forced_signal(self).map(|(median, delay, next_authorities)| {
            (
                median,
                ScheduledChange {
                    delay,
                    next_authorities: next_authorities
                        .iter()
                        .cloned()
                        .map(|(k, w)| (k.into(), w))
                        .collect(),
                },
            )
        })
    }
}

pub trait Trait: system::Trait + consensus::Trait {
    /// Type for all log entries of this module.
    type Log: From<Log<Self>> + Into<system::DigestItemOf<Self>>;

    /// The event type of this module.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

decl_event!(
	pub enum Event<T> where <T as consensus::Trait>::SessionKey {
		/// New authority set has been applied.
		NewAuthorities(Vec<(SessionKey, u64)>),
	}
);

decl_storage! {
    trait Store for Module<T: Trait> as GrandpaFinality {
        // Pending change: (signalled at, scheduled change).
        PendingChange get(pending_change): Option<StoredPendingChange<T::BlockNumber, T::SessionKey>>;
        // next block number where we can force a change.
        NextForced get(next_forced): Option<T::BlockNumber>;
        SessionsPerGrandpa get(sessions_per_grandpa) config(): u32 = 12;
    }
    add_extra_genesis {
        config(authorities): Vec<(T::SessionKey, u64)>;

        build(|storage: &mut primitives::StorageOverlay, _: &mut primitives::ChildrenStorageOverlay, config: &GenesisConfig<T>| {
            let auth_count = config.authorities.len() as u32;
            config.authorities.iter().enumerate().for_each(|(i, v)| {
                storage.insert((i as u32).to_keyed_vec(
                    crate::fg_primitives::well_known_keys::AUTHORITY_PREFIX),
                    v.encode()
                );
            });
            storage.insert(
                crate::fg_primitives::well_known_keys::AUTHORITY_COUNT.to_vec(),
                auth_count.encode(),
            );
        });
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {

        /// Set the number of sessions in an grandpa.
        fn set_sessions_per_era(per_grandpa: u32) {
            <SessionsPerGrandpa<T>>::put(per_grandpa);
        }

        fn set_finalized_height(height: T::BlockNumber) {
            storage::unhashed::put(well_known_keys::AURA_FINALIZE, &height.encode());
        }

        fn deposit_event<T>() = default;

        /// Report some misbehaviour.
        fn report_misbehavior(origin, _report: Vec<u8>) {
            ensure_signed(origin)?;
            // FIXME: https://github.com/paritytech/substrate/issues/1112
        }

        fn on_finalize(_block_number: T::BlockNumber) {
            /*if let Some(pending_change) = <PendingChange<T>>::get() {
                debug!("--current block number:{:?}, pending_change scheduled_at:{:?}", block_number, pending_change.scheduled_at);
                if block_number == pending_change.scheduled_at {
                    if let Some(median) = pending_change.forced {
                        Self::deposit_log(RawLog::ForcedAuthoritiesChangeSignal(
                            median,
                            pending_change.delay,
                            pending_change.next_authorities.clone(),
                        ));
                    } else {
                        Self::deposit_log(RawLog::AuthoritiesChangeSignal(
                            pending_change.delay,
                            pending_change.next_authorities.clone(),
                        ));
                    }
                }

                if block_number >= pending_change.scheduled_at + pending_change.delay {
                    Self::deposit_event(
                        RawEvent::NewAuthorities(pending_change.next_authorities.clone())
                    );
                    <AuthorityStorageVec<<T as consensus::Trait>::SessionKey>>::set_items(pending_change.next_authorities);
                    <PendingChange<T>>::kill();
                }
            }*/
        }
    }
}

impl<T: Trait> Module<T> {
    /// Get the current set of authorities, along with their respective weights.
    pub fn grandpa_authorities() -> Vec<(T::SessionKey, u64)> {
        let tmp = <AuthorityStorageVec<T::SessionKey>>::items();
        tmp
    }

    /// Schedule a change in the authorities.
    ///
    /// The change will be applied at the end of execution of the block
    /// `in_blocks` after the current block. This value may be 0, in which
    /// case the change is applied at the end of the current block.
    ///
    /// If the `forced` parameter is defined, this indicates that the current
    /// set has been synchronously determined to be offline and that after
    /// `in_blocks` the given change should be applied. The given block number
    /// indicates the median last finalized block number and it should be used
    /// as the canon block when starting the new grandpa voter.
    ///
    /// No change should be signalled while any change is pending. Returns
    /// an error if a change is already pending.
    pub fn schedule_change(
        next_authorities: Vec<(<T as consensus::Trait>::SessionKey, u64)>,
        in_blocks: T::BlockNumber,
        forced: Option<T::BlockNumber>,
    ) -> Result {
        if Self::pending_change().is_none() {
            let scheduled_at = system::ChainContext::<T>::default().current_height();

            if let Some(_) = forced {
                if Self::next_forced().map_or(false, |next| next > scheduled_at) {
                    return Err("Cannot signal forced change so soon after last.");
                }

                // only allow the next forced change when twice the window has passed since
                // this one.
                <NextForced<T>>::put(
                    scheduled_at + in_blocks * T::BlockNumber::saturated_from::<u64>(2),
                );
            }

            <PendingChange<T>>::put(StoredPendingChange {
                delay: in_blocks,
                scheduled_at,
                next_authorities,
                forced,
            });

            Ok(())
        } else {
            Err("Attempt to signal GRANDPA change with one already pending.")
        }
    }

    #[allow(dead_code)]
    /// Deposit one of this module's logs.
    fn deposit_log(log: Log<T>) {
        <system::Module<T>>::deposit_log(<T as Trait>::Log::from(log).into());
    }
}

impl<T: Trait> Module<T>
where
    AuthorityId: core::convert::From<<T as consensus::Trait>::SessionKey>,
{
    /// See if the digest contains any standard scheduled change.
    pub fn scrape_digest_change(log: &Log<T>) -> Option<ScheduledChange<T::BlockNumber>> {
        <Log<T> as GrandpaChangeSignal<T::BlockNumber>>::as_signal(log)
    }

    /// See if the digest contains any forced scheduled change.
    pub fn scrape_digest_forced_change(
        log: &Log<T>,
    ) -> Option<(T::BlockNumber, ScheduledChange<T::BlockNumber>)> {
        <Log<T> as GrandpaChangeSignal<T::BlockNumber>>::as_forced_signal(log)
    }
}

/// Helper for authorities being synchronized with the general session authorities.
///
/// This is not the only way to manage an authority set for GRANDPA, but it is
/// a convenient one. When this is used, no other mechanism for altering authority
/// sets should be.
pub struct SyncedAuthorities<T>(::rstd::marker::PhantomData<T>);

// FIXME: remove when https://github.com/rust-lang/rust/issues/26925 is fixed
impl<T> Default for SyncedAuthorities<T> {
    fn default() -> Self {
        SyncedAuthorities(::rstd::marker::PhantomData)
    }
}

impl<X, T> xsession::OnSessionChange<X> for SyncedAuthorities<T>
where
    T: Trait,
    T: xsession::Trait,
    <T as xsession::Trait>::ConvertAccountIdToSessionKey:
        Convert<<T as system::Trait>::AccountId, <T as consensus::Trait>::SessionKey>,
{
    fn on_session_change() {
        let total_missed = <xsession::SessionTotalMissedBlocksCount<T>>::take();
        let finalize_threshold = <xsession::Module<T>>::length().saturated_into::<u64>() / 3;
        debug!(
            "[on_session_change of grandpa] total_missed: {:?}, finalize_threshold: {:?}",
            total_missed, finalize_threshold
        );
        if u64::from(total_missed) < finalize_threshold {
            let height = system::ChainContext::<T>::default().current_height();
            storage::unhashed::put(well_known_keys::AURA_FINALIZE, &height);
        } else {
            warn!("[on_session_change of grandpa] So many missed blocks({:?}) that grandpa fail to finalize.", total_missed);
        }
        /*use primitives::traits::Zero;

        let sessions_per_grandpa = <Module<T>>::sessions_per_grandpa() as u64;
        if <xsession::Module<T>>::current_index().as_() % sessions_per_grandpa
            != sessions_per_grandpa - 1
        {
            return;
        }

        let next_authorities = <xsession::Module<T>>::validators()
            .into_iter()
            .map(|(account_id, weight)| {
                if let Some(session_key) = <xsession::Module<T>>::next_key_for(account_id.clone()) {
                    (session_key, weight)
                } else {
                    (T::ConvertAccountIdToSessionKey::convert(account_id), weight)
                }
            })
            .collect::<Vec<(<T as consensus::Trait>::SessionKey, u64)>>();

        // instant changes
        let last_authorities = <Module<T>>::grandpa_authorities();
        info!(
            "--on_session_change, last_authorities:{:?}",
            last_authorities
        );
        if next_authorities != last_authorities {
            let _ = <Module<T>>::schedule_change(next_authorities, Zero::zero(), None);
        }*/
    }
}

impl<T> finality_tracker::OnFinalizationStalled<T::BlockNumber> for SyncedAuthorities<T>
where
    T: Trait,
    T: xsession::Trait,
    T: finality_tracker::Trait,
    <T as xsession::Trait>::ConvertAccountIdToSessionKey:
        Convert<<T as system::Trait>::AccountId, <T as consensus::Trait>::SessionKey>,
{
    fn on_stalled(further_wait: T::BlockNumber) {
        // when we record old authority sets, we can use `finality_tracker::median`
        // to figure out _who_ failed. until then, we can't meaningfully guard
        // against `next == last` the way that normal session changes do.
        let next_authorities = <xsession::Module<T>>::validators()
            .into_iter()
            .map(|(account_id, weight)| {
                if let Some(session_key) = <xsession::Module<T>>::next_key_for(account_id.clone()) {
                    (session_key, weight)
                } else {
                    (T::ConvertAccountIdToSessionKey::convert(account_id), weight)
                }
            })
            .collect::<Vec<(<T as consensus::Trait>::SessionKey, u64)>>();

        let median = <finality_tracker::Module<T>>::median();

        // schedule a change for `further_wait` blocks.
        let _ = <Module<T>>::schedule_change(next_authorities, further_wait, Some(median));
    }
}
