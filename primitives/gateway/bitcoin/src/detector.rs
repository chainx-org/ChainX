// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use sp_std::fmt::Debug;

use chainx_primitives::ReferralId;
use xp_logging::{debug, warn};

use light_bitcoin::{
    chain::Transaction,
    keys::{Address, Network},
    primitives::hash_rev,
    script::Script,
};

use crate::{
    types::{DepositInfo, MetaTxType},
    utils::{
        extract_addr_from_transaction, extract_opreturn_data, extract_output_addr, is_trustee_addr,
    },
};

/// A helper struct for detecting the bitcoin transaction type.
pub struct BtcTxTypeDetector {
    // The bitcoin network type (mainnet/testnet)
    network: Network,
    // The minimum deposit value of the `Deposit` transaction.
    min_deposit: u64,
    // (current hot trustee address, current cold trustee address)
    current_trustee_pair: (Address, Address),
    // (previous hot trustee address, previous cold trustee address)
    previous_trustee_pair: Option<(Address, Address)>,
}

impl BtcTxTypeDetector {
    /// Create a new bitcoin tx type detector.
    pub fn new(
        network: Network,
        min_deposit: u64,
        current_trustee_pair: (Address, Address),
        previous_trustee_pair: Option<(Address, Address)>,
    ) -> Self {
        Self {
            network,
            min_deposit,
            current_trustee_pair,
            previous_trustee_pair,
        }
    }

    /// Detect X-BTC transaction type.
    ///
    /// We would try to detect `Withdrawal`/`TrusteeTransition`/`HotAndCold` transaction types
    /// when passing `Some(prev_tx)`, otherwise, we would just detect `Deposit` type.
    ///
    /// If the transaction type is `Deposit`, and parsing opreturn successfully,
    /// we would use opreturn data as account info, otherwise, we would use input_addr, which is
    /// extracted from `prev_tx`, as the account.
    ///
    /// If we meet with `prev_tx`, we would parse tx's inputs/outputs into Option<Address>.
    /// e.g. notice the relay tx only has the first input
    ///        _________
    ///  addr |        | Some(addr)
    ///       |   tx   | Some(addr)
    ///       |________| None (OP_RETURN or something unknown)
    pub fn detect_transaction_type<AccountId, Extractor>(
        &self,
        tx: &Transaction,
        prev_tx: Option<&Transaction>,
        extract_account: Extractor,
    ) -> MetaTxType<AccountId>
    where
        AccountId: Debug,
        Extractor: Fn(&[u8]) -> Option<(AccountId, Option<ReferralId>)>,
    {
        // extract input addr from the output of previous transaction
        let input_addr = prev_tx.and_then(|prev_tx| {
            let outpoint = &tx.inputs[0].previous_output;
            extract_addr_from_transaction(prev_tx, outpoint.index as usize, self.network)
        });

        // detect X-BTC `Withdrawal`/`HotAndCold`/`TrusteeTransition` transaction
        if let Some(input_addr) = input_addr {
            let all_outputs_is_trustee = tx
                .outputs
                .iter()
                .map(|output| extract_output_addr(output, self.network).unwrap_or_default())
                .all(|addr| is_trustee_addr(addr, self.current_trustee_pair));

            if is_trustee_addr(input_addr, self.current_trustee_pair) {
                return if all_outputs_is_trustee {
                    MetaTxType::HotAndCold
                } else {
                    MetaTxType::Withdrawal
                };
            }
            if let Some(previous_trustee_pair) = self.previous_trustee_pair {
                if is_trustee_addr(input_addr, previous_trustee_pair) && all_outputs_is_trustee {
                    return MetaTxType::TrusteeTransition;
                }
            }
        }
        // detect X-BTC `Deposit` transaction
        self.detect_deposit_transaction_type(tx, input_addr, extract_account)
    }

    /// Detect X-BTC `Deposit` transaction
    /// The outputs of X-BTC `Deposit` transaction must be in the following
    /// format (ignore the outputs order):
    /// - 2 outputs (e.g. txid=e3639343ca806fe3bf2513971b79130eef88aa05000ce538c6af199dd8ef3ca7):
    ///   --> X-BTC hot trustee address (deposit value)
    ///   --> Null data transaction
    /// - 3 outputs (e.g. txid=003e7e005b172fe0046fd06a83679fbcdc5e3dd64c8ef9295662a463dea486aa):
    ///   --> X-BTC hot trustee address (deposit value)
    ///   --> Change address (don't care)
    ///   --> Null data transaction
    pub fn detect_deposit_transaction_type<AccountId, Extractor>(
        &self,
        tx: &Transaction,
        input_addr: Option<Address>,
        extract_account: Extractor,
    ) -> MetaTxType<AccountId>
    where
        AccountId: Debug,
        Extractor: Fn(&[u8]) -> Option<(AccountId, Option<ReferralId>)>,
    {
        // The numbers of deposit transaction outputs must be 2 or 3.
        if tx.outputs.len() != 2 && tx.outputs.len() != 3 {
            warn!(
                "[detect_deposit_transaction_type] Receive a deposit tx ({:?}), but outputs len ({}) is not 2 or 3, drop it",
                hash_rev(tx.hash()), tx.outputs.len()
            );
            return MetaTxType::Irrelevance;
        }

        let (op_return, deposit_value) =
            self.parse_deposit_transaction_outputs(tx, extract_account);
        // check if deposit value is greater than minimum deposit value.
        if deposit_value >= self.min_deposit {
            // if opreturn.is_none() && input_addr.is_none()
            // we still think it's a deposit tx, but won't process it.
            MetaTxType::Deposit(DepositInfo {
                deposit_value,
                op_return,
                input_addr,
            })
        } else {
            warn!(
                "[detect_deposit_transaction_type] Receive a deposit tx ({:?}), but deposit value ({:}) is too low, drop it",
                hash_rev(tx.hash()), deposit_value,
            );
            MetaTxType::Irrelevance
        }
    }

    /// Parse the outputs of X-BTC `Deposit` transaction.
    /// Return the account info that extracted from OP_RETURN data and the deposit value.
    pub fn parse_deposit_transaction_outputs<AccountId, Extractor>(
        &self,
        tx: &Transaction,
        extract_account: Extractor,
    ) -> (Option<(AccountId, Option<ReferralId>)>, u64)
    where
        AccountId: Debug,
        Extractor: Fn(&[u8]) -> Option<(AccountId, Option<ReferralId>)>,
    {
        // only handle first valid opreturn with account info, other opreturn would be dropped
        let opreturn_script = tx
            .outputs
            .iter()
            .map(|output| Script::new(output.script_pubkey.clone()))
            .filter(|script| script.is_null_data_script())
            .take(1)
            .next();
        debug!(
            "[parse_deposit_transaction_outputs] opreturn_script:{:?}",
            opreturn_script
        );

        let account_info = opreturn_script
            .and_then(|script| extract_opreturn_data(&script))
            .and_then(|opreturn| extract_account(&opreturn));
        let mut deposit_value = 0;

        let (hot_addr, _) = self.current_trustee_pair;
        for output in &tx.outputs {
            // extract destination address from the script of output.
            if let Some(dest_addr) = extract_output_addr(output, self.network) {
                // check if the script address of the output is the hot trustee address
                if dest_addr.hash == hot_addr.hash {
                    deposit_value += output.value;
                }
            }
        }
        debug!(
            "[parse_deposit_transaction_outputs] account_info:{:?}, deposit_value:{}",
            account_info, deposit_value
        );
        (account_info, deposit_value)
    }
}
