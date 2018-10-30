// Copyright 2018 Chainpool

#[cfg(feature = "std")]
use std::{fmt, str};
#[cfg(feature = "std")]
use base58::{ToBase58, FromBase58};
use hex::{ToHex, FromHex};
use hash::{H520, H264, H256, H160};
use bitcrypto::{dhash160, checksum};
use codec::{Encode, Decode};
use rstd::ops;
use script::script::ScriptAddress;

/// 20 bytes long hash derived from public `ripemd160(sha256(public))`
pub type AddressHash = H160;
/// 32 bytes long secret key
pub type Secret = H256;
/// 32 bytes long signable message
pub type Message = H256;

const NETWORK_ID: u32 = 1;

#[cfg_attr(feature = "std", derive(Debug))]
#[derive(PartialEq, Clone, Copy, Encode, Decode)]
pub enum Network {
    Mainnet = 0,
    Testnet = 1,
}

impl Default for Network {
    fn default() -> Network { Network::Mainnet }
}

#[cfg_attr(feature = "std", derive(Debug))]
#[derive(PartialEq, Clone, Copy, Encode, Decode)]
pub enum Type {
    /// Pay to PubKey Hash
    /// Common P2PKH which begin with the number 1, eg: 1BvBMSEYstWetqTFn5Au4m4GFg7xJaNVN2.
    /// https://bitcoin.org/en/glossary/p2pkh-address
    P2PKH,
    /// Pay to Script Hash
    /// Newer P2SH type starting with the number 3, eg: 3J98t1WpEZ73CNmQviecrnyiWrnqRhWNLy.
    /// https://bitcoin.org/en/glossary/p2sh-address
    P2SH,
}

impl Default for Type {
     fn default() -> Type { Type::P2PKH }
}

pub trait DisplayLayout {
    type Target: ops::Deref<Target = [u8]>;

    fn layout(&self) -> Self::Target;

    fn from_layout(data: &[u8]) -> Result<Self, Error> where Self: Sized;
}

#[cfg_attr(feature = "std", derive(Debug))]
#[derive(PartialEq, Clone, Encode, Decode, Default)]
pub struct Address {
    /// The type of the address.
    pub kind: Type,
    /// The network of the address.
    pub network: Network,
    /// Public key hash.
    pub hash: AddressHash,
}

impl From<&ScriptAddress> for Address {
    fn from(address: &ScriptAddress) -> Self {
        let network = if NETWORK_ID == 1 { Network::Testnet } else { Network::Mainnet };
        Address {
            kind: address.kind,
            network: network,
            hash: address.hash.clone(),
        }
    }
}

pub struct AddressDisplayLayout([u8; 25]);

impl ops::Deref for AddressDisplayLayout {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DisplayLayout for Address {
    type Target = AddressDisplayLayout;

    fn layout(&self) -> Self::Target {
        let mut result = [0u8; 25];

        result[0] = match (self.network, self.kind) {
            (Network::Mainnet, Type::P2PKH) => 0,
            (Network::Mainnet, Type::P2SH) => 5,
            (Network::Testnet, Type::P2PKH) => 111,
            (Network::Testnet, Type::P2SH) => 196,
        };

        result[1..21].copy_from_slice(&*self.hash);
        let cs = checksum(&result[0..21]);
        result[21..25].copy_from_slice(&*cs);
        AddressDisplayLayout(result)
    }

    fn from_layout(data: &[u8]) -> Result<Self, Error> where Self: Sized {
        if data.len() != 25 {
            return Err(Error::InvalidAddress);
        }

        let cs = checksum(&data[0..21]);
        if &data[21..] != &*cs {
            return Err(Error::InvalidChecksum);
        }

        let (network, kind) = match data[0] {
            0 => (Network::Mainnet, Type::P2PKH),
            5 => (Network::Mainnet, Type::P2SH),
            111 => (Network::Testnet, Type::P2PKH),
            196 => (Network::Testnet, Type::P2SH),
            _ => return Err(Error::InvalidAddress),
        };

        let mut hash = AddressHash::default();
        hash.copy_from_slice(&data[1..21]);

        let address = Address {
            kind: kind,
            network: network,
            hash: hash,
        };

        Ok(address)
    }
}

#[cfg(feature = "std")]
impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.layout().to_base58().fmt(f)
    }
}

