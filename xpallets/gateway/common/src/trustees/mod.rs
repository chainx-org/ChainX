use codec::{Decode, Encode, Error as CodecError};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
// Substrate
use sp_std::{convert::TryFrom, prelude::Vec};

use crate::types::{TrusteeIntentionProps, TrusteeSessionInfo};

pub mod bitcoin {
    use super::*;
    use btc_keys::{Address, Public as BTCPublic};

    #[derive(PartialEq, Eq, Clone, Encode, Decode)]
    #[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
    #[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
    pub struct BTCTrusteeAddrInfo {
        pub addr: Address,
        pub redeem_script: Vec<u8>,
    }

    impl TryFrom<Vec<u8>> for BTCTrusteeAddrInfo {
        type Error = CodecError;

        fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
            Decode::decode(&mut &value[..])
        }
    }

    impl Into<Vec<u8>> for BTCTrusteeAddrInfo {
        fn into(self) -> Vec<u8> {
            self.encode()
        }
    }
    pub type BTCTrusteeType = BTCPublic;

    pub type BTCTrusteeIntentionProps = TrusteeIntentionProps<BTCTrusteeType>;
    pub type BTCTrusteeSessionInfo<AccountId> = TrusteeSessionInfo<AccountId, BTCTrusteeAddrInfo>;
}
