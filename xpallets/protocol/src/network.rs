// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use codec::{Decode, Encode};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

/// The network type of ChainX.
#[derive(PartialEq, Eq, Clone, Copy, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub enum NetworkType {
    /// Main network type
    Mainnet,
    /// Test network type
    Testnet,
}

impl Default for NetworkType {
    fn default() -> Self {
        NetworkType::Testnet
    }
}

impl NetworkType {
    /// Return the ss58 address format identifier of the network type.
    pub fn ss58_addr_format_id(&self) -> Ss58AddressFormatId {
        match self {
            NetworkType::Mainnet => MAINNET_ADDRESS_FORMAT_ID,
            NetworkType::Testnet => TESTNET_ADDRESS_FORMAT_ID,
        }
    }
}

/// Ss58AddressFormat identifier
pub type Ss58AddressFormatId = u8;
/// ChainX main network ss58 address format identifier
pub const MAINNET_ADDRESS_FORMAT_ID: Ss58AddressFormatId = 44; // 44 is Ss58AddressFormat::ChainXAccount
/// ChainX test network ss58 address format identifier
pub const TESTNET_ADDRESS_FORMAT_ID: Ss58AddressFormatId = 42; // 42 is Ss58AddressFormat::SubstrateAccount
