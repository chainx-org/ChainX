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

use crate::Module as XGatewayBitcoin;
use crate::{
    types::{
        BtcDepositCache, BtcHeaderIndex, BtcRelayedTxInfo, BtcTxResult, BtcTxState, BtcTxVerifier,
        BtcWithdrawalProposal, VoteResult,
    },
    Call, Module, PendingDeposits, Trait, TxState, Verifier, WithdrawalProposal,
};

const ASSET_ID: AssetId = xp_protocol::X_BTC;

pub fn generate_blocks_from_raw() -> BTreeMap<u32, BlockHeader> {
    let bytes = include_bytes!("./tests/res/headers-576576-578692.raw");
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
    // https://btc.com/62c389f1974b8a44737d76f92da0f5cd7f6f48d065e7af6ba368298361141270.rawhex
    const RAW_TX: &'static str = "0100000001052ceda6cf9c93012a994f4ffa2a29c9e31ecf96f472b175eb8e602bfa2b2c5100000000fdfd000047304402200e4d732c456f4722d376252be16554edb27fc93c55db97859e16682bc62b014502202b9c4b01ad55daa1f76e6a564b7762cd0a81240c947806ab3f3b056f2e77c1da01483045022100c7cd680992de60da8c33fc3ef7f5ead85b204660822d9fbda2d85f9fadba732a022021fdc49b20a6007ea971a385732a4065d1d7c792ac9dc391034fb78aa9f5034b014c69522102df92e88c4380778c9c48268460a124a8f4e7da883f80477deaa644ced486efc6210244d81efeb4171b1a8a433b87dd202117f94e44c909c49e42e77b69b5a6ce7d0d2103a36339f413da869df12b1ab0def91749413a0dee87f0bfa85ba7196e6cdad10253aeffffffff03e0349500000000001976a91413256ff2dee6e80c275ddb877abc1ffe453a731488ace00f9700000000001976a914ea6e8dd56703ace584eb9dff0224629f8486672988acc88a02000000000017a914cb94110435d0635223eebe25ed2aaabc03781c458700000000";

    let withdraw = RAW_TX.parse::<Transaction>().unwrap();
    // https://btc.com/512c2bfa2b608eeb75b172f496cf1ee3c9292afa4f4f992a01939ccfa6ed2c05.rawhex
    const RAW_TX_PREV: &'static str = "02000000018554af3a19f2475bb293e81fe123b588a50d7c86ce97ed4f015853b427e45f12040000006a473044022037957f493964792e6bedd37aa5193892bd9fdb5d974d87f5334f36b0d544c7f202203d7bb2ac644204437b77e9c34ea5bf875da41d728ef7352c9d74ff507da64502012102bd47917d4cf403ca8e9cb71c84a127e0451686877fe186614385025ccd1ed9cc000000000260a62f010000000017a914cb94110435d0635223eebe25ed2aaabc03781c45870000000000000000366a343552547a425a4d3274346537414d547442534e3853424c3878316b716e39713769355a75566e3569537876526341326b40484c5400000000";
    let prev = RAW_TX_PREV.parse::<Transaction>().unwrap();

    const RAW_PROOF: &[u8] = &[
        85, 11, 0, 0, 13, 229, 254, 147, 38, 113, 0, 9, 36, 21, 188, 66, 3, 234, 200, 35, 25, 231,
        86, 15, 28, 141, 31, 19, 255, 18, 127, 107, 223, 237, 164, 192, 213, 93, 217, 162, 12, 24,
        191, 48, 143, 176, 158, 229, 202, 3, 214, 233, 135, 14, 222, 69, 151, 133, 161, 87, 201,
        240, 106, 209, 117, 68, 179, 143, 189, 80, 126, 205, 108, 53, 29, 136, 3, 91, 242, 158, 47,
        87, 88, 68, 155, 43, 165, 130, 18, 10, 118, 48, 97, 19, 169, 150, 230, 126, 27, 93, 219, 0,
        221, 185, 159, 211, 7, 82, 29, 62, 43, 99, 209, 69, 191, 158, 38, 218, 158, 148, 78, 78,
        34, 24, 128, 194, 42, 8, 254, 109, 69, 169, 201, 112, 18, 20, 97, 131, 41, 104, 163, 107,
        175, 231, 101, 208, 72, 111, 127, 205, 245, 160, 45, 249, 118, 125, 115, 68, 138, 75, 151,
        241, 137, 195, 98, 69, 157, 176, 202, 22, 245, 246, 166, 179, 105, 171, 18, 231, 122, 125,
        169, 62, 175, 221, 167, 198, 234, 129, 108, 101, 39, 12, 139, 105, 140, 240, 235, 227, 69,
        25, 69, 43, 40, 113, 177, 25, 234, 198, 30, 85, 213, 53, 70, 108, 125, 36, 142, 126, 61,
        73, 81, 138, 162, 166, 22, 172, 106, 191, 55, 30, 91, 22, 56, 56, 177, 140, 36, 106, 144,
        201, 47, 3, 119, 18, 82, 153, 31, 241, 58, 239, 112, 155, 205, 252, 176, 47, 68, 10, 176,
        122, 110, 240, 79, 143, 59, 70, 68, 180, 122, 38, 135, 161, 125, 74, 241, 226, 144, 25, 35,
        253, 97, 30, 152, 71, 44, 217, 131, 234, 31, 53, 16, 53, 81, 67, 169, 24, 113, 85, 19, 112,
        102, 16, 181, 56, 41, 60, 116, 221, 115, 162, 174, 248, 174, 210, 87, 84, 34, 83, 16, 64,
        59, 50, 217, 230, 3, 196, 124, 111, 101, 242, 235, 102, 148, 179, 71, 179, 7, 28, 153, 167,
        92, 242, 75, 77, 29, 252, 172, 163, 7, 246, 103, 6, 230, 186, 201, 185, 33, 142, 0, 247,
        134, 5, 22, 48, 241, 180, 131, 87, 91, 29, 17, 232, 17, 100, 180, 217, 137, 181, 53, 21,
        247, 243, 235, 134, 236, 12, 13, 41, 56, 149, 189, 251, 209, 60, 237, 235, 92, 91, 163,
        145, 166, 110, 249, 201, 229, 58, 196, 126, 153, 124, 161, 34, 204, 139, 250, 216, 234,
        101, 63, 185, 84, 4, 215, 175, 1, 0,
    ];
    let proof: PartialMerkleTree = serialization::deserialize(Reader::new(&RAW_PROOF)).expect("");

    let header = generate_blocks_from_raw()[&577696];
    let block_hash = header.hash();
    let info = BtcRelayedTxInfo {
        block_hash,
        merkle_proof: proof,
    };
    (withdraw, info, prev)
}

