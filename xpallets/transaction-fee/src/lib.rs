// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

//! # Transaction Fee Module
//!
//! This module is a complement to pallet-transaction-payment module, unlike
//! pallet-transaction-payment which merely returns the value of final fee, it
//! exposes all the details of calculated transation fee in a struct `FeeDetails`.

#![cfg_attr(not(feature = "std"), no_std)]

use sp_std::prelude::*;

use codec::{Decode, Encode};
use frame_support::{
    decl_module,
    traits::{Currency, Get},
    weights::{DispatchInfo, Pays, PostDispatchInfo, Weight, WeightToFeePolynomial},
};
use sp_runtime::{
    traits::{DispatchInfoOf, Dispatchable, PostDispatchInfoOf, Saturating},
    FixedPointNumber, FixedPointOperand, RuntimeDebug,
};

type BalanceOf<T> = <<T as pallet_transaction_payment::Trait>::Currency as Currency<
    <T as frame_system::Trait>::AccountId,
>>::Balance;

pub trait Trait: pallet_transaction_payment::Trait {}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
    }
}

/// The base fee and adjusted weight and length fees constitute the _inclusion fee,_ which is
/// the minimum fee for a transaction to be included in a block.
///
/// ```ignore
/// inclusion_fee = base_fee + len_fee + [targeted_fee_adjustment * weight_fee];
/// ```
#[derive(Encode, Decode, Clone, Eq, PartialEq, RuntimeDebug)]
pub struct InclusionFee<Balance> {
    /// This is the minimum amount a user pays for a transaction. It is declared
    /// as a base _weight_ in the runtime and converted to a fee using `WeightToFee`.
    pub base_fee: Balance,
    /// The length fee, the amount paid for the encoded length (in bytes) of the transaction.
    pub len_fee: Balance,
    /// - `targeted_fee_adjustment`: This is a multiplier that can tune the final fee based on
    ///     the congestion of the network.
    /// - `weight_fee`: This amount is computed based on the weight of the transaction. Weight
    /// accounts for the execution time of a transaction.
    ///
    /// adjusted_weight_fee = targeted_fee_adjustment * weight_fee
    pub adjusted_weight_fee: Balance,
}

/// The `final_fee` is composed of:
///   - (Optional) `inclusion_fee`: Only the `Pays::Yes` transaction can have the inclusion fee.
///   - (Optional) `tip`: If included in the transaction, the tip will be added on top. Only
///     signed transactions can have a tip.
///
/// ```ignore
/// final_fee = inclusion_fee + tip;
/// ```
#[derive(Encode, Decode, Clone, Eq, PartialEq, RuntimeDebug)]
pub struct FeeDetails<Balance> {
    pub inclusion_fee: Option<InclusionFee<Balance>>,
    pub tip: Balance,
    pub final_fee: Balance,
}

impl<T: Trait> Module<T>
where
    BalanceOf<T>: FixedPointOperand,
{
    /// Returns the details of fee for a particular transaction.
    ///
    pub fn compute_fee_details(
        len: u32,
        info: &DispatchInfoOf<T::Call>,
        tip: BalanceOf<T>,
    ) -> FeeDetails<BalanceOf<T>>
    where
        T::Call: Dispatchable<Info = DispatchInfo>,
    {
        Self::compute_fee_raw(len, info.weight, tip, info.pays_fee)
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
        )
    }

    fn compute_fee_raw(
        len: u32,
        weight: Weight,
        tip: BalanceOf<T>,
        pays_fee: Pays,
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

            let base_fee = Self::weight_to_fee(T::ExtrinsicBaseWeight::get());
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
                final_fee: total,
            }
        } else {
            FeeDetails {
                inclusion_fee: None,
                tip,
                final_fee: tip,
            }
        }
    }

    fn weight_to_fee(weight: Weight) -> BalanceOf<T> {
        // cap the weight to the maximum defined in runtime, otherwise it will be the
        // `Bounded` maximum of its data type, which is not desired.
        let capped_weight = weight.min(<T as frame_system::Trait>::MaximumBlockWeight::get());
        T::WeightToFee::calc(&capped_weight)
    }
}
