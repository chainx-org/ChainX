// Copyright 2018-2019 Chainpool.

use parity_codec::{Decode, Encode};

#[cfg(feature = "std")]
use serde_derive::{Deserialize, Serialize};

#[cfg(feature = "std")]
use rstd::result;

use inherents::IsFatalError;
#[cfg(feature = "std")]
use inherents::ProvideInherentData;

use super::RuntimeString;
#[cfg(feature = "std")]
use super::{InherentData, InherentIdentifier, INHERENT_IDENTIFIER};

#[derive(PartialEq, Eq, Clone, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub enum NetworkType {
    Mainnet,
    Testnet,
}

impl Default for NetworkType {
    fn default() -> Self {
        NetworkType::Testnet
    }
}

#[derive(Encode)]
#[cfg_attr(feature = "std", derive(Debug, Decode))]
pub enum InherentError {
    /// no producer set
    NoBlockProducer,
    /// Some other error.
    Other(RuntimeString),
}

impl IsFatalError for InherentError {
    fn is_fatal_error(&self) -> bool {
        true
    }
}

#[cfg(feature = "std")]
pub struct InherentDataProvider {
    block_producer_name: Vec<u8>,
}

#[cfg(feature = "std")]
impl InherentDataProvider {
    pub fn new(producer_name: Vec<u8>) -> Self {
        InherentDataProvider {
            block_producer_name: producer_name,
        }
    }
}

#[cfg(feature = "std")]
impl ProvideInherentData for InherentDataProvider {
    fn inherent_identifier(&self) -> &'static InherentIdentifier {
        &INHERENT_IDENTIFIER
    }

    fn provide_inherent_data(
        &self,
        inherent_data: &mut InherentData,
    ) -> result::Result<(), RuntimeString> {
        inherent_data.put_data(INHERENT_IDENTIFIER, &self.block_producer_name)
    }

    fn error_to_string(&self, _error: &[u8]) -> Option<String> {
        // do not handle due no check for this inherent
        None
    }
}
