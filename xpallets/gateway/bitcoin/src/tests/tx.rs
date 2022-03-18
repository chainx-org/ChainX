// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

#![allow(non_upper_case_globals)]

use frame_support::{assert_noop, assert_ok};
use sp_core::crypto::{set_default_ss58_version, Ss58AddressFormat};

use light_bitcoin::script::Script;
use light_bitcoin::{
    chain::Transaction,
    keys::{Address, Network},
    merkle::PartialMerkleTree,
    serialization::{self, Reader},
};

use xp_gateway_bitcoin::{AccountExtractor, BtcTxMetaType, BtcTxType, BtcTxTypeDetector};

use crate::mock::*;

use crate::tx::validator::parse_and_check_signed_tx_impl;
use crate::{
    tx::process_tx,
    types::{
        BtcDepositCache, BtcRelayedTxInfo, BtcTxResult, BtcTxState, BtcWithdrawalProposal,
        VoteResult,
    },
    Config, WithdrawalProposal,
};

// Tyoe is p2tr. Address farmat is Mainnet.:
const DEPOSIT_HOT_ADDR: &str = "bc1pn202yeugfa25nssxk2hv902kmxrnp7g9xt487u256n20jgahuwas6syxhp";
// Tyoe is p2sh. Address farmat is Mainnet.
const DEPOSIT_COLD_ADDR: &str = "3Ac85hjgeyNX96Q4BqUoAH5bh6gARxRDJm";

