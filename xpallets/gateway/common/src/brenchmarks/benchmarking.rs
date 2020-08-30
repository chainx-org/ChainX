use super::*;

use frame_benchmarking::{benchmarks, whitelisted_caller};
use frame_system::RawOrigin;

use chainx_primitives::AssetId;

use crate::Module as XGatewayCommon;
use xpallet_gateway_records::Module as XGatewayRecords;

const ASSET_ID: AssetId = xpallet_protocol::X_BTC;

benchmarks! {
    _{ }

    withdraw {
        let caller: T::AccountId = whitelisted_caller();

        let amount: BalanceOf<T> = 10_00000000.into();
        XGatewayRecords::<T>::deposit(&caller, &ASSET_ID, amount).unwrap();

        let addr = b"3PgYgJA6h5xPEc3HbnZrUZWkpRxuCZVyEP".to_vec();
        let memo = b"".to_vec().into();

    }: _(RawOrigin::Signed(caller.clone()), ASSET_ID, amount, addr, memo)
    verify {
        assert!(XGatewayRecords::<T>::pending_withdrawals(0).is_some());
    }

    setup_trustee {
        let caller: T::AccountId = whitelisted_caller();

        let hot = hex::decode("02df92e88c4380778c9c48268460a124a8f4e7da883f80477deaa644ced486efc6").unwrap();
        let cold = hex::decode("0386b58f51da9b37e59c40262153173bdb59d7e4e45b73994b99eec4d964ee7e88").unwrap();
    }: _(RawOrigin::Signed(caller.clone()), Chain::Bitcoin, b"about".to_vec(), hot, cold)
    verify {
        assert!(XGatewayCommon::<T>::trustee_intention_props_of(caller, Chain::Bitcoin).is_some());
    }

    transition_trustee_session {
        let caller: T::AccountId = whitelisted_caller();
        TrusteeMultiSigAddr::<T>::insert(Chain::Bitcoin, caller.clone());
        let candidators = prepare_intention::<T>();

        assert!(XGatewayCommon::<T>::trustee_session_info_of(Chain::Bitcoin, 0).is_some());

    }: _(RawOrigin::Signed(caller.clone()), Chain::Bitcoin, candidators)
    verify {
        assert!(XGatewayCommon::<T>::trustee_session_info_of(Chain::Bitcoin, 1).is_some());
    }

    set_withdrawal_state {
        let caller: T::AccountId = whitelisted_caller();
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
        let who: T::AccountId = whitelisted_caller();
        let who_lookup: <T::Lookup as StaticLookup>::Source = T::Lookup::unlookup(who.clone());
    }: _(RawOrigin::Root, Chain::Bitcoin, who_lookup.clone(), who_lookup.clone())
    verify {
        assert_eq!(XGatewayCommon::<T>::channel_binding_of(&who, Chain::Bitcoin), Some(who));
    }
}
fn trustees() -> Vec<(AccountId, Vec<u8>, Vec<u8>, Vec<u8>)> {
    let btc_trustees = vec![
        (
            ALICE.clone(),
            b"".to_vec(),
            hex::decode("02df92e88c4380778c9c48268460a124a8f4e7da883f80477deaa644ced486efc6")
                .expect("hex decode failed")
                .into(),
            hex::decode("0386b58f51da9b37e59c40262153173bdb59d7e4e45b73994b99eec4d964ee7e88")
                .expect("hex decode failed")
                .into(),
        ),
        (
            BOB.clone(),
            b"".to_vec(),
            hex::decode("0244d81efeb4171b1a8a433b87dd202117f94e44c909c49e42e77b69b5a6ce7d0d")
                .expect("hex decode failed")
                .into(),
            hex::decode("02e4631e46255571122d6e11cda75d5d601d5eb2585e65e4e87fe9f68c7838a278")
                .expect("hex decode failed")
                .into(),
        ),
        (
            CHARLIE.clone(),
            b"".to_vec(),
            hex::decode("03a36339f413da869df12b1ab0def91749413a0dee87f0bfa85ba7196e6cdad102")
                .expect("hex decode failed")
                .into(),
            hex::decode("0263d46c760d3e04883d4b433c9ce2bc32130acd9faad0192a2b375dbba9f865c3")
                .expect("hex decode failed")
                .into(),
        ),
    ];
    btc_trustees
}
fn prepare_intention<T: Trait>() -> Vec<T::AccountId> {
    use codec::{Decode, Encode};
    let mut v = vec![];
    for (account, about, hot, cold) in trustees() {
        let a = account.encode();
        let acc = T::AccountId::decode(&mut &a[..]).unwrap();
        XGatewayCommon::<T>::setup_trustee(
            RawOrigin::Signed(acc.clone()).into(),
            Chain::Bitcoin,
            about,
            hot,
            cold,
        )
        .unwrap();
        v.push(acc);
    }
    v
}
fn deposit<T: Trait>(who: T::AccountId, amount: BalanceOf<T>) {
    let _ = XGatewayRecords::<T>::deposit(&who, &ASSET_ID, amount);
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
