use super::*;

use frame_benchmarking::{account, benchmarks, whitelisted_caller};
use frame_system::RawOrigin;

use chainx_primitives::AssetId;

use crate::Module as XGatewayCommon;
use xpallet_gateway_records::Module as XGatewayRecords;

const ASSET_ID: AssetId = 1;
const SEED: u32 = 0;

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
    }
}
