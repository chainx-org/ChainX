// Copyright 2018-2019 Chainpool.

use super::*;
use parity_codec::{Decode, Encode};
use primitives::traits::SimpleArithmetic;
#[cfg(feature = "std")]
use serde_derive::{Deserialize, Serialize};

/// A wrapper for the change of staking(nominate, renominate, unnominate) amount.
///
/// The delta of staking amount is Zero when claiming the reward.
pub enum Delta {
    Add(u64),
    Sub(u64),
    Zero,
}

/// A wrapper to unify the u64 and u128 vote weight type.
#[derive(PartialEq, Eq, Clone, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub enum WeightType {
    U64(u64),
    U128(u128),
}

impl Default for WeightType {
    fn default() -> Self {
        WeightType::U64(Default::default())
    }
}

impl WeightType {
    /// Extract the inner value and extend it to u128 if it's u64.
    pub fn into_inner_safe(&self) -> u128 {
        match *self {
            WeightType::U64(x) => u128::from(x),
            WeightType::U128(x) => x,
        }
    }
}

/// RewardHolder includes intention as well as tokens.
#[derive(PartialEq, Eq, Clone, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub enum RewardHolder<AccountId: Default> {
    Intention(AccountId),
    PseduIntention(Token),
}

impl<AccountId: Default> Default for RewardHolder<AccountId> {
    fn default() -> Self {
        RewardHolder::Intention(Default::default())
    }
}

/// Declare the struct IntentionProfs(V1) and impl VoteWeight(V1) trait accordingly.
///
/// IntentionProfs and IntentionProfsV1 are essentially the same except that
/// the type of last_total_vote_weight field is different, u64 for IntentionProfs,
/// u128 for IntentionProfsV1.
///
/// The reason for this extension is that we can only afford 11698 PCX bonding
/// for one whole year with respect to u64, i.e., the accumulative vote weight
/// does have the overflow risk in this case, which is unacceptable.
///
/// seconds_per_block = 2
/// blocks_per_year = 24 * 60 * 60 / 2 * 365
/// u64::max_value() / blocks_per_year / 10u64.pow(8) =~ 11698 PCX
///
/// With u128, the overflow concern is ensured to be eliminated ptractially.
///
/// pcx_total_issueance = 2_100_000_000_000_000
///
///   u128::max_value() / pcx_total_issueance / blocks_per_year
/// = 10276460067434299 YEAR
macro_rules! intention_profs {
    ( $($struct_name:ident : ($weight_base_trait:ident, $weight_trait:ident) => $weight_type:ty;)+ ) => {
        $(
            #[derive(PartialEq, Eq, Clone, Encode, Decode, Default)]
            #[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
            #[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
            pub struct $struct_name<Balance: Default, BlockNumber: Default> {
                pub total_nomination: Balance,
                pub last_total_vote_weight: $weight_type,
                pub last_total_vote_weight_update: BlockNumber,
            }

            impl<Balance: Default, BlockNumber: Default> $struct_name<Balance, BlockNumber> {
                pub fn new(total_nomination: Balance, last_total_vote_weight: $weight_type, last_total_vote_weight_update: BlockNumber) -> Self {
                    Self {
                        total_nomination,
                        last_total_vote_weight,
                        last_total_vote_weight_update,
                    }
                }
            }

            impl<Balance: Default + As<u64> + Clone, BlockNumber: Default + As<u64> + Clone>
                $weight_base_trait<BlockNumber> for $struct_name<Balance, BlockNumber>
            {
                fn amount(&self) -> u64 {
                    self.total_nomination.clone().as_()
                }

                fn set_amount(&mut self, new: u64) {
                    self.total_nomination = Balance::sa(new);
                }

                fn last_acum_weight(&self) -> $weight_type {
                    self.last_total_vote_weight as $weight_type
                }

                fn set_last_acum_weight(&mut self, latest_vote_weight: $weight_type) {
                    self.last_total_vote_weight = latest_vote_weight;
                }

                fn last_acum_weight_update(&self) -> u64 {
                    self.last_total_vote_weight_update.clone().as_()
                }

                fn set_last_acum_weight_update(&mut self, current_block: BlockNumber) {
                    self.last_total_vote_weight_update = current_block;
                }
            }

            impl<Balance: Default + Clone + As<u64>, BlockNumber: Default + Clone + As<u64>>
                $weight_trait<BlockNumber> for $struct_name<Balance, BlockNumber>
            {
            }
        )+
    };
}

intention_profs! {
    IntentionProfs : (VoteWeightBase, VoteWeight) => u64;
    IntentionProfsV1 : (VoteWeightBaseV1, VoteWeightV1) => u128;
}

