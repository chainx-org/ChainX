// Copyright 2018-2019 Chainpool.

#[cfg(feature = "std")]
use parity_codec::Decode;
use parity_codec::Encode;

use inherents::IsFatalError;
#[cfg(feature = "std")]
use inherents::ProvideInherentData;

use super::RuntimeString;
#[cfg(feature = "std")]
use super::{InherentData, InherentIdentifier, StdResult, INHERENT_IDENTIFIER};

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
        match self {
            _ => true,
        }
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
    ) -> StdResult<(), RuntimeString> {
        inherent_data.put_data(INHERENT_IDENTIFIER, &self.block_producer_name)
    }

    fn error_to_string(&self, _error: &[u8]) -> Option<String> {
        // do not handle due no check for this inherent
        None
    }
}