lazy_static::lazy_static! {
    // deposit without op return, output addr is DEPOSIT_HOT_ADDR. Withdraw is an example of spending from the script path.
    static ref deposit_taproot1_input_account: Vec<u8> = b"bc1pexff2s7l58sthpyfrtx500ax234stcnt0gz2lr4kwe0ue95a2e0s5wxhqg".to_vec();
    // https://signet.bitcoinexplorer.org/tx/b647a483444f60e547772fea7297bfceeb7bf9c3897e1b733c3c023b3140e64b#JSON
    static ref deposit_taproot1_prev: Transaction = "020000000001015dce8efe6cbd845587aa230a0b3667d4b52a45d3965d1607ab187de1f9d9d82b00000000000000000002a086010000000000225120dc82a9c33d787242d80fb4535bcc8d90bb13843fea52c9e78bb43c541dd607b900350c0000000000225120c9929543dfa1e0bb84891acd47bfa6546b05e26b7a04af8eb6765fcc969d565f0140708f206174a9e2963dd87d3afbb9f390fb320e2e9d4fdfc7b8bd7bc71a29c252026aa505ae71d4155ee3c13ce189ccba1fc0a26cfbcaa5f8b91bab377c2124eb00000000".parse().unwrap();
    // https://signet.bitcoinexplorer.org/tx/1f8e0f7dfa37b184244d022cdf2bc7b8e0bac8b52143ea786fa3f7bbe049eeae#JSON
    static ref deposit_taproot1: Transaction = "020000000001014be640313b023c3c731b7e89c3f97bebcebf9772ea2f7747e5604f4483a447b601000000000000000002a0860100000000002251209a9ea267884f5549c206b2aec2bd56d98730f90532ea7f7154d4d4f923b7e3bbc027090000000000225120c9929543dfa1e0bb84891acd47bfa6546b05e26b7a04af8eb6765fcc969d565f01404dc68b31efc1468f84db7e9716a84c19bbc53c2d252fd1d72fa6469e860a74486b0990332b69718dbcb5acad9d48634d23ee9c215ab15fb16f4732bed1770fdf00000000".parse().unwrap();
    static ref withdraw_taproot1_prev: Transaction = "020000000001014be640313b023c3c731b7e89c3f97bebcebf9772ea2f7747e5604f4483a447b601000000000000000002a0860100000000002251209a9ea267884f5549c206b2aec2bd56d98730f90532ea7f7154d4d4f923b7e3bbc027090000000000225120c9929543dfa1e0bb84891acd47bfa6546b05e26b7a04af8eb6765fcc969d565f01404dc68b31efc1468f84db7e9716a84c19bbc53c2d252fd1d72fa6469e860a74486b0990332b69718dbcb5acad9d48634d23ee9c215ab15fb16f4732bed1770fdf00000000".parse().unwrap();
    // https://signet.bitcoinexplorer.org/tx/1b342e9799748d4d5d415745350c38d9cb1e9f7fb078229a526ec47440e53ade#JSON
    static ref withdraw_taproot1: Transaction = "02000000000101aeee49e0bbf7a36f78ea4321b5c8bae0b8c72bdf2c024d2484b137fa7d0f8e1f0000000000000000000250c3000000000000225120c9929543dfa1e0bb84891acd47bfa6546b05e26b7a04af8eb6765fcc969d565f409c0000000000002251209a9ea267884f5549c206b2aec2bd56d98730f90532ea7f7154d4d4f923b7e3bb0340cfa8f924e169e72a6a098f8e72dcd03623f3836e5408a3682b77585e7716fd212ea9d842f8d775809e7fa10651fb0f0f709b176408edd58ea5b44b9b0d4dd29a222086a60c7d5dd3f4931cc8ad77a614402bdb591c042347c89281c48c7e9439be9dac61c0e56a1792f348690cdeebe60e3db6c4e94d94e742c619f7278e52f6cbadf5efe96a528ba3f61a5b0d4fbceea425a9028381458b32492bccc3f1faa473a649e23605554f5ea4b4044229173719228a35635eeffbd8a8fe526270b737ad523b99f600000000".parse().unwrap();

    // deposit with op return, output addr is DEPOSIT_HOT_ADDR. Withdraw is an example of spending from the script path.
    static ref op_account: AccountId = "5Qjpo7rQnwQetysagGzc4Rj7oswXSLmMqAuC2AbU6LFFFGj8".parse().unwrap();
    // https://signet.bitcoinexplorer.org/tx/1f8e0f7dfa37b184244d022cdf2bc7b8e0bac8b52143ea786fa3f7bbe049eeae#JSON
    static ref deposit_taproot2_prev: Transaction = "020000000001014be640313b023c3c731b7e89c3f97bebcebf9772ea2f7747e5604f4483a447b601000000000000000002a0860100000000002251209a9ea267884f5549c206b2aec2bd56d98730f90532ea7f7154d4d4f923b7e3bbc027090000000000225120c9929543dfa1e0bb84891acd47bfa6546b05e26b7a04af8eb6765fcc969d565f01404dc68b31efc1468f84db7e9716a84c19bbc53c2d252fd1d72fa6469e860a74486b0990332b69718dbcb5acad9d48634d23ee9c215ab15fb16f4732bed1770fdf00000000".parse().unwrap();
    // https://signet.bitcoinexplorer.org/tx/8e5d37c768acc4f3e794a10ad27bf0256237c80c22fa67117e3e3e1aec22ea5f#JSON
    static ref deposit_taproot2: Transaction = "02000000000101aeee49e0bbf7a36f78ea4321b5c8bae0b8c72bdf2c024d2484b137fa7d0f8e1f01000000000000000003a0860100000000002251209a9ea267884f5549c206b2aec2bd56d98730f90532ea7f7154d4d4f923b7e3bb0000000000000000326a3035516a706f3772516e7751657479736167477a6334526a376f737758534c6d4d7141754332416255364c464646476a38801a060000000000225120c9929543dfa1e0bb84891acd47bfa6546b05e26b7a04af8eb6765fcc969d565f01409e325889515ed47099fdd7098e6fafdc880b21456d3f368457de923f4229286e34cef68816348a0581ae5885ede248a35ac4b09da61a7b9b90f34c200872d2e300000000".parse().unwrap();
    static ref withdraw_taproot2_prev: Transaction = "02000000000101aeee49e0bbf7a36f78ea4321b5c8bae0b8c72bdf2c024d2484b137fa7d0f8e1f01000000000000000003a0860100000000002251209a9ea267884f5549c206b2aec2bd56d98730f90532ea7f7154d4d4f923b7e3bb0000000000000000326a3035516a706f3772516e7751657479736167477a6334526a376f737758534c6d4d7141754332416255364c464646476a38801a060000000000225120c9929543dfa1e0bb84891acd47bfa6546b05e26b7a04af8eb6765fcc969d565f01409e325889515ed47099fdd7098e6fafdc880b21456d3f368457de923f4229286e34cef68816348a0581ae5885ede248a35ac4b09da61a7b9b90f34c200872d2e300000000".parse().unwrap();
    // https://signet.bitcoinexplorer.org/tx/0f592933b493bedab209851cb2cf07871558ff57d86d645877b16651479b51a2#JSON
    static ref withdraw_taproot2: Transaction = "020000000001015fea22ec1a3e3e7e1167fa220cc8376225f07bd20aa194e7f3c4ac68c7375d8e0000000000000000000250c3000000000000225120c9929543dfa1e0bb84891acd47bfa6546b05e26b7a04af8eb6765fcc969d565f409c0000000000002251209a9ea267884f5549c206b2aec2bd56d98730f90532ea7f7154d4d4f923b7e3bb03402639d4d9882f6e7e42db38dbd2845c87b131737bf557643ef575c49f8fc6928869d9edf5fd61606fb07cced365fdc2c7b637e6ecc85b29906c16d314e7543e94222086a60c7d5dd3f4931cc8ad77a614402bdb591c042347c89281c48c7e9439be9dac61c0e56a1792f348690cdeebe60e3db6c4e94d94e742c619f7278e52f6cbadf5efe96a528ba3f61a5b0d4fbceea425a9028381458b32492bccc3f1faa473a649e23605554f5ea4b4044229173719228a35635eeffbd8a8fe526270b737ad523b99f600000000".parse().unwrap();

    // Convert between DEPOSIT_HOT_ADDR and DEPOSIT_COLD_ADDR
    // https://signet.bitcoinexplorer.org/tx/0f592933b493bedab209851cb2cf07871558ff57d86d645877b16651479b51a2#JSON
    static ref hot_to_cold_prev: Transaction = "020000000001015fea22ec1a3e3e7e1167fa220cc8376225f07bd20aa194e7f3c4ac68c7375d8e0000000000000000000250c3000000000000225120c9929543dfa1e0bb84891acd47bfa6546b05e26b7a04af8eb6765fcc969d565f409c0000000000002251209a9ea267884f5549c206b2aec2bd56d98730f90532ea7f7154d4d4f923b7e3bb03402639d4d9882f6e7e42db38dbd2845c87b131737bf557643ef575c49f8fc6928869d9edf5fd61606fb07cced365fdc2c7b637e6ecc85b29906c16d314e7543e94222086a60c7d5dd3f4931cc8ad77a614402bdb591c042347c89281c48c7e9439be9dac61c0e56a1792f348690cdeebe60e3db6c4e94d94e742c619f7278e52f6cbadf5efe96a528ba3f61a5b0d4fbceea425a9028381458b32492bccc3f1faa473a649e23605554f5ea4b4044229173719228a35635eeffbd8a8fe526270b737ad523b99f600000000".parse().unwrap();
    // https://signet.bitcoinexplorer.org/tx/917a751b9ccd91c7e184b028739a5520420df5cf04cd851a6ddf51f7bf33cf8a#JSON
    static ref hot_to_cold: Transaction = "02000000000101a2519b475166b17758646dd857ff58158707cfb21c8509b2dabe93b43329590f01000000000000000002204e00000000000017a91461cc314f71a88ebb492939784ca2663afaa8e88c8710270000000000002251209a9ea267884f5549c206b2aec2bd56d98730f90532ea7f7154d4d4f923b7e3bb0340aba2ce052b2fce8285ad550c4fd9182c8c8b4d2bcb91a4fd548d41c4a52f1137910ec79bf64ebc908db5c2713908e4cbb63d4e57dd723fdaf90f281b091d6f3e222086a60c7d5dd3f4931cc8ad77a614402bdb591c042347c89281c48c7e9439be9dac61c0e56a1792f348690cdeebe60e3db6c4e94d94e742c619f7278e52f6cbadf5efe96a528ba3f61a5b0d4fbceea425a9028381458b32492bccc3f1faa473a649e23605554f5ea4b4044229173719228a35635eeffbd8a8fe526270b737ad523b99f600000000".parse().unwrap();
    // Todo generate cold to hot
    // static ref cold_to_hot_prev: Transaction = "01000000015dfd7ae51ea70f3dfc9d4a49d57ae0d02660f089204fc8c4d086624d065f85620000000000000000000180010b270100000017a91495a12f1eba77d085711e9c837d04e4d8868a83438700000000".parse().unwrap();
    // static ref cold_to_hot: Transaction = "0100000001bc7be600cba239950fd664995bb9bc2cb88a29d95ddd49625644ef188c98012e0000000000000000000180010b270100000022512052898a03a9f04bb83f8a48fb953089de10e6ee70658b059551ebf7c008b05b7a00000000".parse().unwrap();
    static ref multisig_tx: Transaction = "010000000197a359e392f96c247eb3f4fafe81581541073f9cda5e7e8d32c47d5e4a84f4c100000000fd690100483045022100db5e82d95a0a3f02730e7216a9f6620a9803fcd236eda54feb0b90970c7ab762022018d127a637ca8efce400e313f0f3f28cfc0f347422ddf8bffed25158fa6fb9ca01483045022100886da91effdb5bd1e383014198401898c41617ebc8a1174481a70851fe271e6b022073c95c2aa5c16c9a793f81925146640c04fd2bc5dae1eec2a865b5d8e70d01ab01483045022100bbfbc3bc3e1d00ed0546a6876f2aca1759b45a1a30aa0d8073474cce67195ab3022019f38387b56c0f987d8bdadabea23876b8f6c1790178ca88247bad7345ebf1da014c8b53210376b9649206c74cc3dad6332c3a86d925a251bf9a55e6381f5d67b29a47559634210351eb6f687193cb79940541e60c62477aaa4a472d0b95e39f2f88aa61bb1020372103a4384a02d87e8552a2ee7fbaa2f336a7e08503a18c2ea51e85b6cbab4f14d176210285eed6fa121c3a82ba6d0c37fa37e72bb06740761bfe9f294d2fa95fe237d5ba54ae0000000002803801000000000017a914a4af2cb3387e5c34a4e7dbfdd5ddcf3e228748c987102700000000000022512093ef7ac6eb9e3a85629c4ed203e93cdf4ef50259a8f3d5f1b760086cd2d5823600000000".parse().unwrap();
}

