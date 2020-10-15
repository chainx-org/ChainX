// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use frame_benchmarking::benchmarks;
use frame_system::RawOrigin;

use super::*;

const ASSET_ID: AssetId = 8888;

fn b_asset_info_test_data<T: Trait>() -> AssetInfo {
    AssetInfo::new::<T>(
        b"token".to_vec(),
        b"token_name".to_vec(),
        Chain::Bitcoin,
        8,
        b"token_desc".to_vec(),
    )
    .unwrap()
}

benchmarks! {
    _{
        // User account seed
        let u in 0 .. 1000 => ();
    }

    register {
        let asset_info = b_asset_info_test_data::<T>();
    }: _(RawOrigin::Root, ASSET_ID, asset_info.clone(), true, true)
    verify {
        assert_eq!(AssetInfoOf::get(ASSET_ID), Some(asset_info));
    }

    deregister {
        let asset_info = b_asset_info_test_data::<T>();
        Module::<T>::register(RawOrigin::Root.into(), ASSET_ID, asset_info.clone(), true, true)?;
    }: _(RawOrigin::Root, ASSET_ID)
    verify {
        assert!(!AssetOnline::get(ASSET_ID));
    }

    recover {
        let asset_info = b_asset_info_test_data::<T>();
        Module::<T>::register(RawOrigin::Root.into(), ASSET_ID, asset_info.clone(), true, true)?;
        Module::<T>::deregister(RawOrigin::Root.into(), ASSET_ID)?;
    }: _(RawOrigin::Root, ASSET_ID, true)
    verify {
        assert!(AssetOnline::get(ASSET_ID));
    }

    update_asset_info {
        let asset_info = b_asset_info_test_data::<T>();
        Module::<T>::register(RawOrigin::Root.into(), ASSET_ID, asset_info.clone(), true, true)?;
    }: _(
        RawOrigin::Root,
        ASSET_ID,
        Some(b"new_token".to_vec()),
        Some(b"new_token_name".to_vec()),
        Some(b"new_desc".to_vec())
    )
    verify {
        let mut new_asset_info = asset_info.clone();
        new_asset_info.set_token(b"new_token".to_vec());
        new_asset_info.set_token_name(b"new_token_name".to_vec());
        new_asset_info.set_desc(b"new_desc".to_vec());
        assert_eq!(AssetInfoOf::get(ASSET_ID).unwrap(), new_asset_info);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{ExtBuilder, Test};
    use frame_support::assert_ok;

    #[test]
    fn test_benchmarks() {
        ExtBuilder::default().build(vec![]).execute_with(|| {
            assert_ok!(test_benchmark_register::<Test>());
            assert_ok!(test_benchmark_deregister::<Test>());
            assert_ok!(test_benchmark_recover::<Test>());
            assert_ok!(test_benchmark_update_asset_info::<Test>());
        });
    }
}
