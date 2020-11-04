// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use std::{
    fmt::{Debug, Display},
    result::Result as StdResult,
    str::FromStr,
};

pub use jsonrpc_core::{Error, ErrorCode, Result};
use serde::{de, ser, Deserialize, Serialize};

/// The call to runtime failed.
pub const RUNTIME_ERROR: i64 = 1;

/// The call related to trustee to runtime failed.
const RUNTIME_TRUSTEE_ERROR: i64 = RUNTIME_ERROR + 100;

/// Decode the generic trustee info failed.
///
/// TODO: these pallet-specific errors should be moved to its own rpc module
/// when there are many of them.
pub const RUNTIME_TRUSTEE_DECODE_ERROR: i64 = RUNTIME_TRUSTEE_ERROR + 1;

/// The trustees are inexistent.
pub const RUNTIME_TRUSTEE_INEXISTENT_ERROR: i64 = RUNTIME_TRUSTEE_ERROR + 2;

/// The transaction was not decodable.
pub const DECODE_ERROR: i64 = 10000;

/// The bytes failed to be decoded as hex.
pub const DECODE_HEX_ERROR: i64 = DECODE_ERROR + 1;

/// Converts a runtime trap into an RPC error.
pub fn runtime_error_into_rpc_err(err: impl Debug) -> Error {
    Error {
        code: ErrorCode::ServerError(RUNTIME_ERROR),
        message: "Runtime trapped".into(),
        data: Some(format!("{:?}", err).into()),
    }
}

/// Converts a trustee runtime trap into an RPC error.
pub fn trustee_decode_error_into_rpc_err(err: impl Debug) -> Error {
    Error {
        code: ErrorCode::ServerError(RUNTIME_TRUSTEE_DECODE_ERROR),
        message: "Can not decode generic trustee session info".into(),
        data: Some(format!("{:?}", err).into()),
    }
}

/// Converts a trustee runtime trap into an RPC error.
pub fn trustee_inexistent_rpc_err() -> Error {
    Error {
        code: ErrorCode::ServerError(RUNTIME_TRUSTEE_INEXISTENT_ERROR),
        message: "Trustee does not exist".into(),
        data: None,
    }
}

/// Converts a hex decode error into an RPC error.
pub fn hex_decode_error_into_rpc_err(err: impl Debug) -> Error {
    Error {
        code: ErrorCode::ServerError(DECODE_HEX_ERROR),
        message: "Failed to decode hex".into(),
        data: Some(format!("{:?}", err).into()),
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

/// Number string serialization/deserialization
pub mod serde_num_str {
    use super::*;

    /// A serializer that encodes the number as a string
    pub fn serialize<S, T>(value: &T, serializer: S) -> StdResult<S::Ok, S::Error>
    where
        S: ser::Serializer,
        T: Display,
    {
        serializer.serialize_str(&value.to_string())
    }

    /// A deserializer that decodes a string to the number.
    pub fn deserialize<'de, D, T>(deserializer: D) -> StdResult<T, D::Error>
    where
        D: de::Deserializer<'de>,
        T: FromStr,
    {
        let data = String::deserialize(deserializer)?;
        data.parse::<T>()
            .map_err(|_| de::Error::custom("Parse from string failed"))
    }
}

/// Hex serialization/deserialization
pub mod serde_hex {
    use super::*;

    /// A serializer that encodes the bytes as a hex-string
    pub fn serialize<T, S>(value: &T, serializer: S) -> StdResult<S::Ok, S::Error>
    where
        S: ser::Serializer,
        T: AsRef<[u8]>,
    {
        serializer.serialize_str(&format!("0x{}", hex::encode(value)))
    }

    /// A deserializer that decodes the hex-string to bytes (Vec<u8>)
    pub fn deserialize<'de, D>(deserializer: D) -> StdResult<Vec<u8>, D::Error>
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
    pub fn serialize<T, S>(value: &T, serializer: S) -> StdResult<S::Ok, S::Error>
    where
        S: ser::Serializer,
        T: AsRef<[u8]>,
    {
        let output = String::from_utf8_lossy(value.as_ref());
        serializer.serialize_str(&output)
    }

    /// A deserializer that decodes the string to the bytes (Vec<u8>)
    pub fn deserialize<'de, D>(deserializer: D) -> StdResult<Vec<u8>, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let data = String::deserialize(deserializer)?;
        Ok(data.into_bytes())
    }
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};

    #[test]
    fn test_serde_num_str_attr() {
        use super::RpcU128;

        let test = RpcU128(u128::max_value());
        let ser = serde_json::to_string(&test).unwrap();
        assert_eq!(ser, "\"340282366920938463463374607431768211455\"");
        let de = serde_json::from_str::<RpcU128<u128>>(&ser).unwrap();
        assert_eq!(de, test);
    }

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
}
