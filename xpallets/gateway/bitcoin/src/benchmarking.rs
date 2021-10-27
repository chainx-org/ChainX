// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use codec::{Decode, Encode};
use frame_benchmarking::{benchmarks, whitelisted_caller};
use frame_support::storage::{StorageMap, StorageValue};
use frame_system::RawOrigin;
use sp_runtime::{AccountId32, SaturatedConversion};
use sp_std::{collections::btree_map::BTreeMap, prelude::*};

use chainx_primitives::AssetId;
use xp_gateway_bitcoin::BtcTxType;
use xpallet_assets::{BalanceOf, Module as XAssets};
use xpallet_gateway_records::{Module as XGatewayRecords, WithdrawalState};

use light_bitcoin::{
    chain::{BlockHeader, Transaction},
    merkle::PartialMerkleTree,
    primitives::H256,
    serialization::{self, Reader},
};

use crate::{
    types::*, Call, Module, PendingDeposits, Trait, TxState, Verifier, WithdrawalProposal,
};

const ASSET_ID: AssetId = xp_protocol::X_BTC;

fn generate_blocks_576576_578692() -> BTreeMap<u32, BlockHeader> {
    let bytes = include_bytes!("./res/headers-576576-578692.raw");
    Decode::decode(&mut &bytes[..]).unwrap()
}

fn account<T: Trait>(pubkey: &str) -> T::AccountId {
    let pubkey = hex::decode(pubkey).unwrap();
    let mut public = [0u8; 32];
    public.copy_from_slice(pubkey.as_slice());
    let account = AccountId32::from(public).encode();
    Decode::decode(&mut account.as_slice()).unwrap()
}

fn alice<T: Trait>() -> T::AccountId {
    // sr25519 Alice
    account::<T>("d43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d")
}

fn bob<T: Trait>() -> T::AccountId {
    // sr25519 Bob
    account::<T>("8eaf04151687736326c9fea17e25fc5287613693c912909cb226aa4794f26a48")
}

fn withdraw_tx() -> (Transaction, BtcRelayedTxInfo, Transaction) {
    // block height: 577696
    // https://blockchain.info/rawtx/62c389f1974b8a44737d76f92da0f5cd7f6f48d065e7af6ba368298361141270?format=hex
    const RAW_TX: &str = "0100000001052ceda6cf9c93012a994f4ffa2a29c9e31ecf96f472b175eb8e602bfa2b2c5100000000fdfd000047304402200e4d732c456f4722d376252be16554edb27fc93c55db97859e16682bc62b014502202b9c4b01ad55daa1f76e6a564b7762cd0a81240c947806ab3f3b056f2e77c1da01483045022100c7cd680992de60da8c33fc3ef7f5ead85b204660822d9fbda2d85f9fadba732a022021fdc49b20a6007ea971a385732a4065d1d7c792ac9dc391034fb78aa9f5034b014c69522102df92e88c4380778c9c48268460a124a8f4e7da883f80477deaa644ced486efc6210244d81efeb4171b1a8a433b87dd202117f94e44c909c49e42e77b69b5a6ce7d0d2103a36339f413da869df12b1ab0def91749413a0dee87f0bfa85ba7196e6cdad10253aeffffffff03e0349500000000001976a91413256ff2dee6e80c275ddb877abc1ffe453a731488ace00f9700000000001976a914ea6e8dd56703ace584eb9dff0224629f8486672988acc88a02000000000017a914cb94110435d0635223eebe25ed2aaabc03781c458700000000";
    let tx = RAW_TX.parse::<Transaction>().unwrap();

    // https://blockchain.info/rawtx/512c2bfa2b608eeb75b172f496cf1ee3c9292afa4f4f992a01939ccfa6ed2c05?format=hex
    const RAW_TX_PREV: &str = "02000000018554af3a19f2475bb293e81fe123b588a50d7c86ce97ed4f015853b427e45f12040000006a473044022037957f493964792e6bedd37aa5193892bd9fdb5d974d87f5334f36b0d544c7f202203d7bb2ac644204437b77e9c34ea5bf875da41d728ef7352c9d74ff507da64502012102bd47917d4cf403ca8e9cb71c84a127e0451686877fe186614385025ccd1ed9cc000000000260a62f010000000017a914cb94110435d0635223eebe25ed2aaabc03781c45870000000000000000366a343552547a425a4d3274346537414d547442534e3853424c3878316b716e39713769355a75566e3569537876526341326b40484c5400000000";
    let prev_tx = RAW_TX_PREV.parse::<Transaction>().unwrap();

    const RAW_PROOF: &str = "550b00000de5fe93267100092415bc4203eac82319e7560f1c8d1f13ff127f6bdfeda4c0d55dd9a20c18bf308fb09ee5ca03d6e9870ede459785a157c9f06ad17544b38fbd507ecd6c351d88035bf29e2f5758449b2ba582120a76306113a996e67e1b5ddb00ddb99fd307521d3e2b63d145bf9e26da9e944e4e221880c22a08fe6d45a9c970121461832968a36bafe765d0486f7fcdf5a02df9767d73448a4b97f189c362459db0ca16f5f6a6b369ab12e77a7da93eafdda7c6ea816c65270c8b698cf0ebe34519452b2871b119eac61e55d535466c7d248e7e3d49518aa2a616ac6abf371e5b163838b18c246a90c92f03771252991ff13aef709bcdfcb02f440ab07a6ef04f8f3b4644b47a2687a17d4af1e2901923fd611e98472cd983ea1f3510355143a918715513706610b538293c74dd73a2aef8aed25754225310403b32d9e603c47c6f65f2eb6694b347b3071c99a75cf24b4d1dfcaca307f66706e6bac9b9218e00f786051630f1b483575b1d11e81164b4d989b53515f7f3eb86ec0c0d293895bdfbd13cedeb5c5ba391a66ef9c9e53ac47e997ca122cc8bfad8ea653fb95404d7af0100";
    let proof = hex::decode(RAW_PROOF).unwrap();
    let merkle_proof: PartialMerkleTree = serialization::deserialize(Reader::new(&proof)).unwrap();

    let header = generate_blocks_576576_578692()[&577696];
    let info = BtcRelayedTxInfo {
        block_hash: header.hash(),
        merkle_proof,
    };
    (tx, info, prev_tx)
}