fn prepare_withdrawal<T: Trait>() -> Transaction {
    // https://btc.com/62c389f1974b8a44737d76f92da0f5cd7f6f48d065e7af6ba368298361141270.rawhex
    const RAW_TX: &'static str = "0100000001052ceda6cf9c93012a994f4ffa2a29c9e31ecf96f472b175eb8e602bfa2b2c5100000000fdfd000047304402200e4d732c456f4722d376252be16554edb27fc93c55db97859e16682bc62b014502202b9c4b01ad55daa1f76e6a564b7762cd0a81240c947806ab3f3b056f2e77c1da01483045022100c7cd680992de60da8c33fc3ef7f5ead85b204660822d9fbda2d85f9fadba732a022021fdc49b20a6007ea971a385732a4065d1d7c792ac9dc391034fb78aa9f5034b014c69522102df92e88c4380778c9c48268460a124a8f4e7da883f80477deaa644ced486efc6210244d81efeb4171b1a8a433b87dd202117f94e44c909c49e42e77b69b5a6ce7d0d2103a36339f413da869df12b1ab0def91749413a0dee87f0bfa85ba7196e6cdad10253aeffffffff03e0349500000000001976a91413256ff2dee6e80c275ddb877abc1ffe453a731488ace00f9700000000001976a914ea6e8dd56703ace584eb9dff0224629f8486672988acc88a02000000000017a914cb94110435d0635223eebe25ed2aaabc03781c458700000000";
    let old_withdraw = RAW_TX.parse::<Transaction>().unwrap();
    // https://btc.com/092684402f9b21abdb1d2d76511d5983bd1250d173ced171a3f76d03fcc43e97.rawhex
    const ANOTHER_TX: &'static str =
        "0100000001059ec66e2a2123364a56bd48f10f57d8a41ecf4082669e6fc85485637043879100000000fdfd00004830450221009fbe7b8f2f4ae771e8773cb5206b9f20286676e2c7cfa98a8e95368acfc3cb3c02203969727a276d7333d5f8815fa364307b8015783cfefbd53def28befdb81855fc0147304402205e5bbe039457d7657bb90dbe63ac30b9547242b44cc03e1f7a690005758e34aa02207208ed76a269d193f1e10583bd902561dbd02826d0486c33a4b1b1839a3d226f014c69522102df92e88c4380778c9c48268460a124a8f4e7da883f80477deaa644ced486efc6210244d81efeb4171b1a8a433b87dd202117f94e44c909c49e42e77b69b5a6ce7d0d2103a36339f413da869df12b1ab0def91749413a0dee87f0bfa85ba7196e6cdad10253aeffffffff04288e0300000000001976a914eb016d7998c88a79a50a0408dd7d5839b1ce1a6888aca0bb0d00000000001976a914646fe05e35369248c3f8deea436dc2b92c7dc86888ac50c30000000000001976a914d1a68d6e891a88d53d9bc3b88d172a3ff6b238c388ac20ee03020000000017a914cb94110435d0635223eebe25ed2aaabc03781c458700000000";
    let tmp = ANOTHER_TX.parse::<Transaction>().unwrap();

    let alice = alice::<T>();
    let bob = bob::<T>();
    let withdrawal_fee = XGatewayBitcoin::<T>::btc_withdrawal_fee();

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

fn create_tx() -> Transaction {
    "0100000001052ceda6cf9c93012a994f4ffa2a29c9e31ecf96f472b175eb8e602bfa2b2c5100000000b40047304402200e4d732c456f4722d376252be16554edb27fc93c55db97859e16682bc62b014502202b9c4b01ad55daa1f76e6a564b7762cd0a81240c947806ab3f3b056f2e77c1da014c69522102df92e88c4380778c9c48268460a124a8f4e7da883f80477deaa644ced486efc6210244d81efeb4171b1a8a433b87dd202117f94e44c909c49e42e77b69b5a6ce7d0d2103a36339f413da869df12b1ab0def91749413a0dee87f0bfa85ba7196e6cdad10253aeffffffff03e0349500000000001976a91413256ff2dee6e80c275ddb877abc1ffe453a731488ace00f9700000000001976a914ea6e8dd56703ace584eb9dff0224629f8486672988acc88a02000000000017a914cb94110435d0635223eebe25ed2aaabc03781c458700000000".parse::<Transaction>().unwrap()
}

fn prepare_headers<T: Trait>(caller: &T::AccountId) {
    for (height, header) in generate_blocks_from_raw() {
        if height == 576576 {
            continue;
        }
        if height == 577700 {
            // confirm for 577696
            break;
        }
        let v = serialization::serialize(&header).into();
        XGatewayBitcoin::<T>::push_header(RawOrigin::Signed(caller.clone()).into(), v).unwrap();
    }
}

benchmarks! {
    _{ }

    push_header {
        let receiver: T::AccountId = whitelisted_caller();
        let insert_height = 576576 + 1;
        let header = generate_blocks_from_raw()[&insert_height];
        let hash = header.hash();
        let header_raw = serialization::serialize(&header).into();
        let amount: BalanceOf<T> = 1000.into();
    }: _(RawOrigin::Signed(receiver), header_raw)
    verify {
        assert!(XGatewayBitcoin::<T>::headers(&hash).is_some());
    }

    push_transaction {
        let n in 1 .. 1024 * 1024 * 500; // 500KB length
        let l in 1 .. 1024 * 1024 * 500; // 500KB length

        let caller: T::AccountId = whitelisted_caller();

        // set_trustee::<T>();
        prepare_headers::<T>(&caller);
        let (tx, info, prev) = withdraw_tx();
        let tx_hash = tx.hash();
        let tx_raw = serialization::serialize(&tx).into();
        let prev_tx_raw = serialization::serialize(&prev).into();

        XGatewayRecords::<T>::deposit(&caller, ASSET_ID, 9778400.into()).unwrap();
        XGatewayRecords::<T>::deposit(&caller, ASSET_ID, 9900000.into()).unwrap();
        XGatewayRecords::<T>::withdraw(&caller, ASSET_ID, 9778400.into(), b"".to_vec(), b"".to_vec().into()).unwrap();
        XGatewayRecords::<T>::withdraw(&caller, ASSET_ID, 9900000.into(), b"".to_vec(), b"".to_vec().into()).unwrap();
        xpallet_gateway_records::WithdrawalStateOf::insert(0, WithdrawalState::Processing);
        xpallet_gateway_records::WithdrawalStateOf::insert(1, WithdrawalState::Processing);

        let proposal = BtcWithdrawalProposal::<T::AccountId> {
            sig_state: VoteResult::Finish,
            withdrawal_id_list: vec![0, 1],
            tx: tx.clone(),
            trustee_list: vec![],
        };
        WithdrawalProposal::<T>::put(proposal);

    }: _(RawOrigin::Signed(caller), tx_raw, info, Some(prev_tx_raw))
    verify {
        assert!(WithdrawalProposal::<T>::get().is_none());
        assert_eq!(TxState::get(tx_hash), Some(BtcTxState {
            result: BtcTxResult::Success,
            tx_type: BtcTxType::Withdrawal,
        }));
    }

    create_withdraw_tx {
        let n in 1 .. 100; // 100 withdrawl count
        let l in 1 .. 1024 * 1024 * 500; // 500KB length

        let caller = alice::<T>();

        let (tx, info, prev) = withdraw_tx();
        let tx_hash = tx.hash();
        let tx_raw: Vec<u8> = serialization::serialize(&tx).into();
        let prev_tx_raw: Vec<u8> = serialization::serialize(&prev).into();

        let btc_withdrawal_fee = XGatewayBitcoin::<T>::btc_withdrawal_fee();
        let first_withdraw = (9778400 + btc_withdrawal_fee).saturated_into();
        let second_withdraw = (9900000 + btc_withdrawal_fee).saturated_into();
        XGatewayRecords::<T>::deposit(&caller, ASSET_ID, first_withdraw).unwrap();
        XGatewayRecords::<T>::deposit(&caller, ASSET_ID, second_withdraw).unwrap();
        XGatewayRecords::<T>::withdraw(&caller, ASSET_ID, first_withdraw, b"12kEgqNShFw7BN27QCMQZCynQpSuV4x1Ax".to_vec(), b"".to_vec().into()).unwrap();
        XGatewayRecords::<T>::withdraw(&caller, ASSET_ID, second_withdraw, b"1NNZZKR6pos2M4yiJhS76NjcRHxoJUATy4".to_vec(), b"".to_vec().into()).unwrap();

        let tx = create_tx();
        let tx_raw: Vec<u8> = serialization::serialize(&tx).into();
    }: _(RawOrigin::Signed(caller), vec![0, 1], tx_raw)
    verify {
        assert!(WithdrawalProposal::<T>::get().is_some());
    }

    sign_withdraw_tx {
        let l in 1 .. 1024 * 1024 * 500; // 500KB length
        let tx = create_tx();
        let alice = alice::<T>();
        let bob = bob::<T>();

        let proposal = BtcWithdrawalProposal::<T::AccountId> {
            sig_state: VoteResult::Unfinish,
            withdrawal_id_list: vec![0, 1],
            tx: tx,
            trustee_list: vec![ (alice, true) ],
        };
        WithdrawalProposal::<T>::put(proposal);

        let (signed_tx, _, _) = withdraw_tx();
        let tx_raw: Vec<u8> = serialization::serialize(&signed_tx).into();
    }: _(RawOrigin::Signed(bob), Some(tx_raw))
    verify {
        assert_eq!(WithdrawalProposal::<T>::get().unwrap().sig_state, VoteResult::Finish);
    }

    set_best_index {
        let best = BtcHeaderIndex {
            hash: H256::repeat_byte(1),
            height: 100,
        };
    }: _(RawOrigin::Root, best)
    verify {
        assert_eq!(XGatewayBitcoin::<T>::best_index(), best);
    }

    set_confirmed_index {
        let confirmed = BtcHeaderIndex {
            hash: H256::repeat_byte(1),
            height: 100,
        };
    }: _(RawOrigin::Root, confirmed)
    verify {
        assert_eq!(XGatewayBitcoin::<T>::confirmed_index(), Some(confirmed));
    }

    remove_pending {
        let addr = b"3AWmpzJ1kSF1cktFTDEb3qmLcdN8YydxA7".to_vec();
        let v = vec![
            BtcDepositCache {
                txid: H256::repeat_byte(1),
                balance: 100000000,
            },
            BtcDepositCache {
                txid: H256::repeat_byte(2),
                balance: 200000000,
            },
            BtcDepositCache {
                txid: H256::repeat_byte(3),
                balance: 300000000,
            },
        ];
        PendingDeposits::insert(&addr, v);
        let receiver: T::AccountId = whitelisted_caller();
    }: _(RawOrigin::Root, addr.clone(), Some(receiver.clone()))
    verify {
        assert!(XGatewayBitcoin::<T>::pending_deposits(&addr).is_empty());
        assert_eq!(XAssets::<T>::usable_balance(&receiver, &ASSET_ID), (100000000 + 200000000 + 300000000).into());
    }

    remove_proposal {
        let (tx, _, _) = withdraw_tx();
        let proposal = BtcWithdrawalProposal::<T::AccountId> {
            sig_state: VoteResult::Unfinish,
            withdrawal_id_list: vec![0, 1],
            tx: tx,
            trustee_list: vec![],
        };
        WithdrawalProposal::<T>::put(proposal);
    }: _(RawOrigin::Root)
    verify {
        assert!(WithdrawalProposal::<T>::get().is_none());
    }

    force_replace_proposal_tx {
        let l in 1 .. 1024 * 1024 * 500; // 500KB length

        Verifier::put(BtcTxVerifier::Test);
        let tx = prepare_withdrawal::<T>();
        let raw = serialization::serialize(&tx);
    }: _(RawOrigin::Root, raw.into())
    verify {
        assert_eq!(WithdrawalProposal::<T>::get().unwrap().tx, tx);
    }

    set_btc_withdrawal_fee {
        let alice = alice::<T>();
    }: _(RawOrigin::Root,  2000000)
    verify {
    }

    set_btc_deposit_limit {
        let alice = bob::<T>();
    }: _(RawOrigin::Root,  2000000)
    verify {
    }
}
/*
#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::mock::{ExtBuilder, Test};
    use frame_support::assert_ok;

    #[test]
    fn test_benchmarks() {
        ExtBuilder::default().build().execute_with(|| {
            assert_ok!(test_benchmark_push_header::<Test>());
        });

        ExtBuilder::default().build().execute_with(|| {
            assert_ok!(test_benchmark_push_transaction::<Test>());
        });

        ExtBuilder::default().build().execute_with(|| {
            assert_ok!(test_benchmark_create_withdraw_tx::<Test>());
        });

        ExtBuilder::default().build().execute_with(|| {
            assert_ok!(test_benchmark_sign_withdraw_tx::<Test>());
        });

        ExtBuilder::default().build().execute_with(|| {
            assert_ok!(test_benchmark_set_best_index::<Test>());
        });

        ExtBuilder::default().build().execute_with(|| {
            assert_ok!(test_benchmark_set_confirmed_index::<Test>());
        });

        ExtBuilder::default().build().execute_with(|| {
            assert_ok!(test_benchmark_remove_pending::<Test>());
        });

        ExtBuilder::default().build().execute_with(|| {
            assert_ok!(test_benchmark_force_replace_proposal_tx::<Test>());
        });

        ExtBuilder::default().build().execute_with(|| {
            assert_ok!(test_benchmark_set_btc_withdrawal_fee::<Test>());
        });

        ExtBuilder::default().build().execute_with(|| {
            assert_ok!(test_benchmark_set_btc_deposit_limit::<Test>());
        });
    }
}
*/
