// Copyright 2018 Chainpool

use super::{Trait, Transaction, TransactionOutput, TransactionInput, UTXOStorage, OutPoint,
            TxStorage, Bytes, ReceiveAddress, RedeemScript, Result, Script, TxProposal,
            CandidateTx};
use super::builder::Builder;
use super::keys::{Address, Type, Public};
use super::{PhantomData, Vec};
use super::StorageValue;
use super::DisplayLayout;
use super::{SignatureChecker, TransactionSignatureChecker, TransactionInputSigner,
            SignatureVersion};


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
pub fn handle_proposal<T: Trait>(tx: Transaction, who: &T::AccountId) -> Result {
    let redeem_script: Script = if let Some(redeem) = <RedeemScript<T>>::get() {
        redeem.into()
    } else {
        return Err("should set redeem script first");
    };
    let script: Script = tx.inputs[0].script_sig.clone().into();
    let (sigs, dem) = if let Ok((sigs, dem)) = script.extract_multi_scriptsig() {
        (sigs, dem)
    } else {
        return Err("InvalidSignature");
    };
    if redeem_script != dem {
        return Err("redeem script not equail");
    }

    let candidate = if let Some(candidate) = <TxProposal<T>>::get() {
        candidate
    } else {
        return Err("No candidate tx");
    };

    let lenth = candidate.proposer.len() + 1;
    if lenth != sigs.len() {
        return Err("sigs lenth not right");
    }

    let spent_tx =
        if let Some(spent_tx) = <TxStorage<T>>::find_tx(&tx.inputs[0].previous_output.hash) {
            spent_tx
        } else {
            return Err("Can't find this input UTXO");
        };
    let output = &spent_tx.outputs[tx.inputs[0].previous_output.index as usize];
    let (keys, siglen, _keylen) = script.parse_redeem_script().unwrap();
    for sig in sigs.clone() {
        let mut verify = false;
        for key in keys.clone() {
            if verify_sign(&sig, &key, &tx, output) {
                verify = true;
                break;
            }
        }
        if verify == false {
            return Err("Verify sign error");
        }
    }
    let mut proposer = candidate.proposer.clone();
    proposer.push(who.clone());
    <TxProposal<T>>::put(&CandidateTx {
        proposer: proposer,
        tx: tx,
        perfection: sigs.len() == siglen as usize,
        block_hash: Default::default(),
    });

    Ok(())
}

pub struct Proposal<T: Trait>(PhantomData<T>);

impl<T: Trait> Proposal<T> {
    pub fn create_proposal(address: Vec<(Address, u64)>, fee: u64) -> Result {
        if None != <TxProposal<T>>::get() {
            return Err(
                "There are candidates to reflect that the transaction is being processed",
            );
        }

        let mut tx = Transaction {
            version: 1,
            inputs: Vec::new(),
            outputs: Vec::new(),
            lock_time: 0,
        };
        let mut out_balance: u64 = 0;
        for i in 0..address.len() {
            let script = match address[i].0.kind {
                Type::P2PKH => Builder::build_p2pkh(&address[i].0.hash),
                Type::P2SH => Builder::build_p2sh(&address[i].0.hash),
            };
            tx.outputs.push(TransactionOutput {
                value: address[i].1,
                script_pubkey: script.into(),
            });
            out_balance += address[i].1;
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
            let receive_address: keys::Address = if let Some(h) = <ReceiveAddress<T>>::get() {
                h
            } else {
                return Err("should set RECEIVE_ADDRESS first");
            };
            let script = match receive_address.kind {
                Type::P2PKH => Builder::build_p2pkh(&receive_address.hash),
                Type::P2SH => Builder::build_p2sh(&receive_address.hash),
            };
            tx.outputs.push(TransactionOutput {
                value: ret_balance,
                script_pubkey: script.into(),
            });
        }
        <TxProposal<T>>::put(&CandidateTx {
            proposer: Vec::new(),
            tx: tx,
            perfection: false,
            block_hash: Default::default(),
        });

        Ok(())
    }
}
