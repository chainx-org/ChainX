// Copyright 2018-2019 Chainpool.

use super::*;

mod proposal09;

impl<T: Trait> Module<T> {
    fn one_pcx() -> u64 {
        let pcx = <xassets::Module<T> as ChainT>::TOKEN.to_vec();
        let pcx_asset = <xassets::Module<T>>::get_asset(&pcx).expect("PCX definitely exist.");

        10_u64.pow(pcx_asset.precision().into())
    }

    pub fn asset_power(token: &Token) -> Option<T::Balance> {
        Self::asset_power_09(token)
    }
}
