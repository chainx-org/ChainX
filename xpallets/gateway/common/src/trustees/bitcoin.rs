// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use codec::{Decode, Encode, Error as CodecError};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

use sp_runtime::RuntimeDebug;
use sp_std::{convert::TryFrom, fmt, prelude::Vec};

use xpallet_assets::Chain;

use super::{TrusteeMultisigProvider, TrusteeSessionManager};
use crate::traits::ChainProvider;
use crate::types::{TrusteeIntentionProps, TrusteeSessionInfo};

pub type BtcAddress = Vec<u8>;
pub type BtcTrusteeSessionInfo<AccountId> = TrusteeSessionInfo<AccountId, BtcTrusteeAddrInfo>;
pub type BtcTrusteeIntentionProps = TrusteeIntentionProps<BtcTrusteeType>;
pub type BtcTrusteeSessionManager<T> = TrusteeSessionManager<T, BtcTrusteeAddrInfo>;
pub type BtcTrusteeMultisig<T> = TrusteeMultisigProvider<T, BtcTrusteeType>;

#[derive(PartialEq, Eq, Clone, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct BtcTrusteeAddrInfo {
    #[cfg_attr(feature = "std", serde(with = "xp_rpc::serde_text"))]
    pub addr: BtcAddress,
    #[cfg_attr(feature = "std", serde(with = "xp_rpc::serde_hex"))]
    pub redeem_script: Vec<u8>,
}

impl fmt::Debug for BtcTrusteeAddrInfo {
    #[cfg(feature = "std")]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let redeem_script_in_hex = hex::encode(&self.redeem_script);
        if redeem_script_in_hex.len() > 16 {
            write!(
                f,
                "BtcTrusteeAddrInfo {{ addr: {}, redeem_script: 0x{}...{} }}",
                String::from_utf8_lossy(&self.addr),
                &redeem_script_in_hex[..8],
                &redeem_script_in_hex[redeem_script_in_hex.len() - 8..]
            )
        } else {
            write!(
                f,
                "BtcTrusteeAddrInfo {{ addr: {}, redeem_script: 0x{} }}",
                String::from_utf8_lossy(&self.addr),
                redeem_script_in_hex,
            )
        }
    }

    #[cfg(not(feature = "std"))]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "BtcTrusteeAddrInfo {{ addr: {:?}, redeem_script: {:?} }}",
            self.addr, self.redeem_script
        )
    }
}

impl From<BtcTrusteeAddrInfo> for Vec<u8> {
    fn from(value: BtcTrusteeAddrInfo) -> Self {
        value.encode()
    }
}

impl TryFrom<Vec<u8>> for BtcTrusteeAddrInfo {
    type Error = CodecError;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        Decode::decode(&mut &value[..])
    }
}

impl ChainProvider for BtcTrusteeAddrInfo {
    fn chain() -> Chain {
        Chain::Bitcoin
    }
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct BtcTrusteeType(pub light_bitcoin::keys::Public);

impl From<BtcTrusteeType> for Vec<u8> {
    fn from(value: BtcTrusteeType) -> Self {
        value.0.to_vec()
    }
}

impl TryFrom<Vec<u8>> for BtcTrusteeType {
    type Error = ();

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        light_bitcoin::keys::Public::from_slice(&value)
            .map(BtcTrusteeType)
            .map_err(|_| ())
    }
}

impl ChainProvider for BtcTrusteeType {
    fn chain() -> Chain {
        Chain::Bitcoin
    }
}

#[test]
fn test_serde_btc_trustee_type() {
    let pubkey = BtcTrusteeType(light_bitcoin::keys::Public::Compressed(Default::default()));
    let ser = serde_json::to_string(&pubkey).unwrap();
    assert_eq!(
        ser,
        "\"0x000000000000000000000000000000000000000000000000000000000000000000\""
    );
    let de = serde_json::from_str::<BtcTrusteeType>(&ser).unwrap();
    assert_eq!(de, pubkey);
}
