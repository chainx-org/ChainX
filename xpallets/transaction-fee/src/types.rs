// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use codec::{Decode, Encode};
use pallet_transaction_payment::InclusionFee;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

use sp_runtime::RuntimeDebug;

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
pub struct FeeDetails<Balance> {
    /// The minimum fee for a transaction to be included in a block.
    pub inclusion_fee: Option<InclusionFee<Balance>>,
    // Do not serialize and deserialize `tip` as we actually can not pass any tip to the RPC.
    #[cfg_attr(feature = "std", serde(skip))]
    pub tip: Balance,
    pub extra_fee: Balance,
    pub final_fee: Balance,
}
