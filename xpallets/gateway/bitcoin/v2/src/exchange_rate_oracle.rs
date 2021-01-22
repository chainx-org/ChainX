#![cfg_attr(not(feature = "std"), no_std)]

#[frame_support::pallet]
#[allow(dead_code)]
pub mod pallet {
    use sp_std::{convert::TryInto, marker::PhantomData};

    use frame_support::{
        dispatch::DispatchError,
        storage::types::{StorageValue, ValueQuery},
        traits::Currency,
        traits::{GenesisBuild, Hooks},
    };
    use frame_system::pallet_prelude::BlockNumberFor;

    type BalanceOf<T> =
        <<T as Config>::Balances as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    #[pallet::pallet]
    #[pallet::generate_store(pub(crate) trait Store)]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type Balances: Currency<Self::AccountId>;
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::call]
    impl<T: Config> Pallet<T> {}

    #[pallet::error]
    pub enum Error<T> {
        ArithmeticOverflow,
        ArithmeticUnderflow,
        TryIntoError,
    }

    #[pallet::storage]
    #[pallet::getter(fn exchange_rate)]
    pub(crate) type ExchangeRate<T: Config> = StorageValue<_, u128, ValueQuery>;

    #[pallet::genesis_config]
    #[derive(Default)]
    pub struct GenesisConfig {
        /// Exchange rate from btc to pcx
        pub exchange_rate: u128,
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig {
        fn build(&self) {
            <ExchangeRate<T>>::put(self.exchange_rate);
        }
    }

    impl<T: Config> Pallet<T> {
        fn into_u128<I: TryInto<u128>>(x: I) -> Result<u128, DispatchError> {
            TryInto::<u128>::try_into(x).map_err(|_| Error::<T>::TryIntoError.into())
        }
        pub fn btc_to_pcx(amount: BalanceOf<T>) -> Result<BalanceOf<T>, DispatchError> {
            let raw_amount = Self::into_u128(amount)?;
            let rate = Self::exchange_rate()
                .checked_mul(raw_amount)
                .ok_or(Error::<T>::ArithmeticOverflow)?
                .checked_div(100_000u128)
                .ok_or(Error::<T>::ArithmeticUnderflow)?;
            let result = rate.try_into().map_err(|_| Error::<T>::TryIntoError)?;
            Ok(result)
        }

        pub fn pcx_to_btc(amount: BalanceOf<T>) -> Result<BalanceOf<T>, DispatchError> {
            let raw_amount = Self::into_u128(amount)?;
            let rate = raw_amount
                .checked_mul(100_000u128)
                .ok_or(Error::<T>::ArithmeticOverflow)?
                .checked_div(Self::exchange_rate())
                .ok_or(Error::<T>::ArithmeticUnderflow)?;
            let result = rate.try_into().map_err(|_| Error::<T>::TryIntoError)?;
            Ok(result)
        }
    }
}
