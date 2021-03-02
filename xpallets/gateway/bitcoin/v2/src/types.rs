use bitflags::bitflags;
use codec::{Decode, Encode};
use sp_runtime::RuntimeDebug;

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

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
        Status::Running
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
