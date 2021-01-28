#![cfg_attr(not(feature = "std"), no_std)]

/// Types used by pallet
pub mod types {
    use codec::{Decode, Encode};
    use sp_runtime::RuntimeDebug;

    #[cfg(feature = "std")]
    use serde::{Deserialize, Serialize};

    /// This struct represents the price of trading pair PCX/BTC.
    ///
    /// For example, the current price of PCX/BTC in some
    /// exchange is 0.0001779 which will be represented as
    /// `ExchangeRate { price: 1779, decimal: 7 }`.
    #[derive(Encode, Decode, RuntimeDebug, Clone, Default, Eq, PartialEq)]
    #[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
    pub struct ExchangeRate {
        /// Price with decimals.
        pub price: u128,
        /// How many decimals of the exchange price.
        pub decimal: u8,
    }

    impl ExchangeRate {
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
        use super::ExchangeRate;
        #[test]
        fn test_btc_conversion() {
            let exchange_rate = ExchangeRate {
                price: 1,
                decimal: 4,
            };
            assert_eq!(exchange_rate.convert_to_btc(10000), Some(1));
        }

        #[test]
        fn test_pcx_conversion() {
            let exchange_rate = ExchangeRate {
                price: 1,
                decimal: 4,
            };
            assert_eq!(exchange_rate.convert_to_pcx(1), Some(10000));

            let exchange_rate = ExchangeRate {
                price: 1,
                decimal: 38,
            };
            assert_eq!(exchange_rate.convert_to_pcx(1_000_000), None);
        }
    }
}

/// Manage exchanging between assets
#[frame_support::pallet]
#[allow(dead_code)]
pub mod pallet {
    use sp_arithmetic::traits::SaturatedConversion;
    use sp_std::marker::PhantomData;

    #[cfg(feature = "std")]
    use frame_support::traits::GenesisBuild;
    use frame_support::{
        dispatch::{DispatchError, DispatchResult, DispatchResultWithPostInfo},
        ensure,
        storage::types::{StorageValue, ValueQuery},
        traits::{Currency, Hooks, ReservableCurrency},
    };
    use frame_system::{
        ensure_root, ensure_signed,
        pallet_prelude::{BlockNumberFor, OriginFor},
    };

    use super::types;

    pub type BalanceOf<T> = <<T as xpallet_assets::Config>::Currency as Currency<
        <T as frame_system::Config>::AccountId,
    >>::Balance;

    #[pallet::pallet]
    #[pallet::generate_store(pub(crate) trait Store)]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::config]
    pub trait Config: frame_system::Config + xpallet_assets::Config {}

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Force update the exchange rate.
        #[pallet::weight(0)]
        pub(crate) fn force_update_exchange_rate(
            origin: OriginFor<T>,
            exchange_rate: types::ExchangeRate,
        ) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;
            // TODO: sanity check?
            ExchangeRate::<T>::put(exchange_rate);
            // Self::deposit_event(Event::ExchangeRateSet(exchange_rate));
            Ok(().into())
        }

        #[pallet::weight(0)]
        pub(crate) fn update_exchange_rate(
            origin: OriginFor<T>,
            exchange_rate: types::ExchangeRate,
        ) -> DispatchResultWithPostInfo {
            let sender = ensure_signed(origin)?;
            ensure!(Self::is_oracle(&sender), Error::<T>::OperationForbidden);
            // TODO: sanity check?
            ExchangeRate::<T>::put(exchange_rate);
            // Self::deposit_event(Event::ExchangeRateSet(exchange_rate));
            Ok(().into())
        }
    }

    /// Errors for assets module
    #[pallet::error]
    pub enum Error<T> {
        /// Permission denied.
        OperationForbidden,
        /// Requester doesn't has enough pcx for collateral.
        InsufficientFunds,
        /// Arithmetic underflow/overflow.
        ArithmeticError,
    }

    /// Total collateral
    #[pallet::storage]
    #[pallet::getter(fn total_collateral)]
    pub(crate) type TotalCollateral<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

    /// Exchange rate from btc to pcx
    #[pallet::storage]
    #[pallet::getter(fn exchange_rate)]
    pub(crate) type ExchangeRate<T: Config> = StorageValue<_, types::ExchangeRate, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn oracle_accounts)]
    pub(crate) type OracleAccounts<T: Config> = StorageValue<_, Vec<T::AccountId>, ValueQuery>;

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        /// pcx/btc trading pair
        pub exchange_rate: types::ExchangeRate,
        /// oracles allowed to update exchange_rate
        pub oracle_accounts: Vec<T::AccountId>,
    }

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
    }
}
