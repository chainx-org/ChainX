// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use codec::{Decode, Encode};
use frame_benchmarking::benchmarks;
use frame_support::storage::StorageMap;
use frame_system::RawOrigin;
use sp_core::crypto::AccountId32;
use sp_runtime::traits::StaticLookup;
use sp_std::prelude::*;

use chainx_primitives::AssetId;
use xpallet_assets::{BalanceOf, Chain};
use xpallet_gateway_records::{Module as XGatewayRecords, WithdrawalRecordId, WithdrawalState};

use crate::{types::*, Call, Module, Trait, TrusteeMultiSigAddr};

const ASSET_ID: AssetId = xp_protocol::X_BTC;

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
fn charlie<T: Trait>() -> T::AccountId {
    // sr25519 Charlie
    account::<T>("90b5ab205c6974c9ea841be688864633dc9ca8a357843eeacf2314649965fe22")
}
fn new_trustees<T: Trait>() -> Vec<(T::AccountId, Vec<u8>, Vec<u8>, Vec<u8>)> {
    vec![
        (
            alice::<T>(),
            b"Alice".to_vec(),
            hex::decode("02df92e88c4380778c9c48268460a124a8f4e7da883f80477deaa644ced486efc6")
                .unwrap(),
            hex::decode("0386b58f51da9b37e59c40262153173bdb59d7e4e45b73994b99eec4d964ee7e88")
                .unwrap(),
        ),
        (
            bob::<T>(),
            b"Bob".to_vec(),
            hex::decode("0244d81efeb4171b1a8a433b87dd202117f94e44c909c49e42e77b69b5a6ce7d0d")
                .unwrap(),
            hex::decode("02e4631e46255571122d6e11cda75d5d601d5eb2585e65e4e87fe9f68c7838a278")
                .unwrap(),
        ),
        (
            charlie::<T>(),
            b"Charlie".to_vec(),
            hex::decode("03a36339f413da869df12b1ab0def91749413a0dee87f0bfa85ba7196e6cdad102")
                .unwrap(),
            hex::decode("0263d46c760d3e04883d4b433c9ce2bc32130acd9faad0192a2b375dbba9f865c3")
                .unwrap(),
        ),
    ]
}

