#![cfg_attr(not(feature = "std"), no_std)]

#[frame_support::pallet]
pub mod vault {
    use frame_support::traits::{Currency, ReservableCurrency};
    use frame_support::{pallet_prelude::*, storage::types::ValueQuery};
    use frame_system::pallet_prelude::*;
    use sp_runtime::DispatchResult;
    use v1::BtcAddress;

    pub type BalanceOf<T> =
        <<T as Config>::PCX as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type PCX: Currency<Self::AccountId> + ReservableCurrency<Self::AccountId>;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Register a vault.
        #[pallet::weight(0)]
        fn register_vault(
            origin: OriginFor<T>,
            collateral: BalanceOf<T>,
            btc_address: BtcAddress,
        ) -> DispatchResultWithPostInfo {
            let sender = ensure_signed(origin);
            Ok(().into())
        }
    }

    #[pallet::type_value]
    pub(super) fn DefaultCollateral<T: Config>() -> BalanceOf<T> {
        0.into()
    }

    #[pallet::storage]
    #[pallet::getter(fn total_collateral)]
    pub(super) type TotalCollateral<T: Config> =
        StorageValue<_, BalanceOf<T>, ValueQuery, DefaultCollateral<T>>;

    #[pallet::error]
    pub enum Error<T> {
        InsufficientFunds,
    }

    impl<T: Config> Pallet<T> {
        /// Lock collateral
        fn lock_collateral(sender: &T::AccountId, amount: BalanceOf<T>) -> DispatchResult {
            T::PCX::reserve(sender, amount).map_err(|_| Error::<T>::InsufficientFunds)?;
            Ok(())
        }

        /// increase total collateral
        fn increase_total_collateral(amount: BalanceOf<T>) {
            let new_collateral = Self::total_collateral() + amount;
            <TotalCollateral<T>>::put(new_collateral);
        }
    }
}
