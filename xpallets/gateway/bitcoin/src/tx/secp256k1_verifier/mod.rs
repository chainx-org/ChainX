use frame_support::dispatch::DispatchResult;

use light_bitcoin::{chain::Transaction, primitives::Bytes};

use crate::types::BtcTxVerifier;
use crate::{Error, Module, Trait};

mod recover_verifier;

mod runtime_interface {
    use super::*;
    pub fn verify_sig_impl<T: Trait>(
        _sig: &Bytes,
        _pubkey: &Bytes,
        _tx: &Transaction,
        _script_pubkey: &Bytes,
        _index: usize,
    ) -> DispatchResult {
        Err(Error::<T>::VerifySignFailed)?
    }
}

pub fn verify_sig<T: Trait>(
    sig: &Bytes,
    pubkey: &Bytes,
    tx: &Transaction,
    script_pubkey: &Bytes,
    index: usize,
) -> DispatchResult {
    match Module::<T>::verifier() {
        BtcTxVerifier::Recover => {
            recover_verifier::verify_sig_impl::<T>(sig, pubkey, tx, script_pubkey, index)
        }
        BtcTxVerifier::RuntimeInterface => {
            runtime_interface::verify_sig_impl::<T>(sig, pubkey, tx, script_pubkey, index)
        }
    }
}
