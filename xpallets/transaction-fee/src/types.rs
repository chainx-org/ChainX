// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use codec::{Decode, Encode};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

use sp_runtime::RuntimeDebug;

// replace by the struct InclusionFee in substrate frame
// /// The base fee and adjusted weight and length fees constitute the _inclusion fee,_ which is
// /// the minimum fee for a transaction to be included in a block.
// ///
// /// ```ignore
// /// inclusion_fee = base_fee + len_fee + [targeted_fee_adjustment * weight_fee];
// /// ```
// #[derive(Encode, Decode, Clone, Eq, PartialEq, RuntimeDebug)]
// #[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
// #[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
// pub struct InclusionFee<Balance> {
//     /// This is the minimum amount a user pays for a transaction. It is declared
//     /// as a base _weight_ in the runtime and converted to a fee using `WeightToFee`.
//     pub base_fee: Balance,
//     /// The length fee, the amount paid for the encoded length (in bytes) of the transaction.
//     pub len_fee: Balance,
//     /// - `targeted_fee_adjustment`: This is a multiplier that can tune the final fee based on
//     ///     the congestion of the network.
//     /// - `weight_fee`: This amount is computed based on the weight of the transaction. Weight
//     /// accounts for the execution time of a transaction.
//     ///
//     /// adjusted_weight_fee = targeted_fee_adjustment * weight_fee
//     pub adjusted_weight_fee: Balance,
// }

/// The `final_fee` is composed of:
///   - (Optional) `inclusion_fee`: Only the `Pays::Yes` transaction can have the inclusion fee.
///   - (Optional) `tip`: If included in the transaction, the tip will be added on top. Only
///     signed transactions can have a tip.
///
/// ```ignore
/// final_fee = inclusion_fee + tip;
/// ```
#[derive(Encode, Decode, Clone, Eq, PartialEq, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct FeeDetails<Balance: Default> {
    /// Some calls might be charged extra fee besides the essential `inclusion_fee`.
    /// use the struct Feedetails in substrate instead of the previous protery inclusion_fee and tip
    pub partial_details: pallet_transaction_payment::FeeDetails<Balance>,
    pub extra_fee: Balance,
    pub final_fee: Balance,
}