fn prepare_withdrawal<T: Trait>() -> Transaction {
    // https://blockchain.info/rawtx/62c389f1974b8a44737d76f92da0f5cd7f6f48d065e7af6ba368298361141270?format=hex
    const RAW_TX: &str = "0100000001052ceda6cf9c93012a994f4ffa2a29c9e31ecf96f472b175eb8e602bfa2b2c5100000000fdfd000047304402200e4d732c456f4722d376252be16554edb27fc93c55db97859e16682bc62b014502202b9c4b01ad55daa1f76e6a564b7762cd0a81240c947806ab3f3b056f2e77c1da01483045022100c7cd680992de60da8c33fc3ef7f5ead85b204660822d9fbda2d85f9fadba732a022021fdc49b20a6007ea971a385732a4065d1d7c792ac9dc391034fb78aa9f5034b014c69522102df92e88c4380778c9c48268460a124a8f4e7da883f80477deaa644ced486efc6210244d81efeb4171b1a8a433b87dd202117f94e44c909c49e42e77b69b5a6ce7d0d2103a36339f413da869df12b1ab0def91749413a0dee87f0bfa85ba7196e6cdad10253aeffffffff03e0349500000000001976a91413256ff2dee6e80c275ddb877abc1ffe453a731488ace00f9700000000001976a914ea6e8dd56703ace584eb9dff0224629f8486672988acc88a02000000000017a914cb94110435d0635223eebe25ed2aaabc03781c458700000000";
    let old_withdraw = RAW_TX.parse::<Transaction>().unwrap();

    // https://blockchain.info/rawtx/092684402f9b21abdb1d2d76511d5983bd1250d173ced171a3f76d03fcc43e97?format=hex
    const ANOTHER_TX: &str = "0100000001059ec66e2a2123364a56bd48f10f57d8a41ecf4082669e6fc85485637043879100000000fdfd00004830450221009fbe7b8f2f4ae771e8773cb5206b9f20286676e2c7cfa98a8e95368acfc3cb3c02203969727a276d7333d5f8815fa364307b8015783cfefbd53def28befdb81855fc0147304402205e5bbe039457d7657bb90dbe63ac30b9547242b44cc03e1f7a690005758e34aa02207208ed76a269d193f1e10583bd902561dbd02826d0486c33a4b1b1839a3d226f014c69522102df92e88c4380778c9c48268460a124a8f4e7da883f80477deaa644ced486efc6210244d81efeb4171b1a8a433b87dd202117f94e44c909c49e42e77b69b5a6ce7d0d2103a36339f413da869df12b1ab0def91749413a0dee87f0bfa85ba7196e6cdad10253aeffffffff04288e0300000000001976a914eb016d7998c88a79a50a0408dd7d5839b1ce1a6888aca0bb0d00000000001976a914646fe05e35369248c3f8deea436dc2b92c7dc86888ac50c30000000000001976a914d1a68d6e891a88d53d9bc3b88d172a3ff6b238c388ac20ee03020000000017a914cb94110435d0635223eebe25ed2aaabc03781c458700000000";
    let tmp = ANOTHER_TX.parse::<Transaction>().unwrap();

    let alice = alice::<T>();
    let bob = bob::<T>();
    let withdrawal_fee = Module::<T>::btc_withdrawal_fee();

    let balance1 = (9778400 + withdrawal_fee).saturated_into();
    let balance2 = (9900000 + withdrawal_fee).saturated_into();
    XGatewayRecords::<T>::deposit(&alice, ASSET_ID, balance1).unwrap();
    XGatewayRecords::<T>::deposit(&bob, ASSET_ID, balance2).unwrap();
    // prepare withdraw info
    XGatewayRecords::<T>::withdraw(
        &alice,
        ASSET_ID,
        balance1,
        b"12kEgqNShFw7BN27QCMQZCynQpSuV4x1Ax".to_vec(),
        b"memo".to_vec().into(),
    )
    .unwrap();
    XGatewayRecords::<T>::withdraw(
        &bob,
        ASSET_ID,
        balance2,
        b"1NNZZKR6pos2M4yiJhS76NjcRHxoJUATy4".to_vec(),
        b"memo".to_vec().into(),
    )
    .unwrap();

    let proposal = BtcWithdrawalProposal::<T::AccountId> {
        sig_state: VoteResult::Finish,
        withdrawal_id_list: vec![0, 1],
        tx: old_withdraw.clone(),
        trustee_list: vec![(alice, true), (bob, true)],
    };
    WithdrawalProposal::<T>::put(proposal);

    // replace tx
    let mut new_withdraw = old_withdraw;
    new_withdraw.inputs = tmp.inputs; // replace inputs
    new_withdraw
}

