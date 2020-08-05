use super::*;
use btc_keys::Public as BtcPublic;

#[derive(PartialEq, Eq, Clone, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct BtcTrusteeAddrInfo {
    #[cfg_attr(feature = "std", serde(with = "xpallet_support::serde_impl::text"))]
    pub addr: BtcAddress,
    #[cfg_attr(feature = "std", serde(with = "xpallet_support::serde_impl::hex"))]
    pub redeem_script: Vec<u8>,
}

#[cfg(feature = "std")]
impl std::fmt::Debug for BtcTrusteeAddrInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "BtcTrusteeAddrInfo {{ addr: {:?}, redeem_script: {} }}",
            self.addr,
            hex::encode(&self.redeem_script)
        )
    }
}

#[cfg(not(feature = "std"))]
impl core::fmt::Debug for BtcTrusteeAddrInfo {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "BtcTrusteeAddrInfo {{ addr: {:?}, redeem_script: {:?} }}",
            self.addr, self.redeem_script
        )
    }
}

impl TryFrom<Vec<u8>> for BtcTrusteeAddrInfo {
    type Error = CodecError;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        Decode::decode(&mut &value[..])
    }
}

impl Into<Vec<u8>> for BtcTrusteeAddrInfo {
    fn into(self) -> Vec<u8> {
        self.encode()
    }
}

#[derive(PartialEq, Eq, Clone, Encode, Decode)]
pub struct BtcTrusteeType(pub BtcPublic);
impl Into<Vec<u8>> for BtcTrusteeType {
    fn into(self) -> Vec<u8> {
        self.0.to_vec()
    }
}

#[cfg(feature = "std")]
mod serde_impl {
    use super::*;
    use serde::{de::Error, Deserializer, Serializer};
    use xpallet_support::serde_impl::hex;

    impl Serialize for BtcTrusteeType {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            let d = (&*self.0).to_vec();
            hex::serialize(&d, serializer)
        }
    }
    impl<'de> Deserialize<'de> for BtcTrusteeType {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            let data: Vec<u8> = hex::deserialize(deserializer)?;
            let pubkey = BtcPublic::from_slice(&data)
                .map_err(|e| Error::custom(format!("not valid pubkey hex:{:?}", e)))?;
            Ok(BtcTrusteeType(pubkey))
        }
    }

    // pub mod btc_addr {
    //     use super::*;
    //     use sp_std::str::FromStr;
    //
    //     pub fn serialize<S>(value: &Address, serializer: S) -> Result<S::Ok, S::Error>
    //     where
    //         S: Serializer,
    //     {
    //         let output = value.to_string();
    //         serializer.serialize_str(&output)
    //     }
    //
    //     pub fn deserialize<'de, D>(deserializer: D) -> Result<Address, D::Error>
    //     where
    //         D: Deserializer<'de>,
    //     {
    //         let s: String = Deserialize::deserialize(deserializer)?;
    //         Address::from_str(&s).map_err(|e| Error::custom(format!("{:?}", e)))
    //     }
    // }
}

impl TryFrom<Vec<u8>> for BtcTrusteeType {
    type Error = ();

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        BtcPublic::from_slice(&value)
            .map(BtcTrusteeType)
            .map_err(|_| ())
    }
}

impl ChainProvider for BtcTrusteeType {
    fn chain() -> Chain {
        Chain::Bitcoin
    }
}

impl ChainProvider for BtcTrusteeAddrInfo {
    fn chain() -> Chain {
        Chain::Bitcoin
    }
}

pub type BtcTrusteeIntentionProps = TrusteeIntentionProps<BtcTrusteeType>;
pub type BtcTrusteeSessionInfo<AccountId> = TrusteeSessionInfo<AccountId, BtcTrusteeAddrInfo>;
pub type BtcTrusteeSessionManager<T> = TrusteeSessionManager<T, BtcTrusteeAddrInfo>;
pub type BtcTrusteeMultisig<T> = TrusteeMultisigProvider<T, BtcTrusteeType>;
pub type BtcAddress = Vec<u8>;
