// Copyright 2018-2019 Chainpool.

use parity_codec::{Decode, Encode};

//#[derive(PartialEq, Eq, Clone, Encode, Decode, Default)]
//#[cfg_attr(feature = "std", derive(Debug))]
//pub struct SwitchStore {
//    pub global: bool,
//    pub spot: bool,
//    pub xbtc: bool,
//    pub sdot: bool,
//}

#[derive(PartialEq, PartialOrd, Ord, Eq, Clone, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Debug))]
pub enum CallSwitcher {
    Global,
    Spot,
    XBTC,
    XBTCLockup,
    SDOT,
}
