use codec::{Decode, Encode};
use sp_runtime::RuntimeDebug;

use bitflags::bitflags;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

pub use chainx_primitives::AddrStr;

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

#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Vault<BlockNumber, Balance> {
    /// Number of tokens issued
    pub issue_tokens: Balance,
    /// Number of tokens pending issue
    pub to_be_issued_tokens: Balance,
    /// Number of tokens pending redeem
    pub to_be_redeemed_tokens: Balance,
    /// Bitcoin address of this Vault (P2PKH, P2SH, P2PKH, P2WSH)
    pub wallet: AddrStr,
    /// Block height until which this Vault is banned from being
    /// used for Issue, Redeem (except during automatic liquidation) and Replace .
    pub banned_until: Option<BlockNumber>,
}

impl<BlockNumber: Default, Balance: Default> Vault<BlockNumber, Balance> {
    pub(crate) fn new(address: AddrStr) -> Self {
        Self {
            wallet: address,
            ..Default::default()
        }
    }
}

#[derive(Encode, Decode, RuntimeDebug, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct RedeemRequest<AccountId, BlockNumber, Balance> {
    /// Vault id
    pub vault: AccountId,
    /// Block height when the redeem requested
    pub open_time: BlockNumber,
    /// Who requests redeem
    pub requester: AccountId,
    /// Requester's outer chain address
    pub outer_address: AddrStr,
    /// Amount that user wants to redeem
    pub amount: Balance,
    /// Redeem fee amount
    pub redeem_fee: Balance,
    /// If redeem is reimbursed by redeemer
    pub reimburse: bool,
}

/// Contains all informations while executing a issue request needed.
#[derive(Encode, Decode, RuntimeDebug, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Deserialize, Serialize))]
pub struct IssueRequest<AccountId, BlockNumber, Balance> {
    /// Vault id
    pub vault: AccountId,
    /// Block height when the issue requested
    pub open_time: BlockNumber,
    /// Who requests issue
    pub requester: AccountId,
    /// Vault's outer chain address
    pub outer_address: AddrStr,
    /// Amount that user wants to issue
    pub amount: Balance,
    /// Collateral locked to avoid user griefing
    pub griefing_collateral: Balance,
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
