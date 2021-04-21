// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use codec::{Decode, Encode};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

// Substate
use sp_runtime::RuntimeDebug;

#[derive(RuntimeDebug, Decode, Encode, PartialEq, PartialOrd, Ord, Eq, Copy, Clone)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum BridgeSubPot {
    Vault,
    User,
}