benchmarks! {
    _{ }

    withdraw {
        let caller: T::AccountId = alice::<T>();
        let amount: BalanceOf<T> = 10_00000000.into();
        XGatewayRecords::<T>::deposit(&caller, ASSET_ID, amount).unwrap();

        let addr = b"3PgYgJA6h5xPEc3HbnZrUZWkpRxuCZVyEP".to_vec();
        let memo = b"".to_vec().into();
    }: _(RawOrigin::Signed(caller.clone()), ASSET_ID, amount, addr, memo)
    verify {
        assert!(XGatewayRecords::<T>::pending_withdrawals(0).is_some());
        assert_eq!(
            XGatewayRecords::<T>::state_of(0),
            Some(WithdrawalState::Applying)
        );
    }

    cancel_withdrawal {
        let caller: T::AccountId = alice::<T>();
        let amount: BalanceOf<T> = 10_00000000.into();
        XGatewayRecords::<T>::deposit(&caller, ASSET_ID, amount).unwrap();

        let withdrawal = amount - 500.into();
        let addr = b"3PgYgJA6h5xPEc3HbnZrUZWkpRxuCZVyEP".to_vec();
        let memo = b"".to_vec().into();
        Module::<T>::withdraw(
            RawOrigin::Signed(caller.clone()).into(),
            ASSET_ID, withdrawal, addr, memo,
        )
        .unwrap();

        let withdrawal_id: WithdrawalRecordId = 0;
        assert!(XGatewayRecords::<T>::pending_withdrawals(withdrawal_id).is_some());
        assert_eq!(
            XGatewayRecords::<T>::state_of(withdrawal_id),
            Some(WithdrawalState::Applying)
        );

    }: _(RawOrigin::Signed(caller.clone()), withdrawal_id)
    verify {
        assert!(XGatewayRecords::<T>::pending_withdrawals(withdrawal_id).is_none());
        assert!(XGatewayRecords::<T>::state_of(withdrawal_id).is_none());
    }

    setup_trustee {
        let caller: T::AccountId = alice::<T>();
        let hot = hex::decode("02df92e88c4380778c9c48268460a124a8f4e7da883f80477deaa644ced486efc6")
                .unwrap();
        let cold = hex::decode("0386b58f51da9b37e59c40262153173bdb59d7e4e45b73994b99eec4d964ee7e88")
                .unwrap();

        assert!(Module::<T>::trustee_intention_props_of(caller.clone(), Chain::Bitcoin).is_none());
    }: _(RawOrigin::Signed(caller.clone()), Chain::Bitcoin, b"about".to_vec(), hot, cold)
    verify {
        assert!(Module::<T>::trustee_intention_props_of(caller, Chain::Bitcoin).is_some());
    }

    transition_trustee_session {
        let caller: T::AccountId = alice::<T>();
        TrusteeMultiSigAddr::<T>::insert(Chain::Bitcoin, caller.clone());

        let mut candidators = vec![];
        for (account, about, hot, cold) in new_trustees::<T>() {
            Module::<T>::setup_trustee_impl(account.clone(), Chain::Bitcoin, about, hot, cold).unwrap();
            candidators.push(account);
        }

        assert_eq!(Module::<T>::next_trustee_session_info_number_of(Chain::Bitcoin), 0);
        assert!(Module::<T>::trustee_session_info_of(Chain::Bitcoin, 0).is_none());
    }: _(RawOrigin::Signed(caller.clone()), Chain::Bitcoin, candidators)
    verify {
        assert_eq!(Module::<T>::next_trustee_session_info_number_of(Chain::Bitcoin), 1);
        assert!(Module::<T>::trustee_session_info_of(Chain::Bitcoin, 0).is_some());
    }

    set_withdrawal_state {
        let caller: T::AccountId = alice::<T>();
        TrusteeMultiSigAddr::<T>::insert(Chain::Bitcoin, caller.clone());

        let amount: BalanceOf<T> = 10_00000000.into();
        XGatewayRecords::<T>::deposit(&caller, ASSET_ID, amount).unwrap();

        let withdrawal = amount - 500.into();
        let addr = b"3PgYgJA6h5xPEc3HbnZrUZWkpRxuCZVyEP".to_vec();
        let memo = b"".to_vec().into();
        Module::<T>::withdraw(
            RawOrigin::Signed(caller.clone()).into(),
            ASSET_ID, withdrawal, addr, memo,
        )
        .unwrap();

        let withdrawal_id: WithdrawalRecordId = 0;
        assert!(XGatewayRecords::<T>::pending_withdrawals(withdrawal_id).is_some());
        assert_eq!(XGatewayRecords::<T>::state_of(withdrawal_id), Some(WithdrawalState::Applying));

        XGatewayRecords::<T>::process_withdrawal(withdrawal_id, Chain::Bitcoin).unwrap();
        assert_eq!(XGatewayRecords::<T>::state_of(withdrawal_id), Some(WithdrawalState::Processing));
    }: _(RawOrigin::Signed(caller.clone()), withdrawal_id, WithdrawalState::RootFinish)
    verify {
        assert!(XGatewayRecords::<T>::pending_withdrawals(withdrawal_id).is_none());
    }

    set_trustee_info_config {
        let config = TrusteeInfoConfig {
            min_trustee_count: 5,
            max_trustee_count: 15,
        };
    }: _(RawOrigin::Root, Chain::Bitcoin, config.clone())
    verify {
        assert_eq!(Module::<T>::trustee_info_config_of(Chain::Bitcoin), config);
    }

    force_set_referral_binding {
        let who: T::AccountId = alice::<T>();
        let who_lookup: <T::Lookup as StaticLookup>::Source = T::Lookup::unlookup(who.clone());
    }: _(RawOrigin::Root, Chain::Bitcoin, who_lookup.clone(), who_lookup.clone())
    verify {
        assert_eq!(Module::<T>::referral_binding_of(&who, Chain::Bitcoin), Some(who));
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
            assert_ok!(test_benchmark_withdraw::<Test>());
            assert_ok!(test_benchmark_cancel_withdrawal::<Test>());
            assert_ok!(test_benchmark_setup_trustee::<Test>());
            assert_ok!(test_benchmark_transition_trustee_session::<Test>());
            assert_ok!(test_benchmark_set_withdrawal_state::<Test>());
            assert_ok!(test_benchmark_set_trustee_info_config::<Test>());
            assert_ok!(test_benchmark_force_set_referral_binding::<Test>());
        });
    }
}