fn mock_detect_transaction_type<T: Config>(
    tx: &Transaction,
    prev_tx: Option<&Transaction>,
) -> BtcTxMetaType<T::AccountId> {
    let btc_tx_detector = BtcTxTypeDetector::new(Network::Mainnet, 0);
    let current_trustee_pair = (
        DEPOSIT_HOT_ADDR.parse::<Address>().unwrap(),
        DEPOSIT_COLD_ADDR.parse::<Address>().unwrap(),
    );
    btc_tx_detector.detect_transaction_type::<T::AccountId, _>(
        tx,
        prev_tx,
        |script| T::AccountExtractor::extract_account(script),
        current_trustee_pair,
        None,
    )
}

#[test]
fn test_detect_tx_type() {
    set_default_ss58_version(Ss58AddressFormat::ChainXAccount);
    match mock_detect_transaction_type::<Test>(&deposit_taproot1, None) {
        BtcTxMetaType::Deposit(info) => {
            assert!(info.input_addr.is_none() && info.op_return.is_none())
        }
        _ => unreachable!("wrong type"),
    }
    match mock_detect_transaction_type::<Test>(&deposit_taproot2, None) {
        BtcTxMetaType::Deposit(info) => {
            assert!(info.input_addr.is_none() && info.op_return.is_some())
        }
        _ => unreachable!("wrong type"),
    }

    match mock_detect_transaction_type::<Test>(&deposit_taproot1, Some(&deposit_taproot1_prev)) {
        BtcTxMetaType::Deposit(info) => {
            assert!(info.input_addr.is_some() && info.op_return.is_none())
        }
        _ => unreachable!("wrong type"),
    }

    match mock_detect_transaction_type::<Test>(&deposit_taproot2, Some(&deposit_taproot2_prev)) {
        BtcTxMetaType::Deposit(info) => {
            assert!(info.input_addr.is_some() && info.op_return.is_some())
        }
        _ => unreachable!("wrong type"),
    }

    match mock_detect_transaction_type::<Test>(&withdraw_taproot1, Some(&withdraw_taproot1_prev)) {
        BtcTxMetaType::Withdrawal => {}
        _ => unreachable!("wrong type"),
    }

    match mock_detect_transaction_type::<Test>(&withdraw_taproot2, Some(&withdraw_taproot2_prev)) {
        BtcTxMetaType::Withdrawal => {}
        _ => unreachable!("wrong type"),
    }

    // hot_to_cold
    // if not pass a prev, would judge to a deposit, but this deposit could not be handled due to
    // opreturn and input_addr are all none, or if all send to cold, it would be Irrelevance
    match mock_detect_transaction_type::<Test>(&hot_to_cold, None) {
        BtcTxMetaType::Deposit(info) => {
            assert!(info.input_addr.is_none() && info.op_return.is_none())
        }
        _ => unreachable!("wrong type"),
    }
    // then if provide prev, it would be judge to a HotAndCold
    match mock_detect_transaction_type::<Test>(&hot_to_cold, Some(&hot_to_cold_prev)) {
        BtcTxMetaType::HotAndCold => {}
        _ => unreachable!("wrong type"),
    }

    // // cold_to_hot
    // // if not pass a prev, would judge to a deposit, but this deposit could not be handled due to
    // // opreturn and input_addr are all none
    // match mock_detect_transaction_type::<Test>(&cold_to_hot, None) {
    //     BtcTxMetaType::Deposit(info) => {
    //         assert!(info.input_addr.is_none() && info.op_return.is_none())
    //     }
    //     _ => unreachable!("wrong type"),
    // }
    // // then if provide prev, it would be judge to a HotAndCold
    // match mock_detect_transaction_type::<Test>(&cold_to_hot, Some(&cold_to_hot_prev)) {
    //     BtcTxMetaType::HotAndCold => {}
    //     _ => unreachable!("wrong type"),
    // }
}

