use super::*;

pub use frame_benchmarking::{account, benchmarks};
use frame_system::RawOrigin;
use xpallet_protocol::X_BTC;

const SEED: u32 = 0;
const PAIR_ID: u32 = 0;

benchmarks! {
    _{
        // User account seed
        let u in 0 .. 1000 => ();
    }

    // TODO: put_order with matching.
    put_order {
        let user: T::AccountId = account("user", u, SEED);

        <T as xpallet_assets::Trait>::Currency::make_free_balance_be(&user, 1000.into());
        <T as xpallet_assets::Trait>::Currency::issue(1000.into());

        <xpallet_assets::Module<T>>::issue(&X_BTC, &user, 100.into())?;

    }: put_order(RawOrigin::Signed(user.clone()), PAIR_ID, OrderType::Limit, Side::Buy, 1000.into(), 1_000_200.into())
    verify {
        assert!(OrderInfoOf::<T>::get(user, 0).is_some());
    }

    cancel_order {
        let user: T::AccountId = account("user", u, SEED);

        <T as xpallet_assets::Trait>::Currency::make_free_balance_be(&user, 1000.into());
        <T as xpallet_assets::Trait>::Currency::issue(1000.into());

        <xpallet_assets::Module<T>>::issue(&X_BTC, &user, 100.into())?;

        Module::<T>::put_order(RawOrigin::Signed(user.clone()).into(), PAIR_ID, OrderType::Limit, Side::Buy, 1000.into(), 1_000_200.into())?;
    }: _(RawOrigin::Signed(user.clone()), PAIR_ID, 0)
    verify {
        assert!(OrderInfoOf::<T>::get(user, 0).is_none());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::{ExtBuilder, Test, XSpot};
    use crate::tests::{t_generic_issue, t_issue_pcx, t_set_handicap};
    use frame_support::assert_ok;

    #[test]
    fn test_benchmarks() {
        ExtBuilder::default().build().execute_with(|| {
            let pair_id = 0;
            let who = 1;
            let trading_pair = XSpot::trading_pair_of(pair_id).unwrap();

            t_set_handicap(pair_id, 1_000_000, 1_100_000);

            // Reserve asset.
            t_generic_issue(trading_pair.quote(), who, 10);

            // Reserve native coin, 100 native coins should be reserved.
            t_issue_pcx(who, 1000);

            assert_ok!(test_benchmark_put_order::<Test>());
            assert_ok!(test_benchmark_cancel_order::<Test>());
        });
    }
}
