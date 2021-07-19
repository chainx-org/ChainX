use codec::{Decode, Encode};
use sp_std::{vec, vec::Vec};

use frame_benchmarking::benchmarks_instance_pallet;
use frame_support::traits::{fungible::Mutate, Currency};
use frame_system::RawOrigin;
use sp_runtime::AccountId32;

use crate::pallet::*;
use crate::types::TradingPrice;

type VaultInfo<T: Config> = (T::AccountId, Vec<u8>);

fn account<T: Config<I>, I: 'static>(pubkey: &str) -> T::AccountId {
    let pubkey = hex::decode(pubkey).unwrap();
    let mut public = [0u8; 32];
    public.copy_from_slice(pubkey.as_slice());
    let account = AccountId32::from(public).encode();
    Decode::decode(&mut account.as_slice()).unwrap()
}

fn vault_alice<T: Config<I>, I: 'static>() -> VaultInfo<T> {
    // sr25519 Alice
    (
        account::<T, I>("d43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d"),
        "2MyLnphRhkZij648SVeQhZoR3ry1ZoHzasW".into(),
    )
}

fn register_alice<T: Config<I>, I: 'static>() -> VaultInfo<T> {
    let (caller, addr) = vault_alice::<T, I>();
    Pallet::<T, I>::inner_register_vault(&caller, addr.clone(), 50000u32.into()).unwrap();
    (caller, addr)
}

benchmarks_instance_pallet! {
    update_exchange_rate {
        let (caller, _) = vault_alice::<T, I>();
    }: _(RawOrigin::Signed(caller), TradingPrice {
            price: 1,
            decimal: 3,
    })
    verify{}

    register_vault {
        let (caller, addr) = vault_alice::<T, I>();
    }: _(RawOrigin::Signed(caller), 5000u32.into(), addr)
    verify {}

    add_extra_collateral {
        let (caller, addr) = register_alice::<T, I>();
    }: _(RawOrigin::Signed(caller), 5000u32.into())
    verify {}

    request_issue {
        let (caller, addr) = register_alice::<T, I>();
    }: _(RawOrigin::Signed(caller), caller.clone(), 10u32.into())
    verify {}

    execute_issue {
        let (caller, addr) = register_alice::<T, I>();
        let request_id = Pallet::<T, I>::insert_new_issue_request(caller.clone(), &caller, 1000u32.into(), 100u32.into()).unwrap();
    }: _(RawOrigin::Signed(caller), request_id, vec![], vec![], vec![])
    verify {}

    cancel_issue {
        let (caller, addr) = register_alice::<T, I>();
        let request_id = Pallet::<T, I>::insert_new_issue_request(caller.clone(), &caller, 1000u32.into(), 100u32.into()).unwrap();
        frame_system::Pallet::<T>::set_block_number(80000u32.into());
    }: _(RawOrigin::Signed(caller), request_id)
    verify {}

    request_redeem {
        let (caller, addr) = register_alice::<T, I>();
        Pallet::<T, I>::mint(&caller, &caller, 100000u32.into()).unwrap();
    }: _(RawOrigin::Signed(caller), caller.clone(), 20000u32.into(), addr)
    verify {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::*;
    use frame_support::assert_ok;
    #[test]
    fn test_benchmarks() {
        ExtBuilder::build(Default::default()).execute_with(|| {
            assert_ok!(test_benchmark_update_exchange_rate::<Test>());
            assert_ok!(test_benchmark_register_vault::<Test>());
            assert_ok!(test_benchmark_request_issue::<Test>());
            assert_ok!(test_benchmark_execute_issue::<Test>());
            assert_ok!(test_benchmark_cancel_issue::<Test>());
            assert_ok!(test_benchmark_request_redeem::<Test>());
        })
    }
}