fn mock_process_tx<T: Config>(tx: Transaction, prev_tx: Option<Transaction>) -> BtcTxState {
    let network = Network::Mainnet;
    let min_deposit = 0;
    let current_trustee_pair = (
        DEPOSIT_HOT_ADDR.parse::<Address>().unwrap(),
        DEPOSIT_COLD_ADDR.parse::<Address>().unwrap(),
    );
    let previous_trustee_pair = None;
    process_tx::<T>(
        tx,
        prev_tx,
        network,
        min_deposit,
        current_trustee_pair,
        previous_trustee_pair,
    )
}

#[test]
fn test_process_tx() {
    set_default_ss58_version(Ss58AddressFormat::ChainXAccount);
    ExtBuilder::default().build_and_execute(|| {
        // without op return and input address
        let r = mock_process_tx::<Test>(deposit_taproot1.clone(), None);
        assert_eq!(r.result, BtcTxResult::Failure);
        // without op return and with input address
        let r = mock_process_tx::<Test>(
            deposit_taproot1.clone(),
            Some(deposit_taproot1_prev.clone()),
        );
        assert_eq!(r.result, BtcTxResult::Success);
        assert_eq!(
            XGatewayBitcoin::pending_deposits(&deposit_taproot1_input_account.to_vec()),
            vec![BtcDepositCache {
                txid: deposit_taproot1.hash(),
                balance: 100000,
            }]
        );

        // withdraw
        WithdrawalProposal::<Test>::put(BtcWithdrawalProposal {
            sig_state: VoteResult::Unfinish,
            withdrawal_id_list: vec![],
            tx: withdraw_taproot1.clone(),
            trustee_list: vec![],
        });

        let r = mock_process_tx::<Test>(withdraw_taproot1.clone(), None);
        assert_eq!(r.result, BtcTxResult::Failure);
        let r = mock_process_tx::<Test>(
            withdraw_taproot1.clone(),
            Some(withdraw_taproot1_prev.clone()),
        );
        assert_eq!(r.result, BtcTxResult::Success);

        // with op return and without input address
        let r = mock_process_tx::<Test>(deposit_taproot2.clone(), None);
        assert_eq!(r.result, BtcTxResult::Success);
        assert_eq!(XAssets::usable_balance(&op_account, &X_BTC), 100000);
        assert_eq!(XGatewayCommon::bound_addrs(&op_account), Default::default());
        // with op return and input address
        let r = mock_process_tx::<Test>(
            deposit_taproot2.clone(),
            Some(deposit_taproot2_prev.clone()),
        );
        assert_eq!(r.result, BtcTxResult::Success);
        assert_eq!(XAssets::usable_balance(&op_account, &X_BTC), 300000);

        // withdraw
        WithdrawalProposal::<Test>::put(BtcWithdrawalProposal {
            sig_state: VoteResult::Unfinish,
            withdrawal_id_list: vec![],
            tx: withdraw_taproot2.clone(),
            trustee_list: vec![],
        });

        let r = mock_process_tx::<Test>(withdraw_taproot2.clone(), None);
        assert_eq!(r.result, BtcTxResult::Failure);
        let r = mock_process_tx::<Test>(
            withdraw_taproot2.clone(),
            Some(withdraw_taproot2_prev.clone()),
        );
        assert_eq!(r.result, BtcTxResult::Success);

        // hot and cold
        let r = mock_process_tx::<Test>(hot_to_cold.clone(), None);
        assert_eq!(r.result, BtcTxResult::Failure);
        let r = mock_process_tx::<Test>(hot_to_cold.clone(), Some(hot_to_cold_prev.clone()));
        assert_eq!(r.tx_type, BtcTxType::HotAndCold);
        assert_eq!(r.result, BtcTxResult::Success);
    })
}

