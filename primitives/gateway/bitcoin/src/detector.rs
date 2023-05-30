// Copyright 2019-2023 ChainX Project Authors. Licensed under GPL-3.0.

use sp_std::fmt::Debug;

use chainx_primitives::ReferralId;
use frame_support::log::{debug, warn};

use light_bitcoin::{
    chain::Transaction,
    keys::{Address, Network},
    primitives::hash_rev,
    script::Script,
};

use crate::{
    types::{BtcDepositInfo, BtcTxMetaType, OpReturnAccount, TrusteePair},
    utils::{
        extract_addr_from_transaction, extract_opreturn_data, extract_output_addr, is_trustee_addr,
    },
};

/// A helper struct for detecting the bitcoin transaction type.
#[derive(Clone, Debug)]
pub struct BtcTxTypeDetector {
    // The bitcoin network type (mainnet/testnet)
    network: Network,
    // The minimum deposit value of the `Deposit` transaction.
    min_deposit: u64,
}

impl BtcTxTypeDetector {
    /// Create a new bitcoin tx type detector.
    pub fn new(network: Network, min_deposit: u64) -> Self {
        Self {
            network,
            min_deposit,
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
    // If we meet with `prev_tx`, we would parse tx's inputs/outputs into Option<Address>.
    // e.g. notice the relay tx only has the first input
    //        _________
    //  addr |        | Some(addr)
    //       |   tx   | Some(addr)
    //       |________| None (OP_RETURN or something unknown)
    pub fn detect_transaction_type<AccountId, Extractor>(
        &self,
        tx: &Transaction,
        prev_tx: Option<&Transaction>,
        extract_account: Extractor,
        current_trustee_pair: TrusteePair,
        prev_trustee_pair: Option<TrusteePair>,
    ) -> BtcTxMetaType<AccountId>
    where
        AccountId: Debug,
        Extractor: Fn(&[u8]) -> Option<(OpReturnAccount<AccountId>, Option<ReferralId>)>,
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
                .all(|addr| is_trustee_addr(addr, current_trustee_pair));

            if is_trustee_addr(input_addr, current_trustee_pair) {
                return if all_outputs_is_trustee {
                    BtcTxMetaType::HotAndCold
                } else {
                    // outputs contains other address, it's a user address, thus it's a withdrawal
                    BtcTxMetaType::Withdrawal
                };
            }
            if let Some(prev_trustee_pair) = prev_trustee_pair {
                // inputs: previous trustee cold address --> outputs: current trustee cold address
                let all_outputs_is_cold_address = tx
                    .outputs
                    .iter()
                    .map(|output| extract_output_addr(output, self.network).unwrap_or_default())
                    .all(|addr| addr.hash == current_trustee_pair.1.hash);
                if input_addr.hash == prev_trustee_pair.1.hash && all_outputs_is_cold_address {
                    return BtcTxMetaType::TrusteeTransition;
                }
            }
        }

        // detect X-BTC `Deposit` transaction
        self.detect_deposit_transaction_type(tx, input_addr, extract_account, current_trustee_pair)
    }

