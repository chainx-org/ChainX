#![cfg_attr(not(feature = "std"), no_std)]

/// Types used by pallet
pub mod types {
    use bitflags::bitflags;
    use codec::{Decode, Encode, Error, Input, Output};
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
        Error,
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

    impl Encode for ErrorCode {
        fn encode_to<T: Output + ?Sized>(&self, dest: &mut T) {
            dest.push_byte(self.bits())
        }
    }

    impl codec::EncodeLike for ErrorCode {}

    impl Decode for ErrorCode {
        fn decode<I: Input>(input: &mut I) -> Result<Self, Error> {
            Self::from_bits(input.read_byte()?).ok_or_else(|| Error::from("Invalid bytes"))
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
}

/// Manage exchanging between assets
#[frame_support::pallet]
#[allow(dead_code)]
pub mod pallet {
    use sp_arithmetic::traits::SaturatedConversion;
    use sp_std::{collections::btree_set::BTreeSet, default::Default, marker::PhantomData};

    #[cfg(feature = "std")]
    use frame_support::traits::GenesisBuild;
    use frame_support::{
        dispatch::{DispatchError, DispatchResult, DispatchResultWithPostInfo},
        ensure,
        storage::types::{StorageValue, ValueQuery},
        traits::{Currency, Hooks, IsType, ReservableCurrency},
    };
    use frame_system::{
        ensure_root, ensure_signed,
        pallet_prelude::{BlockNumberFor, OriginFor},
    };

    use super::types::{ErrorCode, Status, TradingPrice};

    pub type BalanceOf<T> = <<T as xpallet_assets::Config>::Currency as Currency<
        <T as frame_system::Config>::AccountId,
    >>::Balance;

    pub type CurrencyOf<T> = <T as xpallet_assets::Config>::Currency;

    #[pallet::pallet]
    #[pallet::generate_store(pub(crate) trait Store)]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::config]
    pub trait Config: frame_system::Config + xpallet_assets::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(n: BlockNumberFor<T>) -> frame_support::weights::Weight {
            let height = Self::exchange_rate_update_time();
            let period = Self::exchange_rate_expired_period();
            if n - height > period {
                <BridgeStatus<T>>::put(Status::Error);
                <BridgeErrorCodes<T>>::mutate(|errors| {
                    errors.insert(ErrorCode::EXCHANGE_RATE_EXPIRED)
                })
            };
            0u64.into()
        }

        fn on_finalize(_: BlockNumberFor<T>) {
            // recover from error if all errors were solved.
            if Self::bridge_error_codes().is_empty() {
                <BridgeStatus<T>>::put(Status::Running);
            }
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Force update the exchange rate.
        #[pallet::weight(0)]
        pub(crate) fn force_update_exchange_rate(
            origin: OriginFor<T>,
            exchange_rate: TradingPrice,
        ) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;
            Self::_update_exchange_rate(exchange_rate.clone())?;
            Self::deposit_event(Event::<T>::ExchangeRateForceUpdated(exchange_rate));
            Ok(().into())
        }

        /// Force update oracles.
        #[pallet::weight(0)]
        pub(crate) fn force_update_oracles(
            origin: OriginFor<T>,
            oracles: Vec<T::AccountId>,
        ) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;
            OracleAccounts::<T>::put(oracles.clone());
            Self::deposit_event(Event::<T>::OracleForceUpdated(oracles));
            Ok(().into())
        }

        /// Update exchange rate by oracle
        #[pallet::weight(0)]
        pub(crate) fn update_exchange_rate(
            origin: OriginFor<T>,
            exchange_rate: TradingPrice,
        ) -> DispatchResultWithPostInfo {
            let sender = ensure_signed(origin)?;
            ensure!(Self::is_oracle(&sender), Error::<T>::OperationForbidden);
            Self::_update_exchange_rate(exchange_rate.clone())?;
            Self::deposit_event(Event::<T>::ExchangeRateUpdated(sender, exchange_rate));
            Ok(().into())
        }
    }

    /// Errors for assets module
    #[pallet::error]
    pub enum Error<T> {
        /// Permission denied.
        OperationForbidden,
        /// Requester doesn't have enough pcx for collateral.
        InsufficientFunds,
        /// Arithmetic underflow/overflow.
        ArithmeticError,
        /// Account doesn't have enough collateral to be slashed.
        InsufficientCollateral,
        /// Bridge was shutdown or in error.
        BridgeNotRunning,
    }

