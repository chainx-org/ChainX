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

    #[pallet::call]
    impl<T: Config> Pallet<T> {}

    #[pallet::event]
    #[pallet::metadata(T::AccountId = "AccountId", BalanceOf<T> = "Balance")] // Optional
    #[pallet::generate_deposit(pub(super) fn deposit_event)] // Optional
    pub enum Event<T: Config> {
        /// Transaction fee was paid to the block author and its reward pot in 1:9.
        /// [author, author_fee, reward_pot, reward_pot_fee]
        FeePaid(T::AccountId, BalanceOf<T>, T::AccountId, BalanceOf<T>),
    }
}

impl<T: Config> Pallet<T>
where
    BalanceOf<T>: FixedPointOperand,
{
    /// Returns the details of fee for a particular transaction.
    ///
    /// The basic logic is identical to [`compute_fee`] but returns
    /// the details of final fee instead.
    ///
    /// [`compute_fee`]: https://docs.rs/pallet-transaction-payment/2.0.0/pallet_transaction_payment/struct.Module.html#method.compute_fee
    pub fn query_fee_details<Extrinsic: GetDispatchInfo>(
        unchecked_extrinsic: Extrinsic,
        len: u32,
    ) -> FeeDetails<BalanceOf<T>>
    where
        T: Send + Sync,
        BalanceOf<T>: Send + Sync + Default,
        T::Call: Dispatchable<Info = DispatchInfo>,
    {
        let dispatch_info = <Extrinsic as GetDispatchInfo>::get_dispatch_info(&unchecked_extrinsic);
        let details = pallet_transaction_payment::Module::<T>::compute_fee_details(
            len,
            &dispatch_info,
            0u32.into(),
        );

        match details.inclusion_fee {
            Some(fee) => {
                let total = fee
                    .base_fee
                    .saturating_add(fee.len_fee)
                    .saturating_add(fee.adjusted_weight_fee)
                    .saturating_add(details.tip);
                FeeDetails {
                    inclusion_fee: Some(fee),
                    tip: details.tip,
                    extra_fee: 0u32.into(),
                    final_fee: total,
                }
            }
            None => FeeDetails {
                inclusion_fee: None,
                tip: details.tip,
                extra_fee: 0u32.into(),
                final_fee: details.tip,
            },
        }
    }
}
