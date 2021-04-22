// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use chainx_primitives::{AssetId, Decimals};

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
//      and others is all match to SLIP44 "coin type" index.
//
// 2. some token which not in SLIP44 coin types:
//      e.g. USDT not int SLIP44, thus we use `0x01000000 | id` to extend AssetId for containing
//      there assets. The AssetId in this part is decided by ChainX.
//      For example, we agree on pointing 0x01 as the USDT, thus USDT AssetId is `0x01000000|0x01`
//
// 3. derived token on ChainX for the cross chain token.
//      ChainX would derived some special token which just on ChainX and it is not real cross
//      assets but also have some relationship to source chain assets. Thus we use some
//      particular prefix to distinguish with base token.
//      (e.g. L_BTC means locked bitcoin, S_DOT means shadow DOT, E_BTC means experimental BTC)
//      to distinguish with base token AssetId, we use `<Some Prefix>|<base token AssetId>` to
//      express the derived token. Different derived situation have different prefix.
//      thus we agree on the prefix:
//      L_: use 0x90000000
//      S_: use 0xa0000000
//      E_: use 0xc0000000

/// Native asset of ChainX.
pub const PCX: AssetId = 0;
/// Decimals of PCX, the native token of ChainX.
pub const PCX_DECIMALS: Decimals = 8;

/// BTC asset in ChainX backed by the Mainnet Bitcoin.
pub const X_BTC: AssetId = 1;
/// Decimals of BTC.
pub const BTC_DECIMALS: Decimals = 8;
/// Reserved since this symbol had been used in legacy ChainX 1.0.
pub const L_BTC: AssetId = 0x90000000 | X_BTC;
/// Experimental BTC for early access version feature, to avoid it mess up the legacy feature
pub const E_BTC: AssetId = 0xc0000000 | X_BTC;
/// Shadow token for E_BTC
pub const S_BTC: AssetId = 0xa0000000 | X_BTC;

/// ETH asset in ChainX backed by the Mainnet Ethereum.
pub const X_ETH: AssetId = 60;

/// DOT asset in ChainX backed by the Mainnet Polkadot.
pub const X_DOT: AssetId = 354;
/// Reserved since this symbol had been used in legacy ChainX 1.0.
pub const S_DOT: AssetId = 0xa0000000 | X_DOT;

const EXTEND: AssetId = 0x01000000;
/// USDT asset in ChainX.
pub const USDT: AssetId = EXTEND | 0x01;
