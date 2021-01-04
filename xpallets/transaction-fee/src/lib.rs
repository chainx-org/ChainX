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

use frame_support::{
    decl_event, decl_module,
    traits::Get,
    weights::{
        DispatchClass, DispatchInfo, GetDispatchInfo, Pays, PostDispatchInfo, Weight,
        WeightToFeePolynomial,
    },
};
use sp_runtime::{
    traits::{DispatchInfoOf, Dispatchable, PostDispatchInfoOf, Saturating},
    FixedPointNumber, FixedPointOperand,
};

pub use self::types::{FeeDetails, InclusionFee};

type BalanceOf<T> = <<T as pallet_transaction_payment::Config>::OnChargeTransaction as pallet_transaction_payment::OnChargeTransaction<T>>::Balance;

pub trait Trait: pallet_transaction_payment::Config {}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {}
}

decl_event!(
    /// Event for the XTransactionFee Module
    pub enum Event<T>
    where
        Balance = BalanceOf<T>,
        <T as frame_system::Config>::AccountId,
    {
        /// Transaction fee was paid to the block author and its reward pot in 1:9.
        /// [author, author_fee, reward_pot, reward_pot_fee]
        FeePaid(AccountId, Balance, AccountId, Balance),
    }
);

impl<T: Trait> Module<T>
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
        BalanceOf<T>: Send + Sync,
        T::Call: Dispatchable<Info = DispatchInfo>,
    {
        let dispatch_info = <Extrinsic as GetDispatchInfo>::get_dispatch_info(&unchecked_extrinsic);
        Self::compute_fee(len, &dispatch_info, 0u32.into())
    }

    pub fn compute_fee(
        len: u32,
        info: &DispatchInfoOf<T::Call>,
        tip: BalanceOf<T>,
    ) -> FeeDetails<BalanceOf<T>>
    where
        T::Call: Dispatchable<Info = DispatchInfo>,
    {
        Self::compute_fee_raw(len, info.weight, tip, info.pays_fee, info.class)
    }

    /// Returns the details of the actual post dispatch fee for a particular transaction.
    ///
    /// Identical to `compute_fee_details` with the only difference that the post dispatch corrected
    /// weight is used for the weight fee calculation.
    pub fn compute_actual_fee_details(
        len: u32,
        info: &DispatchInfoOf<T::Call>,
        post_info: &PostDispatchInfoOf<T::Call>,
        tip: BalanceOf<T>,
    ) -> FeeDetails<BalanceOf<T>>
    where
        T::Call: Dispatchable<Info = DispatchInfo, PostInfo = PostDispatchInfo>,
    {
        Self::compute_fee_raw(
            len,
            post_info.calc_actual_weight(info),
            tip,
            post_info.pays_fee(info),
            info.class,
        )
    }

    fn compute_fee_raw(
        len: u32,
        weight: Weight,
        tip: BalanceOf<T>,
        pays_fee: Pays,
        class: DispatchClass,
    ) -> FeeDetails<BalanceOf<T>> {
        if pays_fee == Pays::Yes {
            let len = <BalanceOf<T>>::from(len);
            let per_byte = T::TransactionByteFee::get();

            // length fee. this is not adjusted.
            let fixed_len_fee = per_byte.saturating_mul(len);

            // the adjustable part of the fee.
            let unadjusted_weight_fee = Self::weight_to_fee(weight);
            let multiplier = pallet_transaction_payment::Module::<T>::next_fee_multiplier();
            // final adjusted weight fee.
            let adjusted_weight_fee = multiplier.saturating_mul_int(unadjusted_weight_fee);

            let base_fee = Self::weight_to_fee(T::BlockWeights::get().get(class).base_extrinsic);
            let total = base_fee
                .saturating_add(fixed_len_fee)
                .saturating_add(adjusted_weight_fee)
                .saturating_add(tip);

            FeeDetails {
                inclusion_fee: Some(InclusionFee {
                    base_fee,
                    len_fee: fixed_len_fee,
                    adjusted_weight_fee,
                }),
                tip,
                extra_fee: 0u32.into(),
                final_fee: total,
            }
        } else {
            FeeDetails {
                inclusion_fee: None,
                tip,
                extra_fee: 0u32.into(),
                final_fee: tip,
            }
        }
    }

    fn weight_to_fee(weight: Weight) -> BalanceOf<T> {
        // cap the weight to the maximum defined in runtime, otherwise it will be the
        // `Bounded` maximum of its data type, which is not desired.
        let capped_weight = weight.min(T::BlockWeights::get().max_block);
        T::WeightToFee::calc(&capped_weight)
    }
}
