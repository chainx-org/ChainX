// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use std::{fmt::Display, str::FromStr};

use serde::{de, ser, Deserialize, Serialize};

/// Hex serialization/deserialization
pub mod serde_hex {
    use super::*;

    /// A serializer that encodes the bytes as a hex-string
    pub fn serialize<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
        T: AsRef<[u8]>,
    {
        serializer.serialize_str(&format!("0x{}", hex::encode(value)))
    }

    /// A deserializer that decodes the hex-string to bytes (Vec<u8>)
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let data = String::deserialize(deserializer)?;
        let data = if data.starts_with("0x") {
            &data[2..]
        } else {
            &data[..]
        };
        let hex = hex::decode(data).map_err(de::Error::custom)?;
        Ok(hex)
    }
}

/// Text serialization/deserialization
pub mod serde_text {
    use super::*;

    /// A serializer that encodes the bytes as a string
    pub fn serialize<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
        T: AsRef<[u8]>,
    {
        let output = String::from_utf8_lossy(value.as_ref());
        serializer.serialize_str(&output)
    }

    /// A deserializer that decodes the string to the bytes (Vec<u8>)
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let data = String::deserialize(deserializer)?;
        Ok(data.into_bytes())
    }
}

/// Number string serialization/deserialization
pub mod serde_num_str {
    use super::*;

    /// A serializer that encodes the number as a string
    pub fn serialize<S, T>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
        T: Display,
    {
        serializer.serialize_str(&value.to_string())
    }

    /// A deserializer that decodes a string to the number.
    pub fn deserialize<'de, D, T>(deserializer: D) -> Result<T, D::Error>
    where
        D: de::Deserializer<'de>,
        T: FromStr,
    {
        let data = String::deserialize(deserializer)?;
        data.parse::<T>()
            .map_err(|_| de::Error::custom("Parse from string failed"))
    }
}

/// Balance type when interacting with RPC.
pub type RpcBalance<Balance> = RpcU128<Balance>;

/// Price type of order when interacting with RPC.
pub type RpcPrice<Price> = RpcU128<Price>;

/// Weight type of mining when interacting with RPC.
pub type RpcMiningWeight<Weight> = RpcU128<Weight>;

/// Weight type of staking when interacting with RPC.
pub type RpcVoteWeight<Weight> = RpcU128<Weight>;

/// A helper struct for handling u128 serialization/deserialization of RPC.
/// See https://github.com/polkadot-js/api/issues/2464 for details (shit!).
#[derive(Eq, PartialEq, Copy, Clone, Debug, Serialize, Deserialize)]
pub struct RpcU128<T: Display + FromStr>(#[serde(with = "self::serde_num_str")] T);

impl<T: Display + FromStr> From<T> for RpcU128<T> {
    fn from(value: T) -> Self {
        RpcU128(value)
    }
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};

    #[test]
    fn test_serde_hex_attr() {
        #[derive(PartialEq, Debug, Serialize, Deserialize)]
        struct HexTest(#[serde(with = "super::serde_hex")] Vec<u8>);

        let test = HexTest(b"0123456789".to_vec());
        let ser = serde_json::to_string(&test).unwrap();
        assert_eq!(ser, "\"0x30313233343536373839\"");
        let de = serde_json::from_str::<HexTest>(&ser).unwrap();
        assert_eq!(de, test);
        // without 0x
        let de = serde_json::from_str::<HexTest>("\"30313233343536373839\"").unwrap();
        assert_eq!(de, test);
    }

    #[test]
    fn test_serde_text_attr() {
        #[derive(PartialEq, Debug, Serialize, Deserialize)]
        struct TextTest(#[serde(with = "super::serde_text")] Vec<u8>);

        let test = TextTest(b"0123456789".to_vec());
        let ser = serde_json::to_string(&test).unwrap();
        assert_eq!(ser, "\"0123456789\"");
        let de = serde_json::from_str::<TextTest>(&ser).unwrap();
        assert_eq!(de, test);
    }

    #[test]
    fn test_serde_num_str_attr() {
        use super::RpcU128;

        let test = RpcU128(u128::max_value());
        let ser = serde_json::to_string(&test).unwrap();
        assert_eq!(ser, "\"340282366920938463463374607431768211455\"");
        let de = serde_json::from_str::<RpcU128<u128>>(&ser).unwrap();
        assert_eq!(de, test);
    }
}
