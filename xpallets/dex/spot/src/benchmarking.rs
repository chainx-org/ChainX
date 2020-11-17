// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

pub use frame_benchmarking::{account, benchmarks};
use frame_system::RawOrigin;
use xp_protocol::X_BTC;

use super::*;

const EOS: AssetId = 8888;
const ETH: AssetId = 9999;

const SEED: u32 = 0;
const PAIR_ID: u32 = 0;

fn b_prepare_put_order<T: Trait>(
    user: &T::AccountId,
    pcx_value: u32,
    btc_value: u32,
) -> DispatchResult {
    <T as xpallet_assets::Trait>::Currency::make_free_balance_be(user, pcx_value.into());
    <T as xpallet_assets::Trait>::Currency::issue(pcx_value.into());

    <xpallet_assets::Module<T>>::issue(&X_BTC, user, btc_value.into())?;
    Ok(())
}

fn b_put_order<T: Trait>(
    user: T::AccountId,
    pcx_value: u32,
    btc_value: u32,
    price: u32,
) -> DispatchResult {
    b_prepare_put_order::<T>(&user, pcx_value, btc_value)?;
    Module::<T>::put_order(
        RawOrigin::Signed(user.clone()).into(),
        PAIR_ID,
        OrderType::Limit,
        Side::Buy,
        pcx_value.into(),
        price.into(),
    )?;
    Ok(())
}

benchmarks! {
    _{
        // User account seed
        let u in 0 .. 1000 => ();
    }

    // TODO: put_order with matching.
    put_order {
        let user: T::AccountId = account("user", u, SEED);

        b_prepare_put_order::<T>(&user, 1000, 100)?;

    }: put_order(RawOrigin::Signed(user.clone()), PAIR_ID, OrderType::Limit, Side::Buy, 1000.into(), 1_000_200.into())
    verify {
        assert!(OrderInfoOf::<T>::get(user, 0).is_some());
    }

    cancel_order {
        let user: T::AccountId = account("user", u, SEED);

        b_put_order::<T>(user.clone(), 1000, 100, 1_000_200)?;

    }: _(RawOrigin::Signed(user.clone()), PAIR_ID, 0)
    verify {
        assert!(OrderInfoOf::<T>::get(user, 0).is_none());
    }

    force_cancel_order {
        let user: T::AccountId = account("user", u, SEED);

        let user_lookup: <T::Lookup as StaticLookup>::Source = T::Lookup::unlookup(user.clone());

        b_put_order::<T>(user.clone(), 1000, 100, 1_000_200)?;

    }: _(RawOrigin::Root, user_lookup, PAIR_ID, 0)
    verify {
        assert!(OrderInfoOf::<T>::get(user, 0).is_none());
    }

    set_handicap {
    }: _(RawOrigin::Root, PAIR_ID, Handicap::new(100.into(), 110.into()))
    verify {
        assert_eq!(HandicapOf::<T>::get(PAIR_ID), Handicap { highest_bid: 100.into(), lowest_ask: 110.into() });
    }

    set_price_fluctuation {
    }: _(RawOrigin::Root, PAIR_ID, 1000)
    verify {
        assert_eq!(PriceFluctuationOf::get(PAIR_ID), 1000);
    }

    add_trading_pair {
        let pair = CurrencyPair::new(EOS, ETH);
    }: _(RawOrigin::Root, pair.clone(), 2, 1, 100.into(), true)
    verify {
        #[cfg(test)]
        assert_eq!(Module::<T>::trading_pair_count(), 3);
        #[cfg(feature = "runtime-benchmarks")]
        assert_eq!(Module::<T>::trading_pair_count(), 2);
        assert_eq!(
            Module::<T>::get_trading_pair_by_currency_pair(&pair)
                .unwrap()
                .base(),
            pair.base
        );
    }

    update_trading_pair {
        let pair = CurrencyPair::new(EOS, ETH);
        Module::<T>::add_trading_pair(RawOrigin::Root.into(), pair.clone(), 2, 1, 100.into(), true)?;
    }: _(RawOrigin::Root, PAIR_ID, 888, false)
    verify {
        assert_eq!(Module::<T>::trading_pair_of(PAIR_ID).unwrap().tick_decimals, 888);
        assert_eq!(Module::<T>::trading_pair_of(PAIR_ID).unwrap().tradable, false);
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
            assert_ok!(test_benchmark_force_cancel_order::<Test>());
            assert_ok!(test_benchmark_set_handicap::<Test>());
            assert_ok!(test_benchmark_set_price_fluctuation::<Test>());
            assert_ok!(test_benchmark_add_trading_pair::<Test>());
            assert_ok!(test_benchmark_update_trading_pair::<Test>());
        });
    }
}
