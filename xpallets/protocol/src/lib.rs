#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

use chainx_primitives::AssetId;

pub use assets_registration::*;
pub use network::*;

/// assets
pub const ASSET_SYMBOL_LEN: usize = 24;
///
pub const ASSET_NAME_LEN: usize = 48;
///
pub const ASSET_DESC_LEN: usize = 128;
///
pub const MEMO_BYTES_LEN: usize = 80;

mod assets_registration {
    use super::*;

    // match to SLIP-0044 Registered coin types for BIP-0044
    // [Registered coin types](https://github.com/satoshilabs/slips/blob/master/slip-0044.md)
    //
    // Particular, ChainX Native token PCX occupies Testnet index, which is not same in SLIP44 standard.
    // so that, ChainX AssetId protocol is:
    //
    // 1. base token:
    //      base token stands for the real token for this Asset on ChainX, all have "X_" prefix, means
    //      cross chain (e.g. BTC is X_BTC, ETH is X_ETH), and ths base token AssetId is from SLIP44
    //      standard "coin type".
    //      But inside, we agree on using Testnet index 1 to stand for **mainnet Bitcoin asset**,
    //      not testnet. And on the other hand, we use 0 to stand for ChainX native token "PCX",
    // and others is all match to SLIP44 "coin type" index.
    //
    // 2. some token chich not in SLIP44 coin types:
    //      e.g. USDT not int SLIP44, thus we use `0x01000000 | id` to extend AssetId for containing
    //      there assets. The AssetId in this part is decided by ChainX.
    //      For example, we agree on pointing 0x01 as the USDT, thus USDT AssetId is `0x01000000|0x01`
    //
    // 3. derived token on ChainX for the cross chain token.
    //      ChainX would derived some special token which just on ChainX and it is not real cross
    //      assets but also have some relationship to source chain assets. Thus we use some
    //      particular prefix to distinguish with base token.
    //      (e.g. L_BTC means locked bitcoin, S_DOT means shadow DOT)
    //      to distinguish with base token AssetId, we use `<Some Prefix>|<base token AssetId>` to
    //      express the derived token. Different derived situation have different prefix.
    //      thus we agree on the prefix:
    //      L_: use 0x90000000
    //      S_: use 0xa0000000

    /// Native coin type of ChainX.
    pub const PCX: AssetId = 0;
    /// BTC asset in ChainX backed by the Mainnet Bitcoin.
    ///
    /// Notice index 1 stands for mainnet Bitcoin, not testnet Bitcoin asset.
    pub const X_BTC: AssetId = 1;
    /// Reserved since this symbol had been used in legacy ChainX 1.0.
    pub const L_BTC: AssetId = 0x90000000 | X_BTC;
    ///
    pub const X_ETH: AssetId = 60;
    ///
    pub const X_DOT: AssetId = 354;
    /// Reserved since this symbol had been used in legacy ChainX 1.0.
    pub const S_DOT: AssetId = 0xa0000000 | X_DOT;

    const EXTEND: AssetId = 0x01000000;
    ///
    pub const USDT: AssetId = EXTEND | 0x01;
}

mod network {
    use super::*;
    pub type AddrVersion = u8;

    #[derive(PartialEq, Eq, Clone, Copy, Encode, Decode)]
    #[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
    pub enum NetworkType {
        Mainnet,
        Testnet,
    }

    impl Default for NetworkType {
        fn default() -> Self {
            NetworkType::Testnet
        }
    }

    impl NetworkType {
        pub fn addr_version(&self) -> AddrVersion {
            match self {
                NetworkType::Mainnet => MAINNET_ADDR_VER,
                NetworkType::Testnet => TESTNET_ADDR_VER,
            }
        }
    }

    pub const MAINNET_ADDR_VER: AddrVersion = 44;
    pub const TESTNET_ADDR_VER: AddrVersion = 45;
}
