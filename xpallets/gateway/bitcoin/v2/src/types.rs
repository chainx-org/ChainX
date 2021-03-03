use bitflags::bitflags;
use codec::{Decode, Encode};
use light_bitcoin::keys::Address;
use sp_runtime::RuntimeDebug;

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

pub type BtcAddress = Address;

/// Bridge status
#[derive(Encode, Decode, RuntimeDebug, Clone, Eq, PartialEq)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum Status {
    /// `Running` means bridge runs normally.
    Running,
    /// `Error` means bridge has errors need to be solved.
    /// Bridge may be in multiple error state.
    Error(ErrorCode),
    /// `Shutdown` means bridge is closed, and all feature are unavailable.
    Shutdown,
}

impl Default for Status {
    fn default() -> Self {
        Self::Running
    }
}

bitflags! {
    /// Bridge error with bitflag
    #[derive(Encode, Decode)]
    #[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
    pub struct ErrorCode : u8 {
        const NONE = 0b00000000;
        /// During liquidation
        /// Bridge ecovers after debt was paid off.
        const LIQUIDATING = 0b00000001;
        /// Oracle doesn't update exchange rate in time.
        /// Bridge recovers after exchange rate updating
        const EXCHANGE_RATE_EXPIRED = 0b00000010;
    }
}

impl Default for ErrorCode {
    fn default() -> Self {
        Self::NONE
    }
}

/// This struct represents the price of trading pair PCX/BTC.
///
/// For example, the current price of PCX/BTC in some
/// exchange is 0.0001779 which will be represented as
/// `ExchangeRate { price: 1779, decimal: 7 }`.
#[derive(Encode, Decode, RuntimeDebug, Clone, Default, Eq, PartialEq)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct TradingPrice {
    /// Price with decimals.
    pub price: u128,
    /// How many decimals of the exchange price.
    pub decimal: u8,
}

impl TradingPrice {
    /// Returns the converted amount of BTC given the `pcx_amount`.
    pub fn convert_to_btc(&self, pcx_amount: u128) -> Option<u128> {
        self.price
            .checked_mul(pcx_amount)
            .and_then(|c| c.checked_div(10_u128.pow(u32::from(self.decimal))))
    }

    /// Returns the converted amount of PCX given the `btc_amount`.
    pub fn convert_to_pcx(&self, btc_amount: u128) -> Option<u128> {
        btc_amount
            .checked_mul(10_u128.pow(u32::from(self.decimal)))
            .and_then(|c| c.checked_div(self.price))
    }
}

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
        Self::Active
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

#[derive(Encode, Decode, RuntimeDebug, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct RedeemRequest<AccountId, BlockNumber, XBTC, PCX> {
    /// Vault id
    pub(crate) vault: AccountId,
    /// Block height when the redeem requested
    pub(crate) open_time: BlockNumber,
    /// Who requests redeem
    pub(crate) requester: AccountId,
    /// Vault's btc address
    pub(crate) btc_address: BtcAddress,
    /// Amount that user wants to redeem
    pub(crate) btc_amount: XBTC,
    /// Redeem fee amount
    pub(crate) redeem_fee: PCX,
    /// If redeem is reimbursed by redeemer
    pub(crate) reimburse: bool,
}

/// Contains all informations while executing a issue request needed.
#[derive(Encode, Decode, RuntimeDebug, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Deserialize, Serialize))]
pub struct IssueRequest<AccountId, BlockNumber, XBTC, PCX> {
    /// Vault id
    pub(crate) vault: AccountId,
    /// Block height when the issue requested
    pub(crate) open_time: BlockNumber,
    /// Who requests issue
    pub(crate) requester: AccountId,
    /// Vault's btc address
    pub(crate) btc_address: BtcAddress,
    /// Amount that user wants to issue
    pub(crate) btc_amount: XBTC,
    /// Collateral locked to avoid user griefing
    pub(crate) griefing_collateral: PCX,
}

#[cfg(test)]
mod tests {
    use super::TradingPrice;
    #[test]
    fn test_btc_conversion() {
        let trading_price = TradingPrice {
            price: 1,
            decimal: 4,
        };
        assert_eq!(trading_price.convert_to_btc(10000), Some(1));
    }

    #[test]
    fn test_pcx_conversion() {
        let trading_price = TradingPrice {
            price: 1,
            decimal: 4,
        };
        assert_eq!(trading_price.convert_to_pcx(1), Some(10000));

        let trading_price = TradingPrice {
            price: 1,
            decimal: 38,
        };
        assert_eq!(trading_price.convert_to_pcx(1_000_000), None);
    }
}
