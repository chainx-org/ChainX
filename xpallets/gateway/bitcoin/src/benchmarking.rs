// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use codec::{Decode, Encode};
use frame_benchmarking::{benchmarks, whitelisted_caller};
use frame_system::RawOrigin;
use sp_runtime::{AccountId32, SaturatedConversion};
use sp_std::{collections::btree_map::BTreeMap, prelude::*};

use chainx_primitives::AssetId;
use xp_gateway_bitcoin::BtcTxType;
use xpallet_gateway_records::{Pallet as XGatewayRecords, WithdrawalState};

use light_bitcoin::{
    chain::{BlockHeader, Transaction},
    merkle::PartialMerkleTree,
    primitives::H256,
    serialization::{self, Reader, SERIALIZE_TRANSACTION_WITNESS},
};

use crate::{
    types::*, BalanceOf, Call, Config, Pallet, PendingDeposits, TransactionOutputArray, TxState,
    WithdrawalProposal,
};

const ASSET_ID: AssetId = xp_protocol::X_BTC;

fn generate_blocks_63290_63310() -> BTreeMap<u32, BlockHeader> {
    let bytes = include_bytes!("./res/headers-63290-63310.raw");
    Decode::decode(&mut &bytes[..]).unwrap()
}

fn account<T: Config>(pubkey: &str) -> T::AccountId {
    let pubkey = hex::decode(pubkey).unwrap();
    let mut public = [0u8; 32];
    public.copy_from_slice(pubkey.as_slice());
    let account = AccountId32::from(public).encode();
    Decode::decode(&mut account.as_slice()).unwrap()
}

fn alice<T: Config>() -> T::AccountId {
    // sr25519 Alice
    account::<T>("d43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d")
}

// fn bob<T: Config>() -> T::AccountId {
//     // sr25519 Bob
//     account::<T>("8eaf04151687736326c9fea17e25fc5287613693c912909cb226aa4794f26a48")
// }

fn withdraw_tx() -> (Transaction, BtcRelayedTxInfo, Transaction) {
    // block height: 63299
    // https://signet.bitcoinexplorer.org/tx/0f592933b493bedab209851cb2cf07871558ff57d86d645877b16651479b51a2
    const RAW_TX: &str = "020000000001015fea22ec1a3e3e7e1167fa220cc8376225f07bd20aa194e7f3c4ac68c7375d8e0000000000000000000250c3000000000000225120c9929543dfa1e0bb84891acd47bfa6546b05e26b7a04af8eb6765fcc969d565f409c0000000000002251209a9ea267884f5549c206b2aec2bd56d98730f90532ea7f7154d4d4f923b7e3bb03402639d4d9882f6e7e42db38dbd2845c87b131737bf557643ef575c49f8fc6928869d9edf5fd61606fb07cced365fdc2c7b637e6ecc85b29906c16d314e7543e94222086a60c7d5dd3f4931cc8ad77a614402bdb591c042347c89281c48c7e9439be9dac61c0e56a1792f348690cdeebe60e3db6c4e94d94e742c619f7278e52f6cbadf5efe96a528ba3f61a5b0d4fbceea425a9028381458b32492bccc3f1faa473a649e23605554f5ea4b4044229173719228a35635eeffbd8a8fe526270b737ad523b99f600000000";
    let tx = RAW_TX.parse::<Transaction>().unwrap();

    // https://signet.bitcoinexplorer.org/tx/8e5d37c768acc4f3e794a10ad27bf0256237c80c22fa67117e3e3e1aec22ea5f
    const RAW_TX_PREV: &str = "02000000000101aeee49e0bbf7a36f78ea4321b5c8bae0b8c72bdf2c024d2484b137fa7d0f8e1f01000000000000000003a0860100000000002251209a9ea267884f5549c206b2aec2bd56d98730f90532ea7f7154d4d4f923b7e3bb0000000000000000326a3035516a706f3772516e7751657479736167477a6334526a376f737758534c6d4d7141754332416255364c464646476a38801a060000000000225120c9929543dfa1e0bb84891acd47bfa6546b05e26b7a04af8eb6765fcc969d565f01409e325889515ed47099fdd7098e6fafdc880b21456d3f368457de923f4229286e34cef68816348a0581ae5885ede248a35ac4b09da61a7b9b90f34c200872d2e300000000";
    let prev_tx = RAW_TX_PREV.parse::<Transaction>().unwrap();

    const RAW_PROOF: &str = "0a00000005b82e08a0de8576c7e073c97ce57656bfdcee783dc2f2d9d2b484a4914c0a5a22a2519b475166b17758646dd857ff58158707cfb21c8509b2dabe93b43329590f513fea84b67b1c21bdf184e97a64ac2d4744570d1959203f5e992e314de4385d1687d11a3fd8f21e2105a52a3c36a17ea870e326ecddb23221d4cc0398b6c44bdcce3f191919a31f4cfaca5a786cc8315db76683ad6b8008f2ed9b348df76a0d023700";
    let proof = hex::decode(RAW_PROOF).unwrap();
    let merkle_proof: PartialMerkleTree = serialization::deserialize(Reader::new(&proof)).unwrap();

    let header = generate_blocks_63290_63310()[&63299];
    let info = BtcRelayedTxInfo {
        block_hash: header.hash(),
        merkle_proof,
    };
    (tx, info, prev_tx)
}

