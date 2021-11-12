// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use codec::{Decode, Encode};
use scale_info::TypeInfo;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

use sp_runtime::{traits::AtLeast32BitUnsigned, RuntimeDebug};

use pallet_transaction_payment::InclusionFee;

/// The `final_fee` is composed of:
///   - (Optional) `inclusion_fee`: Only the `Pays::Yes` transaction can have the inclusion fee.
///   - (Optional) `tip`: If included in the transaction, the tip will be added on top. Only
///     signed transactions can have a tip.
///
/// ```ignore
/// final_fee = inclusion_fee + tip;
/// ```
#[derive(Encode, Decode, Clone, Eq, PartialEq, RuntimeDebug, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct FeeDetails<Balance> {
    /// The minimum fee for a transaction to be included in a block.
    pub inclusion_fee: Option<InclusionFee<Balance>>,
    // Do not serialize and deserialize `tip` as we actually can not pass any tip to the RPC.
    #[cfg_attr(feature = "std", serde(skip))]
    pub tip: Balance,
    /// Additional fee for some ChainX specific calls.
    pub extra_fee: Balance,
    pub final_fee: Balance,
}

impl<Balance: AtLeast32BitUnsigned + Copy> FeeDetails<Balance> {
    pub fn new(
        base: pallet_transaction_payment::FeeDetails<Balance>,
        maybe_extra_fee: Option<Balance>,
    ) -> Self {
        match maybe_extra_fee {
            Some(extra_fee) => Self {
                extra_fee,
                final_fee: base.final_fee() + extra_fee,
                ..base.into()
            },
            None => base.into(),
        }
    }
}

impl<Balance: AtLeast32BitUnsigned + Copy> From<pallet_transaction_payment::FeeDetails<Balance>>
    for FeeDetails<Balance>
{
    fn from(details: pallet_transaction_payment::FeeDetails<Balance>) -> Self {
        let final_fee = details.final_fee();
        Self {
            inclusion_fee: details.inclusion_fee,
            tip: details.tip,
            extra_fee: 0u32.into(),
            final_fee,
        }
    }
}
