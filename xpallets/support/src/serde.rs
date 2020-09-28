// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use serde::{de, ser, Deserialize};

/// Hex serialization/deserialization
pub mod serde_hex {
    use super::*;

    /// A serializer that first encodes the argument as a hex-string
    pub fn serialize<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
        T: AsRef<[u8]>,
    {
        let output = hex::encode(value);
        serializer.serialize_str(&format!("0x{:}", output))
    }

    /// A deserializer that first encodes the argument as a hex-string
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let data = String::deserialize(deserializer)?;
        let data_ref = if data.starts_with("0x") {
            &data[2..]
        } else {
            &data[..]
        };
        let hex = hex::decode(data_ref).map_err(de::Error::custom)?;
        Ok(hex)
    }
}

/// Text serialization/deserialization
pub mod serde_text {
    use super::*;

    /// A serializer that first encodes the argument as a string
    pub fn serialize<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
        T: AsRef<[u8]>,
    {
        let output = String::from_utf8_lossy(value.as_ref());
        serializer.serialize_str(&output)
    }

    /// A deserializer that first encodes the argument as a string
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(s.into_bytes())
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
        let ser2 = "\"30313233343536373839\""; // without 0x
        let de = serde_json::from_str::<HexTest>(&ser2).unwrap();
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
