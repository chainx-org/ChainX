// Copyright 2018-2019 Chainpool.

use parity_codec::{Decode, Encode};

#[derive(PartialEq, PartialOrd, Ord, Eq, Clone, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Debug))]
pub enum CallSwitcher {
    Global,
    Spot,
    XBTC,
    XBTCLockup,
    SDOT,
    XContracts,
    XMiningStaking,
    XMiningTokens,
}
