// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use frame_support::dispatch::DispatchResult;

use light_bitcoin::{chain::Transaction, primitives::Bytes};

use crate::types::BtcTxVerifier;
use crate::{Config, Error, Pallet};

mod recover_verifier;
mod runtime_interface {
    use super::*;
    pub fn verify_sig_impl<T: Config<I>, I: 'static>(
        _sig: &Bytes,
        _pubkey: &Bytes,
        _tx: &Transaction,
        _script_pubkey: &Bytes,
        _index: usize,
    ) -> DispatchResult {
        Err(Error::<T, I>::VerifySignFailed)?
    }
}

pub fn verify_sig<T: Config<I>, I: 'static>(
    sig: &Bytes,
    pubkey: &Bytes,
    tx: &Transaction,
    script_pubkey: &Bytes,
    index: usize,
) -> DispatchResult {
    match Pallet::<T, I>::verifier() {
        BtcTxVerifier::Recover => {
            recover_verifier::verify_sig_impl::<T, I>(sig, pubkey, tx, script_pubkey, index)
        }
        BtcTxVerifier::RuntimeInterface => {
            runtime_interface::verify_sig_impl::<T, I>(sig, pubkey, tx, script_pubkey, index)
        }
        #[cfg(any(feature = "runtime-benchmarks", test))]
        BtcTxVerifier::Test => Ok(()),
    }
}
