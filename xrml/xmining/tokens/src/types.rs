// Copyright 2018-2019 Chainpool.

use parity_codec::{Decode, Encode};
#[cfg(feature = "std")]
use serde_derive::{Deserialize, Serialize};

use super::{Token, Trait};
use primitives::traits::As;
use xstaking::{VoteWeight, VoteWeightBase, VoteWeightBaseV1, VoteWeightV1};
use xsupport::trace;

// Declare the PseduIntentionVoteWeight(V1) and impl VoteWeight(V1) accrodingly.
macro_rules! psedu_intention_vote_weight{
    ( $($struct_name:ident, $struct_wrapper_name:ident: ($base_trait:ident, $trait:ident) => $weight_type:ty;)+ ) => {
        $(
            /// This module only tracks the vote weight related changes.
            /// All the amount related has been taken care by assets module.
            #[derive(PartialEq, Eq, Clone, Encode, Decode, Default)]
            #[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
            #[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
            pub struct $struct_name<BlockNumber: Default> {
                pub last_total_deposit_weight: $weight_type,
                pub last_total_deposit_weight_update: BlockNumber,
            }

            /// `PseduIntentionProfs` and `DepositRecord` is to wrap the vote weight of token,
            /// sharing the vote weight calculation logic originated from staking module.
            pub struct $struct_wrapper_name<'a, T: Trait> {
                pub token: &'a Token,
                pub staking: &'a mut $struct_name<T::BlockNumber>,
            }

            impl<'a, T: Trait> $struct_wrapper_name<'a, T> {
                pub fn new(token: &'a Token, staking: &'a mut $struct_name<T::BlockNumber>) -> Self {
                    $struct_wrapper_name { token, staking }
                }
            }

            impl<'a, T: Trait> $base_trait<T::BlockNumber> for $struct_wrapper_name<'a, T> {
                fn amount(&self) -> u64 {
                    xassets::Module::<T>::all_type_total_asset_balance(&self.token).as_()
                }

                fn last_acum_weight(&self) -> $weight_type {
                    self.staking.last_total_deposit_weight
                }

                fn last_acum_weight_update(&self) -> u64 {
                    self.staking.last_total_deposit_weight_update.as_()
                }

                fn set_amount(&mut self, _: u64) {}

                fn set_last_acum_weight(&mut self, latest_deposit_weight: $weight_type) {
                    trace!(
                        target: "tokens",
                        "[set_last_acum_weight] [psudu_intention] amount: {:?}, last_acum_weight: {:?}, last_acum_weight_update: {:?}, current_block: {:?}, latest_deposit_weight: {:?}",
                        self.amount(),
                        self.last_acum_weight(),
                        self.last_acum_weight_update(),
                        <system::Module<T>>::block_number(),
                        latest_deposit_weight
                    );
                    self.staking.last_total_deposit_weight = latest_deposit_weight;
                }

                fn set_last_acum_weight_update(&mut self, current_block: T::BlockNumber) {
                    self.staking.last_total_deposit_weight_update = current_block;
                }
            }

            impl<'a, T: Trait> $trait<T::BlockNumber> for $struct_wrapper_name<'a, T> {}
        )+
    }
}

psedu_intention_vote_weight! {
    PseduIntentionVoteWeight, PseduIntentionProfs: (VoteWeightBase, VoteWeight) => u64;
    PseduIntentionVoteWeightV1, PseduIntentionProfsV1: (VoteWeightBaseV1, VoteWeightV1) => u128;
}

/// Declare the DepositVoteWeight(V1) and impl VoteWeight(V1) accrodingly.
macro_rules! deposit_vote_weight {
    ( $($struct_name:ident, $record_name:ident : ($base_trait:ident, $trait:ident) => $weight_type:ty;)+ ) => {
        $(
            /// This module only tracks the vote weight related changes.
            /// All the amount related has been taken care by assets module.
            #[derive(PartialEq, Eq, Clone, Encode, Decode, Default)]
            #[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
            #[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
            pub struct $struct_name<BlockNumber: Default> {
                pub last_deposit_weight: $weight_type,
                pub last_deposit_weight_update: BlockNumber,
            }

            impl<BlockNumber: Default> $struct_name<BlockNumber> {
                pub fn new(last_deposit_weight: $weight_type, last_deposit_weight_update: BlockNumber) -> Self {
                    Self {
                        last_deposit_weight,
                        last_deposit_weight_update,
                    }
                }
            }

            pub struct $record_name<'a, T: Trait> {
                pub depositor: &'a T::AccountId,
                pub token: &'a Token,
                pub staking: &'a mut $struct_name<T::BlockNumber>,
            }

            impl<'a, T: Trait> $record_name<'a, T> {
                pub fn new(
                    depositor: &'a T::AccountId,
                    token: &'a Token,
                    staking: &'a mut $struct_name<T::BlockNumber>,
                ) -> Self {
                    $record_name {
                        depositor,
                        token,
                        staking,
                    }
                }
            }

            impl<'a, T: Trait> $base_trait<T::BlockNumber> for $record_name<'a, T> {
                fn amount(&self) -> u64 {
                    xassets::Module::<T>::all_type_asset_balance(&self.depositor, &self.token).as_()
                }

                fn last_acum_weight(&self) -> $weight_type {
                    self.staking.last_deposit_weight
                }

                fn last_acum_weight_update(&self) -> u64 {
                    self.staking.last_deposit_weight_update.as_()
                }

                fn set_amount(&mut self, _: u64) {}

                fn set_last_acum_weight(&mut self, latest_deposit_weight: $weight_type) {
                    trace!(
                        target: "tokens",
                        "[set_last_acum_weight] [depositor] amount: {:?}, last_acum_weight: {:?}, last_acum_weight_update: {:?}, current_block: {:?} => latest_deposit_weight: {:?}",
                        self.amount(),
                        self.last_acum_weight(),
                        self.last_acum_weight_update(),
                        <system::Module<T>>::block_number(),
                        latest_deposit_weight
                    );
                    self.staking.last_deposit_weight = latest_deposit_weight;
                }

                fn set_last_acum_weight_update(&mut self, current_block: T::BlockNumber) {
                    self.staking.last_deposit_weight_update = current_block;
                }
            }

            impl<'a, T: Trait> $trait<T::BlockNumber> for $record_name<'a, T> {}

        )+
    }
}

deposit_vote_weight! {
    DepositVoteWeight, DepositRecord: (VoteWeightBase, VoteWeight) => u64;
    DepositVoteWeightV1, DepositRecordV1: (VoteWeightBaseV1, VoteWeightV1) => u128;
}

impl<BlockNumber: Default> From<PseduIntentionVoteWeight<BlockNumber>>
    for PseduIntentionVoteWeightV1<BlockNumber>
{
    fn from(prof: PseduIntentionVoteWeight<BlockNumber>) -> Self {
        PseduIntentionVoteWeightV1 {
            last_total_deposit_weight: u128::from(prof.last_total_deposit_weight),
            last_total_deposit_weight_update: prof.last_total_deposit_weight_update,
        }
    }
}

impl<BlockNumber: Default> From<DepositVoteWeight<BlockNumber>>
    for DepositVoteWeightV1<BlockNumber>
{
    fn from(d_vote_weight: DepositVoteWeight<BlockNumber>) -> Self {
        DepositVoteWeightV1 {
            last_deposit_weight: u128::from(d_vote_weight.last_deposit_weight),
            last_deposit_weight_update: d_vote_weight.last_deposit_weight_update,
        }
    }
}