// block height: 577696
// https://blockchain.info/rawtx/62c389f1974b8a44737d76f92da0f5cd7f6f48d065e7af6ba368298361141270?format=hex
fn create_tx() -> Transaction {
    "0100000001052ceda6cf9c93012a994f4ffa2a29c9e31ecf96f472b175eb8e602bfa2b2c5100000000b40047304402200e4d732c456f4722d376252be16554edb27fc93c55db97859e16682bc62b014502202b9c4b01ad55daa1f76e6a564b7762cd0a81240c947806ab3f3b056f2e77c1da014c69522102df92e88c4380778c9c48268460a124a8f4e7da883f80477deaa644ced486efc6210244d81efeb4171b1a8a433b87dd202117f94e44c909c49e42e77b69b5a6ce7d0d2103a36339f413da869df12b1ab0def91749413a0dee87f0bfa85ba7196e6cdad10253aeffffffff03e0349500000000001976a91413256ff2dee6e80c275ddb877abc1ffe453a731488ace00f9700000000001976a914ea6e8dd56703ace584eb9dff0224629f8486672988acc88a02000000000017a914cb94110435d0635223eebe25ed2aaabc03781c458700000000".parse::<Transaction>().unwrap()
}

// push header 576577 - 577702 (current confirm height is 577696)
fn prepare_headers<T: Trait>(caller: &T::AccountId) {
    for (height, header) in generate_blocks_576576_578692() {
        if height == 576576 {
            continue;
        }
        if height == 577702 {
            // confirm for 577696, confirmation number is 6
            break;
        }
        let header = serialization::serialize(&header).into();
        Module::<T>::push_header(RawOrigin::Signed(caller.clone()).into(), header).unwrap();
    }
}

