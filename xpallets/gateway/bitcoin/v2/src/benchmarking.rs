use crate::{
    pallet::{Call, Config, Error, Event, ExchangeRate, Module, Pallet},
    types::TradingPrice,
};
pub use frame_benchmarking::{account, benchmarks, benchmarks_instance_pallet};
use frame_system::RawOrigin;

benchmarks! {
    update_exchange_rate {
        let oracle: T::AccountId = account("oracle", 0, 0);
        crate::pallet::Pallet::<T>::force_update_oracles(
                RawOrigin::Root.into(),
                vec![oracle.clone()]
            ).unwrap();
    }: _(RawOrigin::Signed(oracle), TradingPrice {
        price: 1,
        decimal: 3,
    })
    verify {
        assert_eq!(ExchangeRate::<T>::get(), TradingPrice {
            price: 1,
            decimal: 3,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::mock::{BuildConfig, ExtBuilder, Test};

    use super::*;

    #[test]
    fn test_benchmarks() {
        ExtBuilder::build(BuildConfig::default()).execute_with(|| {
            test_benchmark_update_exchange_rate::<Test>().unwrap();
        })
    }
}
