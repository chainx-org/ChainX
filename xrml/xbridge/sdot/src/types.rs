use parity_codec::{Decode, Encode};

pub type EthereumAddress = [u8; 20];

#[derive(Encode, Decode, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct EcdsaSignature(pub [u8; 32], pub [u8; 32], pub i8);
