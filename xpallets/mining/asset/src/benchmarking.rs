use super::*;

pub use frame_benchmarking::{account, benchmarks};
use frame_system::RawOrigin;
use xpallet_protocol::X_BTC;

const SEED: u32 = 0;

benchmarks! {
    _{
        // User account seed
        let u in 0 .. 1000 => ();
    }

    claim {
        let miner = account("miner", u, SEED);
        xpallet_assets::Module::<T>::issue(&X_BTC, &miner, 1000.into())?;

        let reward_pot = T::DetermineRewardPotAccount::reward_pot_account_for(&X_BTC);
        <T as xpallet_assets::Trait>::Currency::make_free_balance_be(&reward_pot, 100.into());
        <T as xpallet_assets::Trait>::Currency::issue(100.into());

        Module::<T>::set_claim_staking_requirement(RawOrigin::Root.into(), X_BTC, 0)?;

        let block_number: T::BlockNumber = frame_system::Module::<T>::block_number();
        frame_system::Module::<T>::set_block_number(block_number + 100.into());

    }: _(RawOrigin::Signed(miner.clone()), X_BTC)
    verify {
        // 10% belongs to the referral/treasury, 90% is the miner's reward.
        assert!(Module::<T>::free_balance(&miner) == 90.into());
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
            assert_ok!(crate::tests::t_register_xbtc());
            assert_ok!(test_benchmark_claim::<Test>());
        });
    }
}