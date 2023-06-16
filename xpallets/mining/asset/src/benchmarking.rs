// Copyright 2019-2023 ChainX Project Authors. Licensed under GPL-3.0.

pub use frame_benchmarking::{account, benchmarks};
use frame_system::RawOrigin;
use xp_protocol::{X_BTC, X_DOT};

use super::*;

const SEED: u32 = 0;

benchmarks! {
    claim {
        xpallet_assets_registrar::Pallet::<T>::register(
            frame_system::RawOrigin::Root.into(),
            X_DOT,
            xpallet_assets_registrar::AssetInfo::new::<T>(
                b"X-DOT".to_vec(),
                b"Polkadot".to_vec(),
                xpallet_assets_registrar::Chain::Polkadot,
                10,
                b"Polkadot".to_vec(),
            ).unwrap(),
            true,
            true,
        ).unwrap();

        FixedAssetPowerOf::<T>::insert(X_DOT, 100);

        let miner = account("miner", 0, SEED);
        xpallet_assets::Pallet::<T>::issue(&X_DOT, &miner, 1000u32.into(), true)?;

        let reward_pot = T::DetermineRewardPotAccount::reward_pot_account_for(&X_DOT);
        <T as xpallet_assets::Config>::Currency::make_free_balance_be(&reward_pot, 100u32.into());
        <T as xpallet_assets::Config>::Currency::issue(100u32.into());

        Pallet::<T>::set_claim_staking_requirement(RawOrigin::Root.into(), X_DOT, 0)?;

        let block_number: T::BlockNumber = frame_system::Pallet::<T>::block_number();
        frame_system::Pallet::<T>::set_block_number(block_number + 100u32.into());

    }: _(RawOrigin::Signed(miner.clone()), X_DOT)
    verify {
        // 10% belongs to the referral/treasury, 90% is the miner's reward.
        assert!(Pallet::<T>::free_balance(&miner) == 90u32.into());
    }

    set_claim_staking_requirement {
        let c = 1000;
    }: _(RawOrigin::Root, X_BTC, c)
    verify {
        assert_eq!(ClaimRestrictionOf::<T>::get(X_BTC).staking_requirement, c);
    }

    set_claim_frequency_limit {
        let c = 1000u32;
    }: _(RawOrigin::Root, X_BTC, c.into())
    verify {
        assert_eq!(ClaimRestrictionOf::<T>::get(X_BTC).frequency_limit, c.into());
    }

    set_asset_power {
        let c = 1000;
    }: _(RawOrigin::Root, X_BTC, c)
    verify {
        assert_eq!(FixedAssetPowerOf::<T>::get(X_BTC), c);
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
            assert_ok!(Pallet::<Test>::test_benchmark_claim());
            assert_ok!(Pallet::<Test>::test_benchmark_set_claim_staking_requirement());
            assert_ok!(Pallet::<Test>::test_benchmark_set_claim_frequency_limit());
            assert_ok!(Pallet::<Test>::test_benchmark_set_asset_power());
        });
    }
}
