// Copyright 2018 Chainpool.

//! Typesafe block interaction.

use super::{
    AccountId, Block, Call, BLOCK_PRODUCER_POSITION, NOTE_OFFLINE_POSITION, TIMESTAMP_SET_POSITION,
};
use timestamp::Call as TimestampCall;
//use session::Call as SessionCall;
use cxsystem::Call as CXSystemCall;

/// Provides a type-safe wrapper around a structurally valid block.
pub struct CheckedBlock {
    inner: Block,
    file_line: Option<(&'static str, u32)>,
}

impl CheckedBlock {
    /// Create a new checked block. Fails if the block is not structurally valid.
    pub fn new(block: Block) -> Result<Self, Block> {
        let has_timestamp = block
            .extrinsics
            .get(TIMESTAMP_SET_POSITION as usize)
            .map_or(false, |xt| {
                !xt.is_signed()
                    && match xt.function {
                        Call::Timestamp(TimestampCall::set(_)) => true,
                        _ => false,
                    }
            });

        if !has_timestamp {
            return Err(block);
        }

        Ok(CheckedBlock {
            inner: block,
            file_line: None,
        })
    }

    // Creates a new checked block, asserting that it is valid.
    #[doc(hidden)]
    pub fn new_unchecked(block: Block, file: &'static str, line: u32) -> Self {
        CheckedBlock {
            inner: block,
            file_line: Some((file, line)),
        }
    }

    /// Extract the timestamp from the block.
    pub fn timestamp(&self) -> ::chainx_primitives::Timestamp {
        let x = self
            .inner
            .extrinsics
            .get(TIMESTAMP_SET_POSITION as usize)
            .and_then(|xt| match xt.function {
                Call::Timestamp(TimestampCall::set(x)) => Some(x),
                _ => None,
            });

        match x {
            Some(x) => x,
            None => panic!("Invalid chainx block asserted at {:?}", self.file_line),
        }
    }

    /// Extract the noted offline validator indices (if any) from the block.
    pub fn noted_offline(&self) -> &[u32] {
        self.inner
            .extrinsics
            .get(NOTE_OFFLINE_POSITION as usize)
            .and_then(|xt| match xt.function {
                //Call::Session(SessionCall::note_offline(ref x)) => Some(&x[..]),
                _ => None,
            })
            .unwrap_or(&[])
    }

    pub fn block_producer(&self) -> Option<AccountId> {
        self.inner
            .extrinsics
            .get(BLOCK_PRODUCER_POSITION as usize)
            .and_then(|xt| match xt.function {
                Call::CXSystem(CXSystemCall::set_block_producer(x)) => Some(x),
                _ => None,
            })
    }

    /// Convert into inner block.
    pub fn into_inner(self) -> Block {
        self.inner
    }
}

impl ::std::ops::Deref for CheckedBlock {
    type Target = Block;

    fn deref(&self) -> &Block {
        &self.inner
    }
}

/// Assert that a block is structurally valid. May lead to panic in the future
/// in case it isn't.
#[macro_export]
macro_rules! assert_chainx_block {
    ($block: expr) => {
        $crate::CheckedBlock::new_unchecked($block, file!(), line!())
    };
}
