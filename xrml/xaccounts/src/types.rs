// Copyright 2018-2019 Chainpool.

use parity_codec::{Decode, Encode};
#[cfg(feature = "std")]
use serde_derive::{Deserialize, Serialize};

// ChainX
use xr_primitives::{XString, URL};

/// Intention properties
#[derive(PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct IntentionProps<SessionKey, BlockNumber> {
    pub url: URL,
    pub is_active: bool,
    pub about: XString,
    pub session_key: Option<SessionKey>,
    pub registered_at: BlockNumber,
    pub last_inactive_since: BlockNumber,
}
