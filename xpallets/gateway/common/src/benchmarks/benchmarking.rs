// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use crate::*;
use codec::{Decode, Encode};

use frame_benchmarking::benchmarks;
use frame_system::RawOrigin;
use sp_core::crypto::AccountId32;
use sp_std::prelude::*;

use chainx_primitives::AssetId;

use crate::Module as XGatewayCommon;
use xpallet_gateway_records::Module as XGatewayRecords;

const ASSET_ID: AssetId = xpallet_protocol::X_BTC;

benchmarks! {
    _{ }

    withdraw {
        let caller: T::AccountId = accounts::<T>()[0].clone();

        let amount: BalanceOf<T> = 10_00000000.into();
        XGatewayRecords::<T>::deposit(&caller, ASSET_ID, amount).unwrap();

        let addr = b"3PgYgJA6h5xPEc3HbnZrUZWkpRxuCZVyEP".to_vec();
        let memo = b"".to_vec().into();

    }: _(RawOrigin::Signed(caller.clone()), ASSET_ID, amount, addr, memo)
    verify {
        assert!(XGatewayRecords::<T>::pending_withdrawals(0).is_some());
    }

    setup_trustee {
        let caller: T::AccountId = accounts::<T>()[0].clone();

        let hot = PUBKEYS[1].0.to_vec();
        let cold = PUBKEYS[1].1.to_vec();
    }: _(RawOrigin::Signed(caller.clone()), Chain::Bitcoin, b"about".to_vec(), hot, cold)
    verify {
        assert!(XGatewayCommon::<T>::trustee_intention_props_of(caller, Chain::Bitcoin).is_some());
    }

    transition_trustee_session {
        let u in 1 .. 64 => ();

        let caller: T::AccountId = accounts::<T>()[0].clone();
        TrusteeMultiSigAddr::<T>::insert(Chain::Bitcoin, caller.clone());
        let candidators = prepare_intention::<T>();

        assert!(XGatewayCommon::<T>::trustee_session_info_of(Chain::Bitcoin, 0).is_some());
    }: _(RawOrigin::Signed(caller.clone()), Chain::Bitcoin, candidators)
    verify {
        assert!(XGatewayCommon::<T>::trustee_session_info_of(Chain::Bitcoin, 1).is_some());
    }

    set_withdrawal_state {
        let caller: T::AccountId = accounts::<T>()[0].clone();
        TrusteeMultiSigAddr::<T>::insert(Chain::Bitcoin, caller.clone());

        let amount: BalanceOf<T> = 10_00000000.into();
        deposit_and_withdraw::<T>(caller.clone(), amount);

        assert!(XGatewayRecords::<T>::pending_withdrawals(0).is_some());

    }: _(RawOrigin::Signed(caller.clone()), 0, WithdrawalState::RootFinish)
    verify {
        assert!(XGatewayRecords::<T>::pending_withdrawals(0).is_none());
    }

    set_trustee_info_config {
        let config = TrusteeInfoConfig {
            min_trustee_count: 5,
            max_trustee_count: 15,
        };
    }: _(RawOrigin::Root, Chain::Bitcoin, config.clone())
    verify {
        assert_eq!(XGatewayCommon::<T>::trustee_info_config(Chain::Bitcoin), config);
    }

    force_set_binding {
        let who: T::AccountId = accounts::<T>()[0].clone();
        let who_lookup: <T::Lookup as StaticLookup>::Source = T::Lookup::unlookup(who.clone());
    }: _(RawOrigin::Root, Chain::Bitcoin, who_lookup.clone(), who_lookup.clone())
    verify {
        assert_eq!(XGatewayCommon::<T>::channel_binding_of(&who, Chain::Bitcoin), Some(who));
    }
}

const PUBKEYS: [([u8; 33], [u8; 33]); 3] = [
    (
        [
            2, 223, 146, 232, 140, 67, 128, 119, 140, 156, 72, 38, 132, 96, 161, 36, 168, 244, 231,
            218, 136, 63, 128, 71, 125, 234, 166, 68, 206, 212, 134, 239, 198,
        ],
        [
            3, 134, 181, 143, 81, 218, 155, 55, 229, 156, 64, 38, 33, 83, 23, 59, 219, 89, 215,
            228, 228, 91, 115, 153, 75, 153, 238, 196, 217, 100, 238, 126, 136,
        ],
    ),
    (
        [
            2, 68, 216, 30, 254, 180, 23, 27, 26, 138, 67, 59, 135, 221, 32, 33, 23, 249, 78, 68,
            201, 9, 196, 158, 66, 231, 123, 105, 181, 166, 206, 125, 13,
        ],
        [
            2, 228, 99, 30, 70, 37, 85, 113, 18, 45, 110, 17, 205, 167, 93, 93, 96, 29, 94, 178,
            88, 94, 101, 228, 232, 127, 233, 246, 140, 120, 56, 162, 120,
        ],
    ),
    (
        [
            3, 163, 99, 57, 244, 19, 218, 134, 157, 241, 43, 26, 176, 222, 249, 23, 73, 65, 58, 13,
            238, 135, 240, 191, 168, 91, 167, 25, 110, 108, 218, 209, 2,
        ],
        [
            2, 99, 212, 108, 118, 13, 62, 4, 136, 61, 75, 67, 60, 156, 226, 188, 50, 19, 10, 205,
            159, 170, 208, 25, 42, 43, 55, 93, 187, 169, 248, 101, 195,
        ],
    ),
];

