// Copyright 2018 Chainpool

use super::builder::Builder;
use super::keys::{Public, Type};
use super::{
    Bytes, CandidateTx, OutPoint, Result, Script, Trait, Transaction, TransactionInput,
    TransactionOutput, TrusteeAddress, UTXOStorage,
};
use super::{PhantomData, Vec};
use super::{
    SignatureChecker, SignatureVersion, TransactionInputSigner, TransactionSignatureChecker,
};
use super::{StorageMap, StorageValue};
use runtime_primitives::traits::As;
use {Module, TxProposal, TxProposalLen};
use xrecords::{Record, RecordListOf};
use xassets::Token;

#[allow(unused)]
fn verify_sign(sign: &Bytes, pubkey: &Bytes, tx: &Transaction, output: &TransactionOutput) -> bool {
    let tx_signer: TransactionInputSigner = tx.clone().into();
    let checker = TransactionSignatureChecker {
        input_index: 0,
        input_amount: output.value,
        signer: tx_signer,
    };
    let sighashtype = 0x41; // Sighsh all
    let signature = sign.clone().take().into();
    let public = if let Ok(public) = Public::from_slice(&pubkey) {
        public
    } else {
        return false;
    };
    let script_code: Script = output.script_pubkey.clone().into();
    return checker.check_signature(
        &signature,
        &public,
        &script_code,
        sighashtype,
        SignatureVersion::Base,
    );
}

// Only support inputs 0, To do: Support every input.
pub fn handle_proposal<T: Trait>(_tx: Transaction, _who: &T::AccountId) -> Result {
    //    let mut candidate = if let Some(candidate) = <TxProposal<T>>::get() {
    //        candidate
    //    } else {
    //        return Err("No candidate tx");
    //    };
    //
    //    let redeem_script: Script = if let Some(redeem) = <RedeemScript<T>>::get() {
    //        redeem.into()
    //    } else {
    //        return Err("should set redeem script first");
    //    };
    //    let script: Script = tx.inputs[0].script_sig.clone().into();
    //    let (sigs, dem) = if let Ok((sigs, dem)) = script.extract_multi_scriptsig() {
    //        (sigs, dem)
    //    } else {
    //        return Err("InvalidSignature");
    //    };
    //    if redeem_script != dem {
    //        return Err("redeem script not equail");
    //    }
    //
    //    let lenth = candidate.proposers.len() + 1;
    //    if lenth != sigs.len() {
    //        return Err("sigs lenth not right");
    //    }
    //
    //    let spent_tx =
    //        if let Some(spent_tx) = <TxStorage<T>>::find_tx(&tx.inputs[0].previous_output.hash) {
    //            spent_tx
    //        } else {
    //            return Err("Can't find this input UTXO");
    //        };
    //    let output = &spent_tx.outputs[tx.inputs[0].previous_output.index as usize];
    //    let (keys, siglen, _keylen) = script.parse_redeem_script().unwrap();
    //    for sig in sigs.clone() {
    //        let mut verify = false;
    //        for key in keys.clone() {
    //            if verify_sign(&sig, &key, &tx, output) {
    //                verify = true;
    //                break;
    //            }
    //        }
    //        if verify == false {
    //            return Err("Verify sign error");
    //        }
    //    }
    //    let mut proposers = candidate.proposers.clone();
    //    proposers.push(who.clone());
    //
    //    candidate.tx = tx;
    //    candidate.proposers = proposers;
    //    TxProposal::<T>::put(condidate);
    Ok(())
}

pub struct Proposal<T: Trait>(PhantomData<T>);

impl<T: Trait> Proposal<T> {
    pub fn create_proposal(withdrawal_record_indexs: Vec<(T::AccountId, u32)>, fee: u64) -> Result {
        let len = Module::<T>::tx_proposal_len();
        if len > 0 {
            if let Some(last_candidate) = Module::<T>::tx_proposal(len - 1) {
                if last_candidate.confirmed == false {
                    return Err("last condidate tx has not confirmed yet");
                }
            }
        }
        let mut tx = Transaction {
            version: 1,
            inputs: Vec::new(),
            outputs: Vec::new(),
            lock_time: 0,
        };
        let mut out_balance: u64 = 0;

        let mut outs: Vec<(T::AccountId, u32)> = Vec::new();

        for index in withdrawal_record_indexs.into_iter() {
            let r: Record<Token, T::Balance, T::BlockNumber> =
                if let Some(r) = RecordListOf::<T>::get(&index) {
                    r
                } else {
                    return Err("not get record for this key");
                };
            let balance = r.balance().as_() as u64;
            let addr: keys::Address =
                Module::<T>::verify_btc_address(&r.addr()).map_err(|_| "parse addr error")?;

            let script = match addr.kind {
                Type::P2PKH => Builder::build_p2pkh(&addr.hash),
                Type::P2SH => Builder::build_p2sh(&addr.hash),
            };
            tx.outputs.push(TransactionOutput {
                value: balance,
                script_pubkey: script.into(),
            });
            outs.push(index);
            out_balance += balance;
        }

        let utxo_set = <UTXOStorage<T>>::select_utxo(out_balance + fee).unwrap();
        let mut ins_balance: u64 = 0;
        for utxo in utxo_set {
            tx.inputs.push(TransactionInput {
                previous_output: OutPoint {
                    hash: utxo.txid,
                    index: utxo.index,
                },
                script_sig: Bytes::default(),
                sequence: 0xffffffff, // SEQUENCE_FINAL
                script_witness: Vec::new(),
            });
            ins_balance += utxo.balance;
        }

        let ret_balance = ins_balance - out_balance - fee;

        if ret_balance > 0 {
            let trustee_address: keys::Address = if let Some(h) = <TrusteeAddress<T>>::get() {
                h
            } else {
                return Err("should set trustee_address first");
            };
            let script = match trustee_address.kind {
                Type::P2PKH => Builder::build_p2pkh(&trustee_address.hash),
                Type::P2SH => Builder::build_p2sh(&trustee_address.hash),
            };
            tx.outputs.push(TransactionOutput {
                value: ret_balance,
                script_pubkey: script.into(),
            });
        }

        TxProposal::<T>::insert(len, CandidateTx::new(tx, outs));
        TxProposalLen::<T>::put(len + 1);
        Ok(())
    }
}
