use sp_std::{default::Default, vec::Vec};

use codec::{Decode, Encode};

#[cfg(feature = "std")]
use frame_support::{Deserialize, Serialize};

pub(crate) type BtcAddress = Vec<u8>;

/// Contains all informations while executing a issue request needed.
#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug, Deserialize, Serialize))]
pub struct IssueRequest<AccountId, BlockNumber, XBTC, PCX> {
    /// Vault id
    pub(crate) vault: AccountId,
    /// Block height when the issue requested
    pub(crate) open_time: BlockNumber,
    /// Who requests issue
    pub(crate) requester: AccountId,
    /// Vault's btc address
    pub(crate) btc_address: BtcAddress,
    /// Wheather request finished
    pub(crate) completed: bool,
    /// Wheather request cancelled
    pub(crate) cancelled: bool,
    /// Amount that user wants to issue
    pub(crate) btc_amount: XBTC,
    /// Collateral locked to avoid user griefing
    pub(crate) griefing_collateral: PCX,
}
