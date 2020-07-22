mod der;
mod error;
mod scalar;

use frame_support::dispatch::DispatchResult;
use sp_core::ecdsa::Public;
use sp_io::{crypto::secp256k1_ecdsa_recover_compressed, EcdsaVerifyError};
use sp_std::{convert::TryFrom, result};

use btc_chain::Transaction;
use btc_keys::Message;
use btc_primitives::Bytes;
use btc_script::{Script, SignatureVersion, TransactionInputSigner};

pub use error::Secp256k1Error;
use scalar::Scalar;

use crate::{Error, Trait};

#[derive(Clone, Eq, PartialEq)]
/// An ECDSA signature.
pub struct Signature {
    pub r: Scalar,
    pub s: Scalar,
}
impl Signature {
    pub fn parse_der_lax(p: &[u8]) -> result::Result<Signature, Secp256k1Error> {
        let mut decoder = der::Decoder::new(p);

        decoder.read_constructed_sequence()?;
        decoder.read_seq_len_lax()?;

        let r = decoder.read_integer_lax()?;
        let s = decoder.read_integer_lax()?;

        Ok(Signature { r, s })
    }
    pub fn serialize(&self) -> [u8; 64] {
        let mut ret = [0u8; 64];
        let mut tmp = [0u8; 32];
        self.r.fill_b32(&mut tmp);
        (&mut ret[0..32]).copy_from_slice(&tmp);
        self.s.fill_b32(&mut tmp);
        (&mut ret[32..64]).copy_from_slice(&tmp);
        ret
    }
}

pub fn verify_sig_impl<T: Trait>(
    sig: &Bytes,
    pubkey: &Bytes,
    tx: &Transaction,
    script_pubkey: &Bytes,
    index: usize,
) -> DispatchResult {
    let tx_signer: TransactionInputSigner = tx.clone().into();
    // TODO WARNNING!!! when support WitnessV0, the `input_amount` must set value
    let checker = TransactionSignatureChecker::<T> {
        input_index: index,
        input_amount: 0,
        signer: tx_signer,
        _marker: Default::default(),
    };
    let sighashtype = 1; // Sighsh all
    let signature = Signature::parse_der_lax(sig).map_err(|_| Error::<T>::ConstructBadSign)?;
    let pubkey = Public::try_from(pubkey.as_slice()).map_err(|_| Error::<T>::InvalidPublicKey)?;

    let script_code: Script = script_pubkey.clone().into();
    checker.check_signature(
        &signature,
        &pubkey,
        &script_code,
        sighashtype,
        SignatureVersion::Base,
    )
}

pub struct TransactionSignatureChecker<T: Trait> {
    pub signer: TransactionInputSigner,
    pub input_index: usize,
    pub input_amount: u64,
    _marker: sp_std::marker::PhantomData<T>,
}

impl<T: Trait> TransactionSignatureChecker<T> {
    fn check_signature(
        &self,
        signature: &Signature,
        public: &Public,
        script_code: &Script,
        sighashtype: u32,
        version: SignatureVersion,
    ) -> DispatchResult {
        let hash = self.signer.signature_hash(
            self.input_index,
            self.input_amount,
            script_code,
            version,
            sighashtype,
        );
        self.verify_signature(signature, public, &hash)
    }

    fn verify_signature(
        &self,
        signature: &Signature,
        pubkey: &Public,
        hash: &Message,
    ) -> DispatchResult {
        // public.verify(hash, signature).unwrap_or(false)
        let mut sig: [u8; 65] = [0; 65];
        (&mut sig[0..64]).copy_from_slice(&signature.serialize());

        fn convert<T: Trait>(e: EcdsaVerifyError) -> Error<T> {
            match e {
                EcdsaVerifyError::BadRS | EcdsaVerifyError::BadV => Error::<T>::ConstructBadSign,
                EcdsaVerifyError::BadSignature => Error::<T>::BadSignature,
            }
        }

        // try recover id 0:
        sig[64] = 0;
        let recover_pub = secp256k1_ecdsa_recover_compressed(&sig, hash.as_fixed_bytes())
            .map_err(convert::<T>)?;
        if &recover_pub[..] == pubkey.as_ref() {
            return Ok(());
        }
        // try recover id 1:
        sig[64] = 1;
        let recover_pub = secp256k1_ecdsa_recover_compressed(&sig, hash.as_fixed_bytes())
            .map_err(convert::<T>)?;
        if &recover_pub[..] == pubkey.as_ref() {
            return Ok(());
        }

        Err(Error::<T>::VerifySignFailed)?
    }
}
