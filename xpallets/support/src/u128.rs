pub type RpcBalance<Balance> = U128<Balance>;

#[derive(Eq, PartialEq, codec::Encode, codec::Decode, Default)]
#[cfg_attr(feature = "std", derive(std::fmt::Debug))]
pub struct U128<Balance>(Balance);

#[cfg(feature = "std")]
impl<Balance: std::fmt::Display> std::fmt::Display for U128<Balance> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<Balance> From<Balance> for U128<Balance> {
    fn from(inner: Balance) -> Self {
        Self(inner)
    }
}

#[cfg(feature = "std")]
impl<Balance: std::str::FromStr> std::str::FromStr for U128<Balance> {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let inner = s.parse::<Balance>().map_err(|_| "Parse Balance failed")?;
        Ok(Self(inner))
    }
}

#[cfg(feature = "std")]
impl<Balance: std::fmt::Display> serde::ser::Serialize for U128<Balance> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        self.0.to_string().serialize(serializer)
    }
}

#[cfg(feature = "std")]
impl<'de, Balance: std::str::FromStr> serde::de::Deserialize<'de> for U128<Balance> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let inner = s
            .parse::<Balance>()
            .map_err(|_| serde::de::Error::custom("Parse Balance from String failed"))?;
        Ok(Self(inner))
    }
}