#[test]
fn test_push_tx_call() {
    set_default_ss58_version(Ss58AddressFormat::ChainXAccount);
    // https://blockchain.info/rawtx/f1a9161a045a01db7ae02b8c0531e2fe2e9740efe30afe6d84a12e3cac251344?format=hex
    let normal_deposit: Transaction = "02000000000101aeee49e0bbf7a36f78ea4321b5c8bae0b8c72bdf2c024d2484b137fa7d0f8e1f01000000000000000003a0860100000000002251209a9ea267884f5549c206b2aec2bd56d98730f90532ea7f7154d4d4f923b7e3bb0000000000000000326a3035516a706f3772516e7751657479736167477a6334526a376f737758534c6d4d7141754332416255364c464646476a38801a060000000000225120c9929543dfa1e0bb84891acd47bfa6546b05e26b7a04af8eb6765fcc969d565f01409e325889515ed47099fdd7098e6fafdc880b21456d3f368457de923f4229286e34cef68816348a0581ae5885ede248a35ac4b09da61a7b9b90f34c200872d2e300000000".parse().unwrap();
    let tx = serialization::serialize(&normal_deposit);
    let headers = generate_blocks_63290_63310();
    let block_hash = headers[&63299].hash();

    let raw_proof = hex::decode("0a000000050a59b195a68a29037580798ca0414941eb46eaf7607db2d0da1ff89e9570ce455fea22ec1a3e3e7e1167fa220cc8376225f07bd20aa194e7f3c4ac68c7375d8e0a35e47541de7d0aa7312dabcf3bc9f06603e832427b8e4fe9a97a309f8cd7141687d11a3fd8f21e2105a52a3c36a17ea870e326ecddb23221d4cc0398b6c44bdcce3f191919a31f4cfaca5a786cc8315db76683ad6b8008f2ed9b348df76a0d022f00").unwrap();
    let proof: PartialMerkleTree = serialization::deserialize(Reader::new(&raw_proof)).unwrap();

    ExtBuilder::default().build_and_execute(|| {
        let confirmed = XGatewayBitcoin::confirmation_number();
        // insert headers
        for i in 63291..=63299 + confirmed {
            assert_ok!(XGatewayBitcoin::apply_push_header(headers[&i]));
        }
        let info = BtcRelayedTxInfo {
            block_hash,
            merkle_proof: proof,
        };

        assert_ok!(XGatewayBitcoin::push_transaction(
            frame_system::RawOrigin::Signed(Default::default()).into(),
            tx.clone().into(),
            info.clone(),
            None,
        ));

        // reject replay
        assert_noop!(
            XGatewayBitcoin::push_transaction(
                frame_system::RawOrigin::Signed(Default::default()).into(),
                tx.clone().into(),
                info,
                None,
            ),
            XGatewayBitcoinErr::ReplayedTx,
        );
    });
}

#[test]
fn test_parse_and_check_signed_tx_impl() {
    ExtBuilder::default().build_and_execute(|| {
    let redeem_script: Script = hex::decode("53210376b9649206c74cc3dad6332c3a86d925a251bf9a55e6381f5d67b29a47559634210351eb6f687193cb79940541e60c62477aaa4a472d0b95e39f2f88aa61bb1020372103a4384a02d87e8552a2ee7fbaa2f336a7e08503a18c2ea51e85b6cbab4f14d176210285eed6fa121c3a82ba6d0c37fa37e72bb06740761bfe9f294d2fa95fe237d5ba54ae").unwrap().into();
    assert_eq!(
        parse_and_check_signed_tx_impl::<Test>(&multisig_tx, redeem_script),
        Ok(3)
    );
    });
}
