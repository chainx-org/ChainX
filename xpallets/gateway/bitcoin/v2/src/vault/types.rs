use codec::{Decode, Encode};
use sp_std::{default::Default, prelude::Vec};

pub type BtcAddress = Vec<u8>;

#[derive(Encode, Decode, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub enum VaultStatus {
    /// Vault is ready to serve issue and redeem request, unless it was banned.
    Active,
    /// Vault is under Liquidation
    Liquidated,
    /// Vault was committed has illegal behavior.
    CommittedTheft,
}

impl Default for VaultStatus {
    fn default() -> Self {
        VaultStatus::Active
    }
}

#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct SystemVault<AccountId, Balance> {
    pub(crate) id: AccountId,
    pub(crate) to_be_issued_tokens: Balance,
    pub(crate) issued_tokens: Balance,
    pub(crate) to_be_redeemed_tokens: Balance,
}

#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Vault<AccountId, BlockNumber, Balance> {
    /// Account identifier of the Vault
    pub id: AccountId,
    /// Number of tokens pending issue
    pub to_be_issued_tokens: Balance,
    /// Number of issued tokens
    pub issued_tokens: Balance,
    /// Number of tokens pending redeem
    pub to_be_redeemed_tokens: Balance,
    /// Bitcoin address of this Vault (P2PKH, P2SH, P2PKH, P2WSH)
    pub wallet: BtcAddress,
    /// Block height until which this Vault is banned from being
    /// used for Issue, Redeem (except during automatic liquidation) and Replace .
    pub banned_until: Option<BlockNumber>,
    /// Current status of the vault
    pub status: VaultStatus,
}

impl<AccountId: Default, BlockNumber: Default, Balance: Default>
    Vault<AccountId, BlockNumber, Balance>
{
    pub(crate) fn new(id: AccountId, address: BtcAddress) -> Self {
        Self {
            id,
            wallet: address,
            ..Default::default()
        }
    }
}
