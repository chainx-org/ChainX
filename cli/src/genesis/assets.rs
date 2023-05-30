// Copyright 2019-2023 ChainX Project Authors. Licensed under GPL-3.0.
#![allow(clippy::type_complexity)]

use xp_protocol::{BTC_DECIMALS, PCX, PCX_DECIMALS, X_BTC};

use chainx_runtime::{AssetId, AssetInfo, AssetRestrictions, Chain, Runtime};

pub(crate) type AssetParams = (AssetId, AssetInfo, AssetRestrictions, bool, bool);

pub(crate) fn init_assets(
    assets: Vec<AssetParams>,
) -> (
    Vec<(AssetId, AssetInfo, bool, bool)>,
    Vec<(AssetId, AssetRestrictions)>,
) {
    let mut init_assets = vec![];
    let mut assets_restrictions = vec![];
    for (a, b, c, d, e) in assets {
        init_assets.push((a, b, d, e));
        assets_restrictions.push((a, c))
    }
    (init_assets, assets_restrictions)
}

pub(crate) fn pcx() -> (AssetId, AssetInfo, AssetRestrictions) {
    (
        PCX,
        AssetInfo::new::<Runtime>(
            b"PCX".to_vec(),
            b"Polkadot ChainX".to_vec(),
            Chain::ChainX,
            PCX_DECIMALS,
            b"ChainX's crypto currency in Polkadot ecology".to_vec(),
        )
        .unwrap(),
        AssetRestrictions::DEPOSIT
            | AssetRestrictions::WITHDRAW
            | AssetRestrictions::DESTROY_WITHDRAWAL
            | AssetRestrictions::DESTROY_USABLE,
    )
}

pub(crate) fn xbtc() -> (AssetId, AssetInfo, AssetRestrictions) {
    (
        X_BTC,
        AssetInfo::new::<Runtime>(
            b"XBTC".to_vec(),
            b"ChainX Bitcoin".to_vec(),
            Chain::Bitcoin,
            BTC_DECIMALS,
            b"ChainX's Cross-chain Bitcoin".to_vec(),
        )
        .unwrap(),
        AssetRestrictions::DESTROY_USABLE,
    )
}

// asset_id, asset_info, asset_restrictions, is_online, has_mining_rights
pub(crate) fn genesis_assets() -> Vec<(AssetId, AssetInfo, AssetRestrictions, bool, bool)> {
    let pcx = pcx();
    let btc = xbtc();
    let assets = vec![
        (pcx.0, pcx.1, pcx.2, true, false),
        (btc.0, btc.1, btc.2, true, true),
    ];
    assets
}