// benchmarks! {
//     _{ }
//
//     push_header {
//         let receiver: T::AccountId = whitelisted_caller();
//         let insert_height = 576576 + 1;
//         let header = generate_blocks_576576_578692()[&insert_height];
//         let hash = header.hash();
//         let header_raw = serialization::serialize(&header).into();
//         let amount: BalanceOf<T> = 1000.into();
//     }: _(RawOrigin::Signed(receiver), header_raw)
//     verify {
//         assert!(Module::<T>::headers(&hash).is_some());
//     }
//
//     push_transaction {
//         let n = 1024 * 1024 * 500; // 500KB length
//         let l = 1024 * 1024 * 500; // 500KB length
//
//         let caller: T::AccountId = whitelisted_caller();
//
//         prepare_headers::<T>(&caller);
//         let (tx, info, prev_tx) = withdraw_tx();
//         let tx_hash = tx.hash();
//         let tx_raw = serialization::serialize(&tx).into();
//         let prev_tx_raw = serialization::serialize(&prev_tx).into();
//
//         XGatewayRecords::<T>::deposit(&caller, ASSET_ID, 9778400.into()).unwrap();
//         XGatewayRecords::<T>::deposit(&caller, ASSET_ID, 9900000.into()).unwrap();
//         XGatewayRecords::<T>::withdraw(&caller, ASSET_ID, 9778400.into(), b"".to_vec(), b"".to_vec().into()).unwrap();
//         XGatewayRecords::<T>::withdraw(&caller, ASSET_ID, 9900000.into(), b"".to_vec(), b"".to_vec().into()).unwrap();
//         xpallet_gateway_records::WithdrawalStateOf::insert(0, WithdrawalState::Processing);
//         xpallet_gateway_records::WithdrawalStateOf::insert(1, WithdrawalState::Processing);
//
//         let proposal = BtcWithdrawalProposal::<T::AccountId> {
//             sig_state: VoteResult::Finish,
//             withdrawal_id_list: vec![0, 1],
//             tx: tx.clone(),
//             trustee_list: vec![],
//         };
//         WithdrawalProposal::<T>::put(proposal);
//
//     }: _(RawOrigin::Signed(caller), tx_raw, info, Some(prev_tx_raw))
//     verify {
//         assert!(WithdrawalProposal::<T>::get().is_none());
//         assert_eq!(
//             TxState::get(tx_hash),
//             Some(BtcTxState {
//                 tx_type: BtcTxType::Withdrawal,
//                 result: BtcTxResult::Success,
//             })
//         );
//     }
//
//     create_withdraw_tx {
//         let n = 100;                // 100 withdrawal count
//         let l = 1024 * 1024 * 500;  // 500KB length
//
//         let caller = alice::<T>();
//
//         let (tx, info, prev_tx) = withdraw_tx();
//         let tx_hash = tx.hash();
//         let tx_raw: Vec<u8> = serialization::serialize(&tx).into();
//         let prev_tx_raw: Vec<u8> = serialization::serialize(&prev_tx).into();
//
//         let btc_withdrawal_fee = Module::<T>::btc_withdrawal_fee();
//         let first_withdraw = (9778400 + btc_withdrawal_fee).saturated_into();
//         let second_withdraw = (9900000 + btc_withdrawal_fee).saturated_into();
//         XGatewayRecords::<T>::deposit(&caller, ASSET_ID, first_withdraw).unwrap();
//         XGatewayRecords::<T>::deposit(&caller, ASSET_ID, second_withdraw).unwrap();
//         XGatewayRecords::<T>::withdraw(&caller, ASSET_ID, first_withdraw, b"12kEgqNShFw7BN27QCMQZCynQpSuV4x1Ax".to_vec(), b"".to_vec().into()).unwrap();
//         XGatewayRecords::<T>::withdraw(&caller, ASSET_ID, second_withdraw, b"1NNZZKR6pos2M4yiJhS76NjcRHxoJUATy4".to_vec(), b"".to_vec().into()).unwrap();
//
//         let tx = create_tx();
//         let tx_raw: Vec<u8> = serialization::serialize(&tx).into();
//     }: _(RawOrigin::Signed(caller), vec![0, 1], tx_raw)
//     verify {
//         assert!(WithdrawalProposal::<T>::get().is_some());
//     }
//
//     sign_withdraw_tx {
//         let l = 1024 * 1024 * 500; // 500KB length
//         let tx = create_tx();
//         let alice = alice::<T>();
//         let bob = bob::<T>();
//
//         let proposal = BtcWithdrawalProposal::<T::AccountId> {
//             sig_state: VoteResult::Unfinish,
//             withdrawal_id_list: vec![0, 1],
//             tx: tx,
//             trustee_list: vec![ (alice, true) ],
//         };
//         WithdrawalProposal::<T>::put(proposal);
//
//         let (signed_tx, _, _) = withdraw_tx();
//         let tx_raw: Vec<u8> = serialization::serialize(&signed_tx).into();
//     }: _(RawOrigin::Signed(bob), Some(tx_raw))
//     verify {
//         assert_eq!(WithdrawalProposal::<T>::get().unwrap().sig_state, VoteResult::Finish);
//     }
//
//     set_best_index {
//         let best = BtcHeaderIndex {
//             hash: H256::repeat_byte(1),
//             height: 100,
//         };
//     }: _(RawOrigin::Root, best)
//     verify {
//         assert_eq!(Module::<T>::best_index(), best);
//     }
//
//     set_confirmed_index {
//         let confirmed = BtcHeaderIndex {
//             hash: H256::repeat_byte(1),
//             height: 100,
//         };
//     }: _(RawOrigin::Root, confirmed)
//     verify {
//         assert_eq!(Module::<T>::confirmed_index(), Some(confirmed));
//     }
//
//     remove_pending {
//         let addr = b"3AWmpzJ1kSF1cktFTDEb3qmLcdN8YydxA7".to_vec();
//         let v = vec![
//             BtcDepositCache {
//                 txid: H256::repeat_byte(1),
//                 balance: 100000000,
//             },
//             BtcDepositCache {
//                 txid: H256::repeat_byte(2),
//                 balance: 200000000,
//             },
//             BtcDepositCache {
//                 txid: H256::repeat_byte(3),
//                 balance: 300000000,
//             },
//         ];
//         PendingDeposits::insert(&addr, v);
//         let receiver: T::AccountId = whitelisted_caller();
//     }: _(RawOrigin::Root, addr.clone(), Some(receiver.clone()))
//     verify {
//         assert!(Module::<T>::pending_deposits(&addr).is_empty());
//         assert_eq!(XAssets::<T>::usable_balance(&receiver, &ASSET_ID), (100000000 + 200000000 + 300000000).into());
//     }
//
//     remove_proposal {
//         let (tx, _, _) = withdraw_tx();
//         let proposal = BtcWithdrawalProposal::<T::AccountId> {
//             sig_state: VoteResult::Unfinish,
//             withdrawal_id_list: vec![0, 1],
//             tx: tx,
//             trustee_list: vec![],
//         };
//         WithdrawalProposal::<T>::put(proposal);
//     }: _(RawOrigin::Root)
//     verify {
//         assert!(WithdrawalProposal::<T>::get().is_none());
//     }
//
//     force_replace_proposal_tx {
//         let l = 1024 * 1024 * 500; // 500KB length
//
//         Verifier::put(BtcTxVerifier::Test);
//         let tx = prepare_withdrawal::<T>();
//         let raw = serialization::serialize(&tx);
//     }: _(RawOrigin::Root, raw.into())
//     verify {
//         assert_eq!(WithdrawalProposal::<T>::get().unwrap().tx, tx);
//     }
//
//     set_btc_withdrawal_fee {
//         let caller = alice::<T>();
//     }: _(RawOrigin::Root,  2000000)
//     verify {
//     }
//
//     set_btc_deposit_limit {
//         let caller = alice::<T>();
//     }: _(RawOrigin::Root,  2000000)
//     verify {
//     }
// }
//
// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::mock::{ExtBuilder, Test};
//     use frame_support::assert_ok;
//
//     #[test]
//     fn test_benchmarks() {
//         ExtBuilder::default().build().execute_with(|| {
//             assert_ok!(test_benchmark_push_header::<Test>());
//             assert_ok!(test_benchmark_push_transaction::<Test>());
//             assert_ok!(test_benchmark_create_withdraw_tx::<Test>());
//             assert_ok!(test_benchmark_sign_withdraw_tx::<Test>());
//             assert_ok!(test_benchmark_set_best_index::<Test>());
//             assert_ok!(test_benchmark_set_confirmed_index::<Test>());
//             assert_ok!(test_benchmark_remove_pending::<Test>());
//             assert_ok!(test_benchmark_force_replace_proposal_tx::<Test>());
//             assert_ok!(test_benchmark_set_btc_withdrawal_fee::<Test>());
//             assert_ok!(test_benchmark_set_btc_deposit_limit::<Test>());
//         });
//     }
// }