    /// Events for assets module
    #[pallet::event]
    #[pallet::generate_deposit(pub(crate) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Update exchange rate by oracle
        ExchangeRateUpdated(T::AccountId, TradingPrice),
        /// Update exchange rate by root
        ExchangeRateForceUpdated(TradingPrice),
        /// Update oracles by root
        OracleForceUpdated(Vec<T::AccountId>),
    }

    /// Total collateral
    #[pallet::storage]
    #[pallet::getter(fn total_collateral)]
    pub(crate) type TotalCollateral<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

    /// Exchange rate from btc to pcx
    #[pallet::storage]
    #[pallet::getter(fn exchange_rate)]
    pub(crate) type ExchangeRate<T: Config> = StorageValue<_, TradingPrice, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn oracle_accounts)]
    pub(crate) type OracleAccounts<T: Config> = StorageValue<_, Vec<T::AccountId>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn bridge_status)]
    pub(crate) type BridgeStatus<T: Config> = StorageValue<_, Status, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn exchange_rate_update_time)]
    pub(crate) type ExchangeRateUpdateTime<T: Config> =
        StorageValue<_, BlockNumberFor<T>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn exchange_rate_expired_period)]
    pub(crate) type ExchangeRateExpiredPeriod<T: Config> =
        StorageValue<_, BlockNumberFor<T>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn bridge_error_codes)]
    pub(crate) type BridgeErrorCodes<T: Config> = StorageValue<_, ErrorCode, ValueQuery>;

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        /// pcx/btc trading pair
        pub exchange_rate: TradingPrice,
        /// oracles allowed to update exchange_rate
        pub oracle_accounts: Vec<T::AccountId>,
    }

    #[cfg(feature = "std")]
    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            Self {
                exchange_rate: Default::default(),
                oracle_accounts: Default::default(),
            }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
        fn build(&self) {
            <ExchangeRate<T>>::put(self.exchange_rate.clone());
            <OracleAccounts<T>>::put(self.oracle_accounts.clone());
        }
    }

    impl<T: Config> Pallet<T> {
        pub fn convert_to_pcx(btc_amount: BalanceOf<T>) -> Result<BalanceOf<T>, DispatchError> {
            //TODO(wangyafei): add lower bound?
            let exchange_rate = Self::exchange_rate();
            let result = exchange_rate
                .convert_to_pcx(btc_amount.saturated_into())
                .ok_or(Error::<T>::ArithmeticError)?;
            Ok(result.saturated_into())
        }
        /// Lock collateral
        #[inline]
        pub fn lock_collateral(sender: &T::AccountId, amount: BalanceOf<T>) -> DispatchResult {
            <<T as xpallet_assets::Config>::Currency as ReservableCurrency<
                <T as frame_system::Config>::AccountId,
            >>::reserve(sender, amount)
            .map_err(|_| Error::<T>::InsufficientFunds)?;
            Ok(())
        }

        /// increase total collateral
        #[inline]
        pub fn increase_total_collateral(amount: BalanceOf<T>) {
            <TotalCollateral<T>>::mutate(|c| *c += amount);
        }

        #[inline]
        pub(crate) fn is_oracle(account: &T::AccountId) -> bool {
            let oracles: Vec<T::AccountId> = Self::oracle_accounts();
            oracles.contains(account)
        }

        pub(crate) fn _update_exchange_rate(exchange_rate: TradingPrice) -> DispatchResult {
            // TODO: sanity check?
            <ExchangeRate<T>>::put(exchange_rate);
            let height = <frame_system::Pallet<T>>::block_number();
            <ExchangeRateUpdateTime<T>>::put(height);
            Self::recover_from_exchange_rate_expired();
            Ok(())
        }

        /// Slash collateral to receiver
        pub fn slash_collateral(
            sender: &T::AccountId,
            receiver: &T::AccountId,
            amount: BalanceOf<T>,
        ) -> DispatchResult {
            let reserved_collateral = <CurrencyOf<T>>::reserved_balance(sender);
            ensure!(
                reserved_collateral >= amount,
                Error::<T>::InsufficientCollateral
            );
            let (slashed, _) = <CurrencyOf<T>>::slash_reserved(sender, amount);

            <CurrencyOf<T>>::resolve_creating(receiver, slashed);
            <CurrencyOf<T>>::reserve(receiver, amount)
                .map_err(|_| Error::<T>::InsufficientFunds)?;
            // Self::deposit_event(...);
            Ok(().into())
        }

        #[inline]
        pub(crate) fn ensure_bridge_running() -> DispatchResult {
            ensure!(
                Self::bridge_status() == Status::Running,
                Error::<T>::BridgeNotRunning
            );
            Ok(())
        }

        /// Clarify `ExchangeRateExpired` is solved and recover from this error.
        /// Dangerous! Ensure this error truly solved is caller's responsibility.
        pub(crate) fn recover_from_exchange_rate_expired() {
            if Self::bridge_status() == Status::Error {
                let error_codes = Self::bridge_error_codes();
                if error_codes.contains(ErrorCode::EXCHANGE_RATE_EXPIRED) {
                    <BridgeErrorCodes<T>>::mutate(|v| v.remove(ErrorCode::EXCHANGE_RATE_EXPIRED));
                }
            }
        }

        /// Clarify `Liquidating` is solved and recover from this error.
        /// Dangerous! Ensure this error truly solved is caller's responsibility.
        pub(crate) fn recover_from_liquidating() {
            if Self::bridge_status() == Status::Error {
                let error_codes = Self::bridge_error_codes();
                if error_codes.contains(ErrorCode::LIQUIDATING) {
                    <BridgeErrorCodes<T>>::mutate(|v| v.remove(ErrorCode::LIQUIDATING));
                }
            }
        }
    }
}
