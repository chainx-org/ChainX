use rstd::prelude::Vec;

use btc_keys::Public as BitcoinPublic;

use crate::traits::IntoVecu8;

impl IntoVecu8 for BitcoinPublic {
    fn into_vecu8(self) -> Vec<u8> {
        (&self).to_vec()
    }

    fn from_vecu8(src: &[u8]) -> Option<Self> {
        Self::from_slice(src).ok()
    }
}
