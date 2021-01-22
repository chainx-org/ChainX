#![cfg_attr(not(feature = "std"), no_std)]

#[frame_support::pallet]
#[allow(dead_code)]
pub mod pallet {
    use sp_std::marker::PhantomData;

    use frame_support::{
        dispatch::DispatchResult,
        storage::types::{StorageValue, ValueQuery},
        traits::{Currency, Hooks, ReservableCurrency},
    };
    use frame_system::pallet_prelude::BlockNumberFor;

    type BalanceOf<T> =
        <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    #[pallet::pallet]
    #[pallet::generate_store(pub(crate) trait Store)]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::config]
    pub trait Config: frame_system::Config + xpallet_assets::Config {
        type Currency: ReservableCurrency<Self::AccountId>;
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::call]
    impl<T: Config> Pallet<T> {}
}
