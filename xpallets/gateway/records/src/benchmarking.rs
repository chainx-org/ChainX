// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use super::*;
use crate::Module as XGatewayRecords;

use frame_benchmarking::{benchmarks, whitelisted_caller};
use frame_system::RawOrigin;

const ASSET_ID: AssetId = xpallet_protocol::X_BTC;

fn deposit<T: Trait>(who: T::AccountId, amount: BalanceOf<T>) {
    let receiver_lookup: <T::Lookup as StaticLookup>::Source = T::Lookup::unlookup(who);
    // root_deposit
    let _ = XGatewayRecords::<T>::root_deposit(
        RawOrigin::Root.into(),
        receiver_lookup,
        ASSET_ID,
        amount,
    );
}

fn deposit_and_withdraw<T: Trait>(who: T::AccountId, amount: BalanceOf<T>) {
    deposit::<T>(who.clone(), amount);
    let withdrawal = amount - 500.into();
    let addr = b"3LFSUKkP26hun42J1Dy6RATsbgmBJb27NF".to_vec();
    let memo = b"memo".to_vec().into();
    let receiver_lookup: <T::Lookup as StaticLookup>::Source = T::Lookup::unlookup(who);
    XGatewayRecords::<T>::root_withdraw(
        RawOrigin::Root.into(),
        receiver_lookup,
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
}

benchmarks! {
    _{ }

    root_deposit {
        let receiver: T::AccountId = whitelisted_caller();
        let receiver_lookup: <T::Lookup as StaticLookup>::Source = T::Lookup::unlookup(receiver.clone());
        let amount: BalanceOf<T> = 1000.into();
    }: _(RawOrigin::Root, receiver_lookup, ASSET_ID, amount)
    verify {
        assert_eq!(xpallet_assets::Module::<T>::usable_balance(&receiver, &ASSET_ID), amount);
    }

    root_withdraw {
        let receiver: T::AccountId = whitelisted_caller();
        let receiver_lookup: <T::Lookup as StaticLookup>::Source = T::Lookup::unlookup(receiver.clone());
        let amount: BalanceOf<T> = 1000.into();
        deposit::<T>(receiver, amount);
        let withdrawal = amount - 500.into();
        let addr = b"3LFSUKkP26hun42J1Dy6RATsbgmBJb27NF".to_vec();
        let memo = b"memo".to_vec().into();
    }: _(RawOrigin::Root, receiver_lookup, ASSET_ID, withdrawal, addr, memo)
    verify {
        assert!(XGatewayRecords::<T>::pending_withdrawals(0).is_some());
        assert_eq!(XGatewayRecords::<T>::state_of(0), Some(WithdrawalState::Applying));
    }

    set_withdrawal_state {
        let receiver: T::AccountId = whitelisted_caller();
        let receiver_lookup: <T::Lookup as StaticLookup>::Source = T::Lookup::unlookup(receiver.clone());
        let amount: BalanceOf<T> = 1000.into();
        deposit_and_withdraw::<T>(receiver, amount);
        let state = WithdrawalState::RootFinish;
    }: _(RawOrigin::Root, 0, state)
    verify {
        assert_eq!(XGatewayRecords::<T>::state_of(0), None);
    }

    set_withdrawal_state_list {
        let u in 1 .. 64 => ();

        let receiver: T::AccountId = whitelisted_caller();
        let receiver_lookup: <T::Lookup as StaticLookup>::Source = T::Lookup::unlookup(receiver.clone());
        let amount: BalanceOf<T> = 1000.into();
        deposit_and_withdraw::<T>(receiver, amount);
        let state = WithdrawalState::RootFinish;
    }: _(RawOrigin::Root, vec![(0, state)])
    verify {
        assert_eq!(XGatewayRecords::<T>::state_of(0), None);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{ExtBuilder, Test};
    use frame_support::assert_ok;

    #[test]
    fn test_benchmarks() {
        ExtBuilder::default().build().execute_with(|| {
            assert_ok!(test_benchmark_root_deposit::<Test>());
        });

        ExtBuilder::default().build().execute_with(|| {
            assert_ok!(test_benchmark_root_withdraw::<Test>());
        });

        ExtBuilder::default().build().execute_with(|| {
            assert_ok!(test_benchmark_set_withdrawal_state::<Test>());
        });
    }
}
