use frame_support::{
    dispatch::{DispatchResult, DispatchResultWithPostInfo},
    traits::Hooks,
};
use frame_system::RawOrigin;

use crate::mock::*;

pub(super) fn t_register_vault(
    id: u64,
    collateral: u128,
    addr: &str,
) -> DispatchResultWithPostInfo {
    XGatewayBitcoin::register_vault(Origin::signed(id), collateral, addr.as_bytes().to_vec())
}

pub(super) fn run_to_block(index: u64) {
    while System::block_number() < index {
        XGatewayBitcoin::on_finalize(System::block_number());
        System::on_finalize(System::block_number());

        System::set_block_number(System::block_number() + 1);

        System::on_initialize(System::block_number());
        XGatewayBitcoin::on_initialize(System::block_number());
    }
}

pub(super) fn t_register_btc() -> DispatchResult {
    type XAssetsRegistrar = xpallet_assets_registrar::Module<Test>;
    type XAssets = xpallet_assets::Module<Test>;
    let assets = vec![
        (
            xp_protocol::X_BTC,
            xpallet_assets_registrar::AssetInfo::new::<Test>(
                b"X-BTC".to_vec(),
                b"X-BTC".to_vec(),
                xpallet_assets_registrar::Chain::Bitcoin,
                xp_protocol::BTC_DECIMALS,
                b"ChainX's cross-chain Bitcoin".to_vec(),
            )
            .unwrap(),
            xpallet_assets::AssetRestrictions::empty(),
        ),
        (
            xp_protocol::C_BTC,
            xpallet_assets_registrar::AssetInfo::new::<Test>(
                b"C-BTC".to_vec(),
                b"C-BTC".to_vec(),
                xpallet_assets_registrar::Chain::Bitcoin,
                xp_protocol::BTC_DECIMALS,
                b"Bridge ChainX's cross-chain Bitcoin".to_vec(),
            )
            .unwrap(),
            xpallet_assets::AssetRestrictions::empty(),
        )
    ];

    for (id, info, restrictions) in assets.into_iter() {
        XAssetsRegistrar::register(RawOrigin::Root.into(), id, info, true, true)?;
        XAssets::set_asset_limit(RawOrigin::Root.into(), id, restrictions)?;
    }
    Ok(())
}
