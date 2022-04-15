// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use codec::{Decode, Encode};
use frame_benchmarking::benchmarks;
use frame_support::traits::Currency;
use frame_system::RawOrigin;
use sp_core::crypto::AccountId32;
#[cfg(feature = "runtime-benchmarks")]
use sp_runtime::traits::CheckedDiv;
use sp_runtime::traits::StaticLookup;
use sp_std::prelude::*;

use xp_assets_registrar::Chain;
use xp_protocol::X_BTC;
use xpallet_assets::BalanceOf;
use xpallet_gateway_records::{Pallet as XGatewayRecords, WithdrawalRecordId, WithdrawalState};

use crate::{
    traits::TrusteeSession, types::*, Call, Config, LittleBlackHouse, Pallet,
    TrusteeIntentionPropertiesOf, TrusteeMultiSigAddr, TrusteeSessionInfoLen, TrusteeSessionInfoOf,
    TrusteeTransitionStatus,
};

#[cfg(feature = "runtime-benchmarks")]
fn update_trustee_info<T: Config>(session_num: u32) {
    TrusteeSessionInfoOf::<T>::mutate(Chain::Bitcoin, session_num, |info| match info {
        None => (),
        Some(trustee) => {
            for i in 0..trustee.0.trustee_list.len() {
                trustee.0.trustee_list[i].1 = i as u64 + 1;
            }
            let end_height = 10u32.into();
            trustee.0.end_height = Some(end_height);
        }
    });
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
fn bob<T: Config>() -> T::AccountId {
    // sr25519 Bob
    account::<T>("8eaf04151687736326c9fea17e25fc5287613693c912909cb226aa4794f26a48")
}
fn charlie<T: Config>() -> T::AccountId {
    // sr25519 Charlie
    account::<T>("90b5ab205c6974c9ea841be688864633dc9ca8a357843eeacf2314649965fe22")
}
fn dave<T: Config>() -> T::AccountId {
    // sr25519 Dave
    account::<T>("306721211d5404bd9da88e0204360a1a9ab8b87c66c1bc2fcdd37f3c2222cc20")
}
fn new_trustees<T: Config>() -> Vec<(T::AccountId, Vec<u8>, Vec<u8>, Vec<u8>)> {
    vec![
        (
            alice::<T>(),
            b"Alice".to_vec(),
            hex::decode("0386b58f51da9b37e59c40262153173bdb59d7e4e45b73994b99eec4d964ee7e88")
                .unwrap(),
            hex::decode("02df92e88c4380778c9c48268460a124a8f4e7da883f80477deaa644ced486efc6")
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
            dave::<T>(),
            b"Charlie".to_vec(),
            hex::decode("0263d46c760d3e04883d4b433c9ce2bc32130acd9faad0192a2b375dbba9f865c3")
                .unwrap(),
            hex::decode("03a36339f413da869df12b1ab0def91749413a0dee87f0bfa85ba7196e6cdad102")
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

/// removes all the storage items to reverse any genesis state.
fn clean<T: Config>() {
    <LittleBlackHouse<T>>::remove_all(None);
    <TrusteeSessionInfoLen<T>>::remove_all(None);
    <TrusteeSessionInfoOf<T>>::remove_all(None);
}

benchmarks! {
    withdraw {
        let caller: T::AccountId = alice::<T>();
        let amount: BalanceOf<T> = 1_000_000_000u32.into();
        XGatewayRecords::<T>::deposit(&caller, X_BTC, amount).unwrap();
        let withdrawal = 100_000_000u32.into();
        let addr = b"3PgYgJA6h5xPEc3HbnZrUZWkpRxuCZVyEP".to_vec();
        let memo = b"".to_vec().into();
    }: _(RawOrigin::Signed(caller.clone()), X_BTC, withdrawal, addr, memo)
    verify {
        assert!(XGatewayRecords::<T>::pending_withdrawals(0).is_some());
        assert_eq!(
            XGatewayRecords::<T>::state_of(0),
            Some(WithdrawalState::Applying)
        );
    }

    cancel_withdrawal {
        let caller: T::AccountId = alice::<T>();
        let amount: BalanceOf<T> = 1_000_000_000_u32.into();
        XGatewayRecords::<T>::deposit(&caller, X_BTC, amount).unwrap();

        let withdrawal = 100_000_000u32.into();
        let addr = b"3PgYgJA6h5xPEc3HbnZrUZWkpRxuCZVyEP".to_vec();
        let memo = b"".to_vec().into();
        Pallet::<T>::withdraw(
            RawOrigin::Signed(caller.clone()).into(),
            X_BTC, withdrawal, addr, memo,
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
        clean::<T>();
        <TrusteeIntentionPropertiesOf<T>>::remove(caller.clone(), Chain::Bitcoin);
        LittleBlackHouse::<T>::mutate(Chain::Bitcoin, |acc| acc.push(caller.clone()));
        let hot = hex::decode("02df92e88c4380778c9c48268460a124a8f4e7da883f80477deaa644ced486efc6")
                .unwrap();
        let cold = hex::decode("0386b58f51da9b37e59c40262153173bdb59d7e4e45b73994b99eec4d964ee7e88")
                .unwrap();

        assert!(Pallet::<T>::trustee_intention_props_of(caller.clone(), Chain::Bitcoin).is_none());
    }: _(RawOrigin::Signed(caller.clone()), None, Chain::Bitcoin, b"about".to_vec(), hot, cold)
    verify {
        assert!(Pallet::<T>::trustee_intention_props_of(caller, Chain::Bitcoin).is_some());
    }

    set_trustee_proxy {
        let caller: T::AccountId = alice::<T>();
        assert!(Pallet::<T>::trustee_intention_props_of(caller.clone(), Chain::Bitcoin).is_some());
    }: _(RawOrigin::Signed(caller.clone()), bob::<T>(), Chain::Bitcoin)
    verify {
        assert_eq!(
            Pallet::<T>::trustee_intention_props_of(caller, Chain::Bitcoin).unwrap().0.proxy_account,
            Some(bob::<T>())
        );
    }

    set_trustee_info_config {
        let config = TrusteeInfoConfig {
            min_trustee_count: 5,
            max_trustee_count: 15,
        };
    }: _(RawOrigin::Root, Chain::Bitcoin, config.clone())
    verify {
        assert_eq!(Pallet::<T>::trustee_info_config_of(Chain::Bitcoin), config);
    }

    set_trustee_admin {
        let who: T::AccountId = alice::<T>();
        for (account, about, hot, cold) in new_trustees::<T>() {
            Pallet::<T>::setup_trustee_impl(account.clone(), None, Chain::Bitcoin, about, hot, cold).unwrap();
        }
        let chain = Chain::Bitcoin;
    }: _(RawOrigin::Root, who.clone(), chain)
    verify {
        assert_eq!(Pallet::<T>::trustee_admin(chain), who);
    }

    set_trustee_admin_multiply {
        let multiply = 12;
    }: _(RawOrigin::Root, Chain::Bitcoin, multiply)
    verify{
        assert_eq!(Pallet::<T>::trustee_admin_multiply(Chain::Bitcoin), multiply);
    }

    claim_trustee_reward {
        let caller: T::AccountId = alice::<T>();
        clean::<T>();
        TrusteeMultiSigAddr::<T>::insert(Chain::Bitcoin, caller.clone());
        assert_eq!(Pallet::<T>::trustee_session_info_len(Chain::Bitcoin), 0);
        assert!(Pallet::<T>::trustee_session_info_of(Chain::Bitcoin, 0).is_none());
        let mut candidators = vec![];
        let trustee_info = new_trustees::<T>();
        let trustee_len = trustee_info.len();
        for (account, about, hot, cold) in (&trustee_info[0..trustee_len-1]).to_vec() {
            Pallet::<T>::setup_trustee_impl(account.clone(), None, Chain::Bitcoin, about, hot, cold).unwrap();
            candidators.push(account);
        }
        assert_eq!(Pallet::<T>::transition_trustee_session_impl(Chain::Bitcoin, candidators), Ok(()));

        let mut candidators = vec![];
        let trustee_info = new_trustees::<T>();
        let trustee_len = trustee_info.len();
        for (account, about, hot, cold) in (&trustee_info[1..trustee_len]).to_vec() {
            Pallet::<T>::setup_trustee_impl(account.clone(), None, Chain::Bitcoin, about, hot, cold).unwrap();
            candidators.push(account);
        }
        assert_eq!(Pallet::<T>::transition_trustee_session_impl(Chain::Bitcoin, candidators), Ok(()));
        assert_eq!(Pallet::<T>::trustee_session_info_len(Chain::Bitcoin), 2);
        assert!(Pallet::<T>::trustee_session_info_of(Chain::Bitcoin, 2).is_some());
        let reward: BalanceOf<T> = 100_000_000u32.into();
        let session_num = 1;
        #[cfg(feature = "runtime-benchmarks")]
        update_trustee_info::<T>(session_num);
        #[cfg(feature = "runtime-benchmarks")]
        let reward: BalanceOf<T> = <T as xpallet_assets::Config>::Currency::free_balance(&caller).checked_div(&2u32.into()).unwrap();
        let multi_account = <T as crate::Config>::BitcoinTrusteeSessionProvider::trustee_session(session_num).unwrap().multi_account.unwrap();
        <T as xpallet_assets::Config>::Currency::deposit_creating(&multi_account, reward);
    }: _(RawOrigin::Signed(caller.clone()), Chain::Bitcoin, session_num as i32)
    verify {
        #[cfg(not(feature = "runtime-benchmarks"))]
        assert_eq!(<T as xpallet_assets::Config>::Currency::free_balance(&trustee_info[0].0), 33333333u32.into());
    }

    force_trustee_election {
        TrusteeTransitionStatus::<T>::insert(Chain::Bitcoin, true);
    }: _(RawOrigin::Root, Chain::Bitcoin)
    verify {
        assert!(!Pallet::<T>::trustee_transition_status(Chain::Bitcoin));
    }

    force_update_trustee {
        let caller: T::AccountId = alice::<T>();
        clean::<T>();
        <TrusteeIntentionPropertiesOf<T>>::remove(caller.clone(), Chain::Bitcoin);
        LittleBlackHouse::<T>::mutate(Chain::Bitcoin, |acc| acc.push(caller.clone()));
        let hot = hex::decode("02df92e88c4380778c9c48268460a124a8f4e7da883f80477deaa644ced486efc6")
                .unwrap();
        let cold = hex::decode("0386b58f51da9b37e59c40262153173bdb59d7e4e45b73994b99eec4d964ee7e88")
                .unwrap();

        assert!(Pallet::<T>::trustee_intention_props_of(caller.clone(), Chain::Bitcoin).is_none());
    }: _(RawOrigin::Root, caller.clone(), None, Chain::Bitcoin, b"about".to_vec(), hot, cold)
    verify {
        assert!(Pallet::<T>::trustee_intention_props_of(caller, Chain::Bitcoin).is_some());
    }

    force_set_referral_binding {
        let who: T::AccountId = alice::<T>();
        let who_lookup: <T::Lookup as StaticLookup>::Source = T::Lookup::unlookup(who.clone());
    }: _(RawOrigin::Root, Chain::Bitcoin, who_lookup.clone(), who_lookup)
    verify {
        assert_eq!(Pallet::<T>::referral_binding_of(&who, Chain::Bitcoin), Some(who));
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
            assert_ok!(Pallet::<Test>::test_benchmark_withdraw());
            assert_ok!(Pallet::<Test>::test_benchmark_cancel_withdrawal());
            assert_ok!(Pallet::<Test>::test_benchmark_setup_trustee());
            assert_ok!(Pallet::<Test>::test_benchmark_set_trustee_proxy());
            assert_ok!(Pallet::<Test>::test_benchmark_set_trustee_info_config());
            assert_ok!(Pallet::<Test>::test_benchmark_set_trustee_admin());
            assert_ok!(Pallet::<Test>::test_benchmark_set_trustee_admin_multiply());
            assert_ok!(Pallet::<Test>::test_benchmark_claim_trustee_reward());
            assert_ok!(Pallet::<Test>::test_benchmark_force_trustee_election());
            assert_ok!(Pallet::<Test>::test_benchmark_force_update_trustee());
            assert_ok!(Pallet::<Test>::test_benchmark_force_set_referral_binding());
        });
    }
}