    /// Detect X-BTC `Deposit` transaction
    ///
    /// # Format
    ///
    /// The outputs of X-BTC `Deposit` transaction must be in the following
    /// format (ignore the outputs order):
    ///
    /// - 1 outputs (e.g. txid=987f12d3ebfaf875c19553bf5e1d4277f24d2be64cbdd8942174d1cd232fdaf8):
    ///   - X-BTC hot trustee address (deposit value)
    ///
    ///   **Precondition**: sent a correct deposit transaction with the same BTC address before.
    ///   **Solution**: call `push_transaction` with the previous transaction of the transaction
    ///   with 1 outputs to get the BTC address.
    ///
    /// - 2 outputs (e.g. txid=7cd6d752c51100c7bc51657433b52facd04a0fea203b8e7776e6420c477912c2):
    ///   - X-BTC hot trustee address (deposit value)
    ///   - Change address (don't care)
    ///
    ///   **Solution**: send a correct deposit transaction with the same BTC address.
    ///
    /// - 2 outputs (e.g. txid=e3639343ca806fe3bf2513971b79130eef88aa05000ce538c6af199dd8ef3ca7) (Recommended):
    ///   - X-BTC hot trustee address (deposit value)
    ///   - Null data transaction (opreturn data with valid account info)
    ///
    /// - 3 outputs (e.g. txid=003e7e005b172fe0046fd06a83679fbcdc5e3dd64c8ef9295662a463dea486aa) (Recommended):
    ///   - X-BTC hot trustee address (deposit value)
    ///   - Change address (don't care)
    ///   - Null data transaction (opreturn data with valid account info)
    ///
    /// - 3+ outputs (Not recommended):
    ///   - X-BTC hot trustee address (deposit value)
    ///   - Null data transaction (opreturn data with valid account info)
    ///   - Null data transaction (useless for us)
    ///   - Null data transaction (useless for us)
    ///   - ...
    ///   - Null data transaction (useless for us)
    ///
    /// # NOTE
    ///
    /// We only handle the first valid opreturn with valid account info, so ensure that there is
    /// **ONLY ONE** opreturn in the transaction outputs as much as possible when constructing
    /// X-BTC deposit transaction.
    pub fn detect_deposit_transaction_type<AccountId, Extractor>(
        &self,
        tx: &Transaction,
        input_addr: Option<Address>,
        extract_account: Extractor,
        current_trustee_pair: TrusteePair,
    ) -> BtcTxMetaType<AccountId>
    where
        AccountId: Debug,
        Extractor: Fn(&[u8]) -> Option<(OpReturnAccount<AccountId>, Option<ReferralId>)>,
    {
        let (op_return, deposit_value) =
            self.parse_deposit_transaction_outputs(tx, extract_account, current_trustee_pair);
        // check if deposit value is greater than minimum deposit value.
        if deposit_value >= self.min_deposit {
            // if opreturn.is_none() && input_addr.is_none()
            // we still think it's a deposit tx, but won't process it.
            BtcTxMetaType::Deposit(BtcDepositInfo {
                deposit_value,
                op_return,
                input_addr,
            })
        } else {
            warn!(
                "[detect_deposit_transaction_type] Receive a deposit tx ({:?}), but deposit value ({:}) is too low, drop it",
                hash_rev(tx.hash()), deposit_value,
            );
            BtcTxMetaType::Irrelevance
        }
    }