#[cfg(feature = "std")]
impl str::FromStr for Address {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Error> where Self: Sized {
        let hex = try!(s.from_base58().map_err(|_| Error::InvalidAddress));
        Address::from_layout(&hex)
    }
}

impl From<&'static str> for Address {
    fn from(s: &'static str) -> Self {
        s.parse().unwrap()
    }
}

#[cfg_attr(feature = "std", derive(Debug))]
#[derive(PartialEq)]
pub enum Error {
    InvalidPublic,
    InvalidSecret,
    InvalidMessage,
    InvalidSignature,
    InvalidNetwork,
    InvalidChecksum,
    InvalidPrivate,
    InvalidAddress,
}

/// Secret public key
pub enum Public {
    /// Normal version of public key
    Normal(H520),
    /// Compressed version of public key
    Compressed(H264),
}

impl Public {
    pub fn from_slice(data: &[u8]) -> Result<Self, Error> {
        match data.len() {
            33 => {
                let mut public = H264::default();
                public.copy_from_slice(data);
                Ok(Public::Compressed(public))
            },
            65 => {
                let mut public = H520::default();
                public.copy_from_slice(data);
                Ok(Public::Normal(public))
            },
            _ => Err(Error::InvalidPublic)
        }
    }

    pub fn address_hash(&self) -> AddressHash {
        let public_key: &[u8] = self;
        dhash160(public_key)
    }
}

impl ops::Deref for Public {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        match *self {
            Public::Normal(ref hash) => &**hash,
            Public::Compressed(ref hash) => &**hash,
        }
    }
}

impl PartialEq for Public {
    fn eq(&self, other: &Self) -> bool {
        let s_slice: &[u8] = self;
        let o_slice: &[u8] = other;
        s_slice == o_slice
    }
}

#[derive(PartialEq)]
pub struct Signature(Vec<u8>);

#[cfg(feature = "std")]
impl fmt::Debug for Signature {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.to_hex::<String>().fmt(f)
    }
}

#[cfg(feature = "std")]
impl fmt::Display for Signature {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.to_hex::<String>().fmt(f)
    }
}

impl ops::Deref for Signature {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(feature = "std")]
impl str::FromStr for Signature {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Error> {
        let vec = try!(s.from_hex().map_err(|_| Error::InvalidSignature));
        Ok(Signature(vec))
    }
}

impl From<&'static str> for Signature {
    fn from(s: &'static str) -> Self {
        s.parse().unwrap()
    }
}

impl From<Vec<u8>> for Signature {
    fn from(v: Vec<u8>) -> Self {
        Signature(v)
    }
}

impl From<Signature> for Vec<u8> {
    fn from(s: Signature) -> Self {
        s.0
    }
}

impl<'a> From<&'a [u8]> for Signature {
    fn from(v: &'a [u8]) -> Self {
        Signature(v.to_vec())
    }
}

#[derive(PartialEq)]
pub struct CompactSignature(H520);

#[cfg(feature = "std")]
impl fmt::Debug for CompactSignature {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.0.to_hex::<String>())
    }
}

#[cfg(feature = "std")]
impl fmt::Display for CompactSignature {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.0.to_hex::<String>())
    }
}

impl ops::Deref for CompactSignature {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

#[cfg(feature = "std")]
impl str::FromStr for CompactSignature {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Error> {
        match s.parse() {
            Ok(hash) => Ok(CompactSignature(hash)),
            _ => Err(Error::InvalidSignature),
        }
    }
}

impl From<&'static str> for CompactSignature {
    fn from(s: &'static str) -> Self {
        s.parse().unwrap()
    }
}

impl From<H520> for CompactSignature {
    fn from(h: H520) -> Self {
        CompactSignature(h)
    }
}
