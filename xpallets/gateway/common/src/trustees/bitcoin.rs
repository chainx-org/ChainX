use super::*;
use btc_keys::{Address, Public as BTCPublic};

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct BTCTrusteeAddrInfo {
    pub addr: Address,
    pub redeem_script: Vec<u8>,
}

impl TryFrom<Vec<u8>> for BTCTrusteeAddrInfo {
    type Error = CodecError;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        Decode::decode(&mut &value[..])
    }
}

impl Into<Vec<u8>> for BTCTrusteeAddrInfo {
    fn into(self) -> Vec<u8> {
        self.encode()
    }
}

#[derive(PartialEq, Eq, Clone, Encode, Decode)]
pub struct BTCTrusteeType(pub BTCPublic);
impl Into<Vec<u8>> for BTCTrusteeType {
    fn into(self) -> Vec<u8> {
        self.0.to_vec()
    }
}

#[cfg(feature = "std")]
mod serde_impl {
    use super::*;
    use serde::{de::Error, Deserializer, Serializer};

    // use serde::{Deserialize, Serialize};
    impl Serialize for BTCTrusteeType {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            serializer.serialize_bytes(&self.0)
        }
    }
    impl<'de> Deserialize<'de> for BTCTrusteeType {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            let data: Vec<u8> = Deserialize::deserialize(deserializer)?;
            let pubkey = BTCPublic::from_slice(&data)
                .map_err(|e| Error::custom(format!("not valid pubkey hex:{:?}", e)))?;
            Ok(BTCTrusteeType(pubkey))
        }
    }
}

impl TryFrom<Vec<u8>> for BTCTrusteeType {
    type Error = ();

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        BTCPublic::from_slice(&value)
            .map(BTCTrusteeType)
            .map_err(|_| ())
    }
}

impl ChainProvider for BTCTrusteeType {
    fn chain() -> Chain {
        Chain::Bitcoin
    }
}

impl ChainProvider for BTCTrusteeAddrInfo {
    fn chain() -> Chain {
        Chain::Bitcoin
    }
}

pub type BTCTrusteeIntentionProps = TrusteeIntentionProps<BTCTrusteeType>;
pub type BTCTrusteeSessionInfo<AccountId> = TrusteeSessionInfo<AccountId, BTCTrusteeAddrInfo>;
pub type BTCTrusteeSessionManager<T> = TrusteeSessionManager<T, BTCTrusteeAddrInfo>;
pub type BTCTrusteeMultisig<T> = TrusteeMultisigProvider<T, BTCTrusteeType>;