// push header 63290 - 63310
fn prepare_headers<T: Config>(caller: &T::AccountId) {
    for (height, header) in generate_blocks_63290_63310() {
        if height == 63290 {
            continue;
        }
        if height == 63307 {
            break;
        }
        let header = serialization::serialize(&header).into();
        Pallet::<T>::push_header(RawOrigin::Signed(caller.clone()).into(), header).unwrap();
    }
}

benchmarks! {
    push_header {
        let receiver: T::AccountId = whitelisted_caller();
        let insert_height = 63290 + 1;
        let header = generate_blocks_63290_63310()[&insert_height];
        let hash = header.hash();
        let header_raw = serialization::serialize(&header).into();
        let amount: BalanceOf<T> = 1000u32.into();
    }: _(RawOrigin::Signed(receiver), header_raw)
    verify {
        assert!(Pallet::<T>::headers(&hash).is_some());
    }

    push_transaction {
        let n = 1024 * 1024 * 500; // 500KB length
        let l = 1024 * 1024 * 500; // 500KB length

        let caller: T::AccountId = alice::<T>();

        prepare_headers::<T>(&caller);
        let (tx, info, prev_tx) = withdraw_tx();
        let tx_hash = tx.hash();
        let tx_raw = serialization::serialize_with_flags(&tx, SERIALIZE_TRANSACTION_WITNESS).into();
        let prev_tx_raw = serialization::serialize_with_flags(&prev_tx, SERIALIZE_TRANSACTION_WITNESS).into();

        XGatewayRecords::<T>::deposit(&caller, ASSET_ID, 100_000u32.into()).unwrap();
        XGatewayRecords::<T>::withdraw(&caller, ASSET_ID, 50_000u32.into(), b"tb1pexff2s7l58sthpyfrtx500ax234stcnt0gz2lr4kwe0ue95a2e0srxsc68".to_vec(), b"".to_vec().into()).unwrap();

        XGatewayRecords::<T>::withdrawal_state_insert(0, WithdrawalState::Processing);

        let proposal = BtcWithdrawalProposal::<T::AccountId> {
            sig_state: VoteResult::Finish,
            withdrawal_id_list: vec![0],
            tx,
            trustee_list: vec![],
        };
        WithdrawalProposal::<T>::put(proposal);

    }: _(RawOrigin::Signed(caller), tx_raw, info, Some(prev_tx_raw))
    verify {
        assert!(WithdrawalProposal::<T>::get().is_none());
        assert_eq!(
            TxState::<T>::get(tx_hash),
            Some(BtcTxState {
                tx_type: BtcTxType::Withdrawal,
                result: BtcTxResult::Success,
            })
        );
    }

    create_taproot_withdraw_tx {
        let n = 100;                // 100 withdrawal count
        let l = 1024 * 1024 * 500;  // 500KB length

        let caller = alice::<T>();

        let (tx, info, prev_tx) = withdraw_tx();
        let tx_hash = tx.hash();
        let tx_raw: Vec<u8> = serialization::serialize_with_flags(&tx, SERIALIZE_TRANSACTION_WITNESS).into();

        let transaction_output = TransactionOutputArray {
            outputs: vec![prev_tx.outputs[0].clone()],
        };
        let spent_outputs_raw = serialization::serialize(&transaction_output).into();

        XGatewayRecords::<T>::deposit(&caller, ASSET_ID, 100_000u32.saturated_into()).unwrap();
        XGatewayRecords::<T>::withdraw(&caller, ASSET_ID, 50_000u32.saturated_into(), b"tb1pexff2s7l58sthpyfrtx500ax234stcnt0gz2lr4kwe0ue95a2e0srxsc68".to_vec(), b"".to_vec().into()).unwrap();

        XGatewayRecords::<T>::withdrawal_state_insert(0, WithdrawalState::Applying);

    }: _(RawOrigin::Signed(caller), vec![0], tx_raw, spent_outputs_raw)
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
        assert_eq!(Pallet::<T>::best_index(), best);
    }

    set_confirmed_index {
        let confirmed = BtcHeaderIndex {
            hash: H256::repeat_byte(1),
            height: 100,
        };
    }: _(RawOrigin::Root, confirmed)
    verify {
        assert_eq!(Pallet::<T>::confirmed_index(), Some(confirmed));
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
        PendingDeposits::<T>::insert(&addr, v);
        let receiver: T::AccountId = whitelisted_caller();
    }: _(RawOrigin::Root, addr.clone(), Some(receiver.clone()))
    verify {
        assert!(Pallet::<T>::pending_deposits(&addr).is_empty());
        // assert_eq!(XAssets::<T>::usable_balance(&receiver, &ASSET_ID), (100000000u32 + 200000000u32 + 300000000u32).into());
    }

    remove_proposal {
        let (tx, _, _) = withdraw_tx();
        let proposal = BtcWithdrawalProposal::<T::AccountId> {
            sig_state: VoteResult::Unfinish,
            withdrawal_id_list: vec![0, 1],
            tx,
            trustee_list: vec![],
        };
        WithdrawalProposal::<T>::put(proposal);
    }: _(RawOrigin::Root)
    verify {
        assert!(WithdrawalProposal::<T>::get().is_none());
    }

    set_btc_withdrawal_fee {
        let caller = alice::<T>();
    }: _(RawOrigin::Root,  2000000)
    verify {
    }

    set_btc_deposit_limit {
        let caller = alice::<T>();
    }: _(RawOrigin::Root,  2000000)
    verify {
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::{ExtBuilder, Test};
    use frame_support::assert_ok;

    #[test]
    fn test_benchmarks() {
        ExtBuilder::default().build().execute_with(|| {
            assert_ok!(Pallet::<Test>::test_benchmark_push_header());
            assert_ok!(Pallet::<Test>::test_benchmark_push_transaction());
            assert_ok!(Pallet::<Test>::test_benchmark_create_taproot_withdraw_tx());
            assert_ok!(Pallet::<Test>::test_benchmark_set_best_index());
            assert_ok!(Pallet::<Test>::test_benchmark_set_confirmed_index());
            assert_ok!(Pallet::<Test>::test_benchmark_remove_pending());
            assert_ok!(Pallet::<Test>::test_benchmark_set_btc_withdrawal_fee());
            assert_ok!(Pallet::<Test>::test_benchmark_set_btc_deposit_limit());
        });
    }
}