impl<Balance: Default, BlockNumber: Default> From<IntentionProfs<Balance, BlockNumber>>
    for IntentionProfsV1<Balance, BlockNumber>
{
    fn from(iprof: IntentionProfs<Balance, BlockNumber>) -> Self {
        IntentionProfsV1 {
            last_total_vote_weight: u128::from(iprof.last_total_vote_weight),
            last_total_vote_weight_update: iprof.last_total_vote_weight_update,
            total_nomination: iprof.total_nomination,
        }
    }
}

impl<
        Balance: Default + As<u64> + Copy,
        BlockNumber: Default + As<u64> + SimpleArithmetic + Copy,
    > IntentionProfsV1<Balance, BlockNumber>
{
    pub fn settle_latest_vote_weight_safe(&self, current_block: BlockNumber) -> u128 {
        assert!(current_block >= self.last_total_vote_weight_update);
        let duration = current_block - self.last_total_vote_weight_update;
        u128::from(self.total_nomination.as_()) * u128::from(duration.as_())
            + self.last_total_vote_weight
    }
}

/// Declare the struct NominationRecord(V1) and impl VoteWeight(V1) trait accordingly.
///
/// Ref intention_profs! comments.
macro_rules! nomination_record {
    ( $($struct_name:ident : ($weight_base_trait:ident, $weight_trait:ident) => $weight_type:ty;)+ ) => {
        $(
            #[derive(PartialEq, Eq, Clone, Encode, Decode, Default)]
            #[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
            #[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
            pub struct $struct_name<Balance, BlockNumber> {
                pub nomination: Balance,
                pub last_vote_weight: $weight_type,
                pub last_vote_weight_update: BlockNumber,
                pub revocations: Vec<(BlockNumber, Balance)>,
            }

            impl<Balance: Default, BlockNumber: Default> $struct_name<Balance, BlockNumber> {
                pub fn new(nomination: Balance, last_vote_weight: $weight_type, last_vote_weight_update: BlockNumber, revocations: Vec<(BlockNumber, Balance)>) -> Self {
                    Self {
                        nomination,
                        last_vote_weight,
                        last_vote_weight_update,
                        revocations,
                    }
                }
            }

            impl<Balance: Default + As<u64> + Clone, BlockNumber: Default + As<u64> + Clone>
                $weight_base_trait<BlockNumber> for $struct_name<Balance, BlockNumber>
            {
                fn amount(&self) -> u64 {
                    self.nomination.clone().as_()
                }

                fn set_amount(&mut self, new: u64) {
                    self.nomination = Balance::sa(new);
                }

                fn last_acum_weight(&self) -> $weight_type {
                    self.last_vote_weight
                }

                fn set_last_acum_weight(&mut self, latest_vote_weight: $weight_type) {
                    self.last_vote_weight = latest_vote_weight;
                }

                fn last_acum_weight_update(&self) -> u64 {
                    self.last_vote_weight_update.clone().as_()
                }

                fn set_last_acum_weight_update(&mut self, current_block: BlockNumber) {
                    self.last_vote_weight_update = current_block;
                }
            }

            impl<Balance: Default + As<u64> + Clone, BlockNumber: Default + As<u64> + Clone>
                $weight_trait<BlockNumber> for $struct_name<Balance, BlockNumber>
            {
            }
        )+
    };
}

nomination_record! {
    NominationRecord : (VoteWeightBase, VoteWeight) => u64;
    NominationRecordV1 : (VoteWeightBaseV1, VoteWeightV1) => u128;
}

impl<Balance: Default, BlockNumber: Default> From<NominationRecord<Balance, BlockNumber>>
    for NominationRecordV1<Balance, BlockNumber>
{
    fn from(record: NominationRecord<Balance, BlockNumber>) -> Self {
        NominationRecordV1 {
            last_vote_weight: u128::from(record.last_vote_weight),
            last_vote_weight_update: record.last_vote_weight_update,
            nomination: record.nomination,
            revocations: record.revocations,
        }
    }
}

impl<
        Balance: Default + As<u64> + Copy,
        BlockNumber: Default + As<u64> + SimpleArithmetic + Copy,
    > NominationRecordV1<Balance, BlockNumber>
{
    pub fn settle_latest_vote_weight_safe(&self, current_block: BlockNumber) -> u128 {
        assert!(current_block >= self.last_vote_weight_update);
        let duration = current_block - self.last_vote_weight_update;
        u128::from(self.nomination.as_()) * u128::from(duration.as_()) + self.last_vote_weight
    }
}

/// This is useful for RPC exposed via runtime api.
#[derive(PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct IntentionInfoCommon<AccountId, Balance, SessionKey> {
    pub account: AccountId,
    pub name: Option<Vec<u8>>,
    pub session_key: Option<SessionKey>,
    pub jackpot_account: AccountId,
    pub jackpot_balance: Balance,
    pub self_bonded: Balance,
    pub is_validator: bool,
}
