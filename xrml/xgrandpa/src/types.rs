// Copyright 2018-2019 Chainpool.

use parity_codec::{Decode, Encode, Input};
#[cfg(feature = "std")]
use serde_derive::Serialize;

// Substrate
use rstd::vec::Vec;

/// The log type of this crate, projected from module trait type.
pub type Log<T> = RawLog<<T as system::Trait>::BlockNumber, <T as consensus::Trait>::SessionKey>;

/// A logs in this module.
#[derive(Encode, Decode, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "std", derive(Serialize, Debug))]
pub enum RawLog<N, SessionKey> {
    /// Authorities set change has been signalled. Contains the new set of authorities
    /// and the delay in blocks _to finalize_ before applying.
    AuthoritiesChangeSignal(N, Vec<(SessionKey, u64)>),
    /// A forced authorities set change. Contains in this order: the median last
    /// finalized block when the change was signaled, the delay in blocks _to import_
    /// before applying and the new set of authorities.
    ForcedAuthoritiesChangeSignal(N, N, Vec<(SessionKey, u64)>),
}

impl<N: Clone, SessionKey> RawLog<N, SessionKey> {
    /// Try to cast the log entry as a contained signal.
    pub fn as_signal(&self) -> Option<(N, &[(SessionKey, u64)])> {
        match *self {
            RawLog::AuthoritiesChangeSignal(ref delay, ref signal) => Some((delay.clone(), signal)),
            RawLog::ForcedAuthoritiesChangeSignal(_, _, _) => None,
        }
    }

    /// Try to cast the log entry as a contained forced signal.
    #[allow(clippy::type_complexity)]
    pub fn as_forced_signal(&self) -> Option<(N, N, &[(SessionKey, u64)])> {
        match *self {
            RawLog::ForcedAuthoritiesChangeSignal(ref median, ref delay, ref signal) => {
                Some((median.clone(), delay.clone(), signal))
            }
            RawLog::AuthoritiesChangeSignal(_, _) => None,
        }
    }
}

/// A stored pending change, old format.
// TODO: remove shim
// https://github.com/paritytech/substrate/issues/1614
#[derive(Encode, Decode)]
pub struct OldStoredPendingChange<N, SessionKey> {
    /// The block number this was scheduled at.
    pub scheduled_at: N,
    /// The delay in blocks until it will be applied.
    pub delay: N,
    /// The next authority set.
    pub next_authorities: Vec<(SessionKey, u64)>,
}

/// A stored pending change.
#[derive(Encode)]
pub struct StoredPendingChange<N, SessionKey> {
    /// The block number this was scheduled at.
    pub scheduled_at: N,
    /// The delay in blocks until it will be applied.
    pub delay: N,
    /// The next authority set.
    pub next_authorities: Vec<(SessionKey, u64)>,
    /// If defined it means the change was forced and the given block number
    /// indicates the median last finalized block when the change was signaled.
    pub forced: Option<N>,
}

impl<N: Decode, SessionKey: Decode> Decode for StoredPendingChange<N, SessionKey> {
    fn decode<I: Input>(value: &mut I) -> Option<Self> {
        let old = OldStoredPendingChange::decode(value)?;
        let forced = <Option<N>>::decode(value).unwrap_or(None);

        Some(StoredPendingChange {
            scheduled_at: old.scheduled_at,
            delay: old.delay,
            next_authorities: old.next_authorities,
            forced,
        })
    }
}
