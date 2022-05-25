// Copyright 2019-2022 ChainX Project Authors. Licensed under GPL-3.0.

//! # Transaction Fee Module

#![cfg_attr(not(feature = "std"), no_std)]

mod types;

pub use self::types::FeeDetails;
pub use pallet_transaction_payment::InclusionFee;

type BalanceOf<T> = <<T as pallet_transaction_payment::Config>::OnChargeTransaction as pallet_transaction_payment::OnChargeTransaction<T>>::Balance;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;

    #[pallet::pallet]
    #[pallet::generate_store(pub(crate) trait Store)]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_transaction_payment::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
    }

    #[pallet::event]
    pub enum Event<T: Config> {
        /// Transaction fee was paid to the block author and its reward pot in 1:9.
        /// [author, author_fee, reward_pot, reward_pot_fee]
        FeePaid(T::AccountId, BalanceOf<T>, T::AccountId, BalanceOf<T>),
    }
}
