// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

//! # Transaction Fee Module
//!
//! This module is a complement to pallet-transaction-payment module, unlike
//! pallet-transaction-payment which merely returns the value of final fee, it
//! exposes all the details of calculated transation fee in a struct `FeeDetails`.
//!
//! The future improvement is to make this feature native to Substrate's transaction-payment
//! module so that we don't have to copy and paste the core logic of fee calculation.

#![cfg_attr(not(feature = "std"), no_std)]

mod types;

use sp_std::prelude::*;

use frame_support::weights::{DispatchInfo, GetDispatchInfo};
use sp_runtime::{
    traits::{Dispatchable, Saturating},
    FixedPointOperand,
};

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
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_transaction_payment::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
    }

    #[pallet::event]
    #[pallet::metadata(T::AccountId = "AccountId", BalanceOf<T> = "Balance")] // Optional
    #[pallet::generate_deposit(pub(super) fn deposit_event)] // Optional
    pub enum Event<T: Config> {
        /// Transaction fee was paid to the block author and its reward pot in 1:9.
        /// [author, author_fee, reward_pot, reward_pot_fee]
        FeePaid(T::AccountId, BalanceOf<T>, T::AccountId, BalanceOf<T>),
    }
}