    /// Parse the outputs of X-BTC `Deposit` transaction.
    /// Return the account info that extracted from OP_RETURN data and the deposit value.
    pub fn parse_deposit_transaction_outputs<AccountId, Extractor>(
        &self,
        tx: &Transaction,
        extract_account: Extractor,
        current_trustee_pair: TrusteePair,
    ) -> (
        Option<(OpReturnAccount<AccountId>, Option<ReferralId>)>,
        u64,
    )
    where
        AccountId: Debug,
        Extractor: Fn(&[u8]) -> Option<(OpReturnAccount<AccountId>, Option<ReferralId>)>,
    {
        let mut account_info = None;
        // only handle first valid opreturn with account info, other opreturn would be dropped
        for op_return_script in tx
            .outputs
            .iter()
            .map(|output| Script::new(output.script_pubkey.clone()))
            .filter(|script| script.is_null_data_script())
        {
            debug!(
                "[parse_deposit_transaction_outputs] op_return_script:{:?}",
                op_return_script
            );
            if let Some(info) = extract_opreturn_data(&op_return_script)
                .and_then(|opreturn| extract_account(&opreturn))
            {
                account_info = Some(info);
                break;
            }
        }

        let mut deposit_value = 0;
        let (hot_addr, _) = current_trustee_pair;
        for output in &tx.outputs {
            // extract destination address from the script of output.
            if let Some(dest_addr) = extract_output_addr(output, self.network) {
                // check if the script address of the output is the hot trustee address
                if dest_addr.hash == hot_addr.hash && output.value > 0 {
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

#[cfg(test)]
mod tests {
    use sp_core::crypto::{set_default_ss58_version, Ss58AddressFormatRegistry};
    use sp_runtime::AccountId32;
    use xp_gateway_common::OpReturnAccount;

    use super::{Address, BtcTxTypeDetector, Network, Transaction};
    use crate::extractor::{AccountExtractor, OpReturnExtractor};

    fn account(addr: &str) -> AccountId32 {
        addr.parse::<AccountId32>().unwrap()
    }

    #[test]
    fn test_parse_deposit_transaction_outputs() {
        set_default_ss58_version(Ss58AddressFormatRegistry::ChainxAccount.into());

        // tx from MathWallet test
        let cases = vec![
            // txid: b368d3b822ec6656af441ccfa0ea2c846ec445286fd264e94a9a6edf0d7a1108
            // opreturn normal with addr (5Uj3ehamDZWPfgA8iAZenhcAmPDakjf4aMbkBB4dXVvjoW6x) (witness)
            // 3 outputs:
            // --> X-BTC hot trustee address (deposit value)
            // --> Change address (don't care)
            // --> Null data transaction (script_pubkey: 6a3035556a336568616d445a57506667413869415a656e6863416d5044616b6a6634614d626b424234645856766a6f573678)
            (
                "020000000001012f0f1be54334c36baf9edce4051acfcc4634e27504e39bc6466a1dadd36110e40100000017160014cd286c8c974540b1019e351c33551dc152e7447bffffffff03307500000000000017a914cb94110435d0635223eebe25ed2aaabc03781c4587672400000000000017a9149b995c9fddc8e5086626f7123631891a209d83a4870000000000000000326a3035556a336568616d445a57506667413869415a656e6863416d5044616b6a6634614d626b424234645856766a6f57367802483045022100f27347145406cc9706cd4d83018b07303c30b8d43f935019bf1d3accb38696f70220546db7a30dc8f0c4f02e17460573d009d26d85bd98a32642e88c6f74e76ac7140121037788522b753d5517cd9191c96f741a0d2b479369697d41567b4b418c7979d77300000000".parse::<Transaction>().unwrap(),
                (
                    Some((
                        OpReturnAccount::Wasm(account("5Uj3ehamDZWPfgA8iAZenhcAmPDakjf4aMbkBB4dXVvjoW6x")),
                        None
                    )),
                    30000
                )
            ),

            // txid: a7c91cb83ec0c0182704cafc447a2eb075c29d7d809b4898cd4aa37324f2b770
            // opreturn normal with address and referral (5QWKZY4QAt4NC8s5qcJVJnSbLSJ1W9iv5S4iJJPUr3Pdkdnj@Axonomy)
            // 3 outputs:
            // --> X-BTC hot trustee address (deposit value)
            // --> Change address (don't care)
            // --> Null data transaction (script_pubkey: 6a383551574b5a5934514174344e4338733571634a564a6e53624c534a3157396976355334694a4a5055723350646b646e6a4041786f6e6f6d79)
            (
                "020000000386389a63d8e858e06236d2b8de206763f2bd858adcbc8deb03bdb1f673b0d19c040000006b483045022100a4f40ddc02bb0326f476e664ac08015e4fd157c545dc2d03933e037b0b380f0e0220653f2fc0c229d3ce73f0829b53007700d6c517d27bcfdd1ad6ebdfce4fcbf1c20121024bfe28c0f47d7913d3fbd4555a63d448529924332d76c3b66251c9cd4ffa8340000000004e82355663aae88d258871ceff235a9c743291e3b1e1f4c2db6dd0774fe8ec8d010000006a473044022030013c331cbaa3a34a827d3c6a02e9dc93a88ef8ecb63a3d33b5c3087bcb8c7702205808f28435a7f22d30bb9540bafc58f2f0a4e2c3e0e5cc6ab59a2c7fbdfd9a610121024bfe28c0f47d7913d3fbd4555a63d448529924332d76c3b66251c9cd4ffa834000000000bd9bb637bc1e3bfa6209abeb59bdfd24aa1e80d911a00762a467a2488b4ba7fd000000006b483045022100bccff95c3298dd74027e5aa65da216384754136dee8b578cd6e70c7c3d19964d022078d71696e92a41d7d228b94020035b102cc3d4958dee2357c7aeeb509561678d0121024bfe28c0f47d7913d3fbd4555a63d448529924332d76c3b66251c9cd4ffa8340000000000380d99f380000000017a914cb94110435d0635223eebe25ed2aaabc03781c4587c0d40100000000001976a9146e9557e4fce7b1bb47056e357811c51b165ff8f488ac00000000000000003a6a383551574b5a5934514174344e4338733571634a564a6e53624c534a3157396976355334694a4a5055723350646b646e6a4041786f6e6f6d7900000000".parse::<Transaction>().unwrap(),
                (
                    Some((
                        OpReturnAccount::Wasm(account("5QWKZY4QAt4NC8s5qcJVJnSbLSJ1W9iv5S4iJJPUr3Pdkdnj")),
                        Some(b"Axonomy".to_vec())
                    )),
                    950000000
                )
            ),

            // txid: 41a5dedd90caa452fda70d50adfe9ce69c6ca75e05bfb8c5a4b426fda29436ad
            // opreturn normal with address and referral (5TtJf6MVyCcmS4SGh35SLzbhA76U5rNdURqZuVhjetsEKRND@MathWallet) (witness)
            // 3 outputs:
            // --> X-BTC hot trustee address (deposit value)
            // --> Change address (don't care)
            // --> Null data transaction (script_pubkey: 6a3b3554744a66364d567943636d53345347683335534c7a62684137365535724e645552715a7556686a657473454b524e44404d61746857616c6c6574)
            (
                "01000000000101b3dce032c6e5f6dd88f39f4197d76cf0b66b7592fdda7ba3e02bcebff9df7a7e010000001716001485863aa315bc11a844bc1eee01547be6a302a7caffffffff0300000000000000003d6a3b3554744a66364d567943636d53345347683335534c7a62684137365535724e645552715a7556686a657473454b524e44404d61746857616c6c6574f82a00000000000017a914cb94110435d0635223eebe25ed2aaabc03781c4587788f03000000000017a914d5ea60928669d832351b023bcfb3e85c530817d98702483045022100a16ac5ceb9ed9bb4aa8099fa5c8e8758e6ade55d2347c1d81c98550156900cb8022030e2b3c3e061ae353770b351c976ec9712a29608cf982d3a42daa2fa5329e6ea0121025eb9e1c63f28cccc67739ee940256fc26259e06167a0e9c411023bb1377ab1a000000000".parse::<Transaction>().unwrap(),
                (
                    Some((
                        OpReturnAccount::Wasm(account("5TtJf6MVyCcmS4SGh35SLzbhA76U5rNdURqZuVhjetsEKRND")),
                        Some(b"MathWallet".to_vec())
                    )),
                    11000
                )
            ),

            // txid: 9dee96445c3c7e9f2f215e009a3fada6118b5d8d0f5824431fd90bdde3ee72bb
            // opreturn normal with address and no referral (5QSHP7aZaW35N88qf7JHJAYZQBkxpMfRpeSBpaj3NT1HMDtn)
            // 3 outputs:
            // --> X-BTC hot trustee address (deposit value)
            // --> Change address (don't care)
            // --> Null data transaction (script_pubkey: 6a30355153485037615a615733354e38387166374a484a41595a51426b78704d66527065534270616a334e5431484d44746e)
            (
                "010000000199ada0c9b227557545aee0a5c948db96b8f009c8e57ba113af5d811fb51306fd000000006a473044022001eb5c5eb0852063e9cbea6d2d92b76b14998bef21af2231280b10a7df0abce80220497d3f8ba4e2c10b23dcff61b6d6c0e8179da0de9a675f81fc3685b5330ff158012103cf3e8985580fb495bddbb3baae07c35f2237da7e3d1a8e853cb2080ba6fa6ca4ffffffff03102700000000000017a914cb94110435d0635223eebe25ed2aaabc03781c4587710c0000000000001976a9140c456455ffdb307bd046ac4def9ee6522c54e24888ac0000000000000000326a30355153485037615a615733354e38387166374a484a41595a51426b78704d66527065534270616a334e5431484d44746e00000000".parse::<Transaction>().unwrap(),
                (
                    Some((
                        OpReturnAccount::Wasm(account("5QSHP7aZaW35N88qf7JHJAYZQBkxpMfRpeSBpaj3NT1HMDtn")),
                        None
                    )),
                    10000,
                )
            ),

            // opreturn data with 80 bytes, but invalid account info
            // 2 outputs:
            // --> X-BTC hot trustee address (deposit value)
            // --> Null data transaction (script_pubkey: 6a4c509999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999)
            (
                "0200000001776ae4d3fbebbd8568c610b265f54a1a8e1f03f2a16cac99ca9490e32583313b000000006a473044022074edd3b4f333ba3b0edb685922420bf904d417cd24584dbe76ad2e9b9c54e37602202a4027f77b7a4f6aaa7a8e7423e0b4740531e7a97527d51f341f75a950480b7f012102ebaf854b6220e3d44a32373aabbe1b6e4c3f824a7855aeac65b6854cd84d6f87ffffffff02a0bb0d00000000001976a9146ffd34b262b5099b80f8e84fe7e5dccaa79e2e7a88ac0000000000000000536a4c50999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999900000000".parse::<Transaction>().unwrap(),
                (None, 0)
            ),

            // txid: e41061d3ad1d6a46c69be30475e23446cccf1a05e4dc9eaf6bc33443e51b0f2f (witness)
            // 3 outputs:
            // --> X-BTC hot trustee address (deposit value)
            // --> Change address (don't care)
            // --> Null data transaction (script_pubkey: 6a)
            (
                "020000000001011529f2fbaca4cc374e12409cc3db0a8fe2509894f8b79f1f67d648f488d7a1f50100000017160014b1ef3d9fd4a68b53e75c56845076bfb4b4ae3974ffffffff03307500000000000017a914cb94110435d0635223eebe25ed2aaabc03781c4587bfe400000000000017a9141df425d522de50d46c32f979d73b823887446fd0870000000000000000016a02483045022100d591090fd8f0d62145d967fad754533fcdb5e7180c8644d16d071c3c5dfcb3a802200ee6cea9eb146d7e24b4142c36baa19e9c4c70095ef9b3ccc736247ecf0b8ed3012102632394028f212c1bc88f01dd14b4f8bc81c16ef464c830021030062a8f7788ae00000000".parse::<Transaction>().unwrap(),
                (None, 30000)
            ),

            // txid: f5a1d788f448d6671f9fb7f8949850e28f0adbc39c40124e37cca4acfbf22915 (witness)
            // 3 outputs:
            // --> X-BTC hot trustee address (deposit value)
            // --> Change address (don't care)
            // --> Null data transaction (script_pubkey: 6a)
            (
                "02000000000101681bd0b1158c7dc4ade8818c20820bedb906773a48c614e6ddc44cfd3c37408f010000001716001485863aa315bc11a844bc1eee01547be6a302a7caffffffff03204e00000000000017a914cb94110435d0635223eebe25ed2aaabc03781c458717a501000000000017a914d5ea60928669d832351b023bcfb3e85c530817d9870000000000000000016a02483045022100be53337e0c816e4f4d61b8b535431199105f04a1c043bd1d0f0362a525d7678502204ec154badbc84435d0c059b742dfddccca6338042fbf7e77bbfdbbfba183e1a10121025eb9e1c63f28cccc67739ee940256fc26259e06167a0e9c411023bb1377ab1a000000000".parse::<Transaction>().unwrap(),
                (None, 20000)
            ),

            // 3 outputs (Not recommended):
            // --> X-BTC hot trustee address (deposit value)
            // --> Null data transaction (script_pubkey: 6a4c509999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999)
            // --> Null data transaction (script_pubkey: 6a3b3554744a66364d567943636d53345347683335534c7a62684137365535724e645552715a7556686a657473454b524e44404d61746857616c6c6574)
            (
                "0200000001776ae4d3fbebbd8568c610b265f54a1a8e1f03f2a16cac99ca9490e32583313b000000006a47304402201871b85a7f608a24bcb95d3c8beeddef2d33377a6956d75d534faf3bca4d4fc102200ad4683ccad758f1f9de1e9d5a6af6d521010778bab4ded856eb4689355f670b012102ebaf854b6220e3d44a32373aabbe1b6e4c3f824a7855aeac65b6854cd84d6f87ffffffff030000000000000000536a4c509999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999a0bb0d000000000017a914cb94110435d0635223eebe25ed2aaabc03781c458700000000000000003d6a3b3554744a66364d567943636d53345347683335534c7a62684137365535724e645552715a7556686a657473454b524e44404d61746857616c6c657400000000".parse::<Transaction>().unwrap(),
                (
                    Some((
                        OpReturnAccount::Wasm(account("5TtJf6MVyCcmS4SGh35SLzbhA76U5rNdURqZuVhjetsEKRND")),
                        Some(b"MathWallet".to_vec())
                    )),
                    900000,
                )
            ),

            // 3 outputs (Not recommended):
            // --> X-BTC hot trustee address (deposit value)
            // --> Null data transaction (script_pubkey: 6a30355153485037615a615733354e38387166374a484a41595a51426b78704d66527065534270616a334e5431484d44746e)
            // --> Null data transaction (script_pubkey: 6a3b3554744a66364d567943636d53345347683335534c7a62684137365535724e645552715a7556686a657473454b524e44404d61746857616c6c6574)
            (
                "0200000001776ae4d3fbebbd8568c610b265f54a1a8e1f03f2a16cac99ca9490e32583313b000000006b483045022100e7526da20fda326cce8181516906fc287c49c6f420843f2ecdb0ee4d72e6f899022053259e1e4e6fea0be0277ec1f5c21822c678ac8999887369c4b05c0f897eae81012102ebaf854b6220e3d44a32373aabbe1b6e4c3f824a7855aeac65b6854cd84d6f87ffffffff03a0bb0d000000000017a914cb94110435d0635223eebe25ed2aaabc03781c45870000000000000000326a30355153485037615a615733354e38387166374a484a41595a51426b78704d66527065534270616a334e5431484d44746e00000000000000003d6a3b3554744a66364d567943636d53345347683335534c7a62684137365535724e645552715a7556686a657473454b524e44404d61746857616c6c657400000000".parse::<Transaction>().unwrap(),
                (
                    Some((
                        OpReturnAccount::Wasm(account("5QSHP7aZaW35N88qf7JHJAYZQBkxpMfRpeSBpaj3NT1HMDtn")),
                        None
                    )),
                    900000,
                )
            )
        ];

        const DEPOSIT_HOT_ADDR: &str = "3LFSUKkP26hun42J1Dy6RATsbgmBJb27NF";
        const DEPOSIT_COLD_ADDR: &str = "3FLBhPfEqmw4Wn5EQMeUzPLrQtJMprgwnw";
        let btc_tx_detector = BtcTxTypeDetector::new(Network::Mainnet, 0);

        let current_trustee_pair = (
            DEPOSIT_HOT_ADDR.parse::<Address>().unwrap(),
            DEPOSIT_COLD_ADDR.parse::<Address>().unwrap(),
        );
        for (tx, expect) in cases {
            let got = btc_tx_detector.parse_deposit_transaction_outputs(
                &tx,
                OpReturnExtractor::extract_account,
                current_trustee_pair,
            );
            assert_eq!(got, expect);
        }
    }
}