fn accounts<T: Trait>() -> [T::AccountId; 3] {
    // sr25519 generate pubkey
    let alice = [
        212, 53, 147, 199, 21, 253, 211, 28, 97, 20, 26, 189, 4, 169, 159, 214, 130, 44, 133, 88,
        133, 76, 205, 227, 154, 86, 132, 231, 165, 109, 162, 125,
    ];

    let bob = [
        142, 175, 4, 21, 22, 135, 115, 99, 38, 201, 254, 161, 126, 37, 252, 82, 135, 97, 54, 147,
        201, 18, 144, 156, 178, 38, 170, 71, 148, 242, 106, 72,
    ];

    let charlie = [
        144, 181, 171, 32, 92, 105, 116, 201, 234, 132, 27, 230, 136, 134, 70, 51, 220, 156, 168,
        163, 87, 132, 62, 234, 207, 35, 20, 100, 153, 101, 254, 34,
    ];
    let alice: AccountId32 = alice.into();
    let bob: AccountId32 = bob.into();
    let charlie: AccountId32 = charlie.into();

    let a = alice.encode();
    let alice = T::AccountId::decode(&mut &a[..]).unwrap();
    let b = bob.encode();
    let bob = T::AccountId::decode(&mut &b[..]).unwrap();
    let c = charlie.encode();
    let charlie = T::AccountId::decode(&mut &c[..]).unwrap();
    [alice, bob, charlie]
}

fn trustees<T: Trait>() -> Vec<(T::AccountId, Vec<u8>, Vec<u8>, Vec<u8>)> {
    let accounts = accounts::<T>();
    let btc_trustees = vec![
        (
            accounts[0].clone(),
            b"".to_vec(),
            PUBKEYS[0].0.to_vec(),
            PUBKEYS[0].1.to_vec(),
        ),
        (
            accounts[1].clone(),
            b"".to_vec(),
            PUBKEYS[1].0.to_vec(),
            PUBKEYS[1].1.to_vec(),
        ),
        (
            accounts[2].clone(),
            b"".to_vec(),
            PUBKEYS[2].0.to_vec(),
            PUBKEYS[2].1.to_vec(),
        ),
    ];
    btc_trustees
}
fn prepare_intention<T: Trait>() -> Vec<T::AccountId> {
    let mut v = vec![];
    for (account, about, hot, cold) in trustees::<T>() {
        XGatewayCommon::<T>::setup_trustee_impl(account.clone(), Chain::Bitcoin, about, hot, cold)
            .unwrap();
        v.push(account);
    }
    v
}
fn deposit<T: Trait>(who: T::AccountId, amount: BalanceOf<T>) {
    let _ = XGatewayRecords::<T>::deposit(&who, ASSET_ID, amount);
}

fn deposit_and_withdraw<T: Trait>(who: T::AccountId, amount: BalanceOf<T>) {
    deposit::<T>(who.clone(), amount);
    let withdrawal = amount - 500.into();
    let addr = b"3LFSUKkP26hun42J1Dy6RATsbgmBJb27NF".to_vec();
    let memo = b"memo".to_vec().into();
    XGatewayCommon::<T>::withdraw(
        RawOrigin::Signed(who).into(),
        ASSET_ID,
        withdrawal,
        addr,
        memo,
    )
    .unwrap();
    assert_eq!(
        XGatewayRecords::<T>::state_of(0),
        Some(WithdrawalState::Applying)
    );
    xpallet_gateway_records::WithdrawalStateOf::insert(0, WithdrawalState::Processing);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::brenchmarks::{ExtBuilder, Test};
    use frame_support::assert_ok;

    #[test]
    fn test_benchmarks() {
        ExtBuilder::default().build().execute_with(|| {
            assert_ok!(test_benchmark_withdraw::<Test>());
        });
        ExtBuilder::default().build().execute_with(|| {
            assert_ok!(test_benchmark_setup_trustee::<Test>());
        });
        ExtBuilder::default().build().execute_with(|| {
            assert_ok!(test_benchmark_transition_trustee_session::<Test>());
        });
        ExtBuilder::default().build().execute_with(|| {
            assert_ok!(test_benchmark_set_withdrawal_state::<Test>());
        });
        ExtBuilder::default().build().execute_with(|| {
            assert_ok!(test_benchmark_set_trustee_info_config::<Test>());
        });
        ExtBuilder::default().build().execute_with(|| {
            assert_ok!(test_benchmark_force_set_binding::<Test>());
        });
    }
}
