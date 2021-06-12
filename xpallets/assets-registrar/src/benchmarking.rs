// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use frame_benchmarking::{benchmarks, impl_benchmark_test_suite};
use frame_support::StorageMap;
use frame_system::RawOrigin;

use chainx_primitives::AssetId;

use crate::{AssetInfo, AssetInfoOf, AssetOnline, Call, Chain, Config, Pallet};

const ASSET_ID: AssetId = 8888;

fn b_asset_info_test_data<T: Config>() -> AssetInfo {
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
    register {
        let asset_info = b_asset_info_test_data::<T>();
    }: _(RawOrigin::Root, ASSET_ID, asset_info.clone(), true, true)
    verify {
        assert_eq!(AssetInfoOf::get(ASSET_ID), Some(asset_info));
    }

    deregister {
        let asset_info = b_asset_info_test_data::<T>();
        Pallet::<T>::register(RawOrigin::Root.into(), ASSET_ID, asset_info.clone(), true, true)?;
    }: _(RawOrigin::Root, ASSET_ID)
    verify {
        assert!(!AssetOnline::get(ASSET_ID));
    }

    recover {
        let asset_info = b_asset_info_test_data::<T>();
        Pallet::<T>::register(RawOrigin::Root.into(), ASSET_ID, asset_info.clone(), true, true)?;
        Pallet::<T>::deregister(RawOrigin::Root.into(), ASSET_ID)?;
    }: _(RawOrigin::Root, ASSET_ID, true)
    verify {
        assert!(AssetOnline::get(ASSET_ID));
    }

    update_asset_info {
        let asset_info = b_asset_info_test_data::<T>();
        Pallet::<T>::register(RawOrigin::Root.into(), ASSET_ID, asset_info.clone(), true, true)?;
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

impl_benchmark_test_suite!(
    Pallet,
    crate::tests::ExtBuilder::default().build_with(),
    crate::tests::Test,
);
