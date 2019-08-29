// Copyright 2018-2019 Chainpool.

use super::*;

#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PseduIntentionInfo {
    #[serde(flatten)]
    pub psedu_intention_common: PseduIntentionInfoCommon,
    #[serde(flatten)]
    pub psedu_intention_profs: xtokens::PseduIntentionVoteWeight<Balance>,
}

#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PseduIntentionInfoV1 {
    #[serde(flatten)]
    pub psedu_intention_common: PseduIntentionInfoCommon,
    #[serde(flatten)]
    pub psedu_intention_profs: PseduIntentionVoteWeightV1ForRpc,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PseduIntentionInfoWrapper {
    pub psedu_intention_common: PseduIntentionInfoCommon,
    pub psedu_intention_profs_wrapper: result::Result<
        xtokens::PseduIntentionVoteWeight<Balance>,
        xtokens::PseduIntentionVoteWeightV1<Balance>,
    >,
}

impl From<PseduIntentionInfoWrapper> for PseduIntentionInfo {
    fn from(info_wrapper: PseduIntentionInfoWrapper) -> Self {
        Self {
            psedu_intention_common: info_wrapper.psedu_intention_common,
            psedu_intention_profs: info_wrapper
                .psedu_intention_profs_wrapper
                .expect("Ensured it's Ok"),
        }
    }
}

impl From<PseduIntentionInfoWrapper> for PseduIntentionInfoV1 {
    fn from(info_wrapper: PseduIntentionInfoWrapper) -> Self {
        Self {
            psedu_intention_common: info_wrapper.psedu_intention_common,
            psedu_intention_profs: match info_wrapper.psedu_intention_profs_wrapper {
                Ok(x) => {
                    let x: xtokens::PseduIntentionVoteWeightV1<Balance> = x.into();
                    x.into()
                }
                Err(x) => x.into(),
            },
        }
    }
}

impl From<PseduIntentionInfoWrapper> for xtokens::PseduIntentionVoteWeightV1<Balance> {
    fn from(info_wrapper: PseduIntentionInfoWrapper) -> Self {
        match info_wrapper.psedu_intention_profs_wrapper {
            Ok(x) => x.into(),
            Err(x) => x,
        }
    }
}

#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PseduIntentionVoteWeightV1ForRpc {
    /// vote weight at last update
    pub last_total_deposit_weight: String,
    /// last update time of vote weight
    pub last_total_deposit_weight_update: BlockNumber,
}

impl From<xtokens::PseduIntentionVoteWeightV1<Balance>> for PseduIntentionVoteWeightV1ForRpc {
    fn from(d1: PseduIntentionVoteWeightV1<Balance>) -> Self {
        Self {
            last_total_deposit_weight: format!("{}", d1.last_total_deposit_weight),
            last_total_deposit_weight_update: d1.last_total_deposit_weight_update,
        }
    }
}

#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PseduNominationRecordCommon {
    /// name of intention
    pub id: String,
    /// total deposit
    pub balance: Balance,
    pub next_claim: BlockNumber,
}

#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PseduNominationRecord {
    #[serde(flatten)]
    pub common: PseduNominationRecordCommon,
    /// vote weight at last update
    pub last_total_deposit_weight: u64,
    /// last update time of vote weight
    pub last_total_deposit_weight_update: BlockNumber,
}

#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PseduNominationRecordV1 {
    #[serde(flatten)]
    pub common: PseduNominationRecordCommon,
    /// vote weight at last update
    pub last_total_deposit_weight: String,
    /// last update time of vote weight
    pub last_total_deposit_weight_update: BlockNumber,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct PseduNominationRecordWrapper {
    pub common: PseduNominationRecordCommon,
    pub deposit_vote_weight_wrapper: result::Result<
        xtokens::DepositVoteWeight<BlockNumber>,
        xtokens::DepositVoteWeightV1<BlockNumber>,
    >,
}

impl From<PseduNominationRecordWrapper> for PseduNominationRecord {
    fn from(record_wrapper: PseduNominationRecordWrapper) -> Self {
        let record: xtokens::DepositVoteWeight<Balance> = record_wrapper
            .deposit_vote_weight_wrapper
            .expect("Ensured it's Ok");
        Self {
            common: record_wrapper.common,
            last_total_deposit_weight: record.last_deposit_weight,
            last_total_deposit_weight_update: record.last_deposit_weight_update,
        }
    }
}

impl From<PseduNominationRecordWrapper> for PseduNominationRecordV1 {
    fn from(record_wrapper: PseduNominationRecordWrapper) -> Self {
        let record_v1: xtokens::DepositVoteWeightV1<Balance> =
            match record_wrapper.deposit_vote_weight_wrapper {
                Ok(r) => r.into(),
                Err(r1) => r1,
            };
        Self {
            common: record_wrapper.common,
            last_total_deposit_weight: format!("{}", record_v1.last_deposit_weight),
            last_total_deposit_weight_update: record_v1.last_deposit_weight_update,
        }
    }
}

impl From<PseduNominationRecordWrapper> for xtokens::DepositVoteWeightV1<Balance> {
    fn from(record_wrapper: PseduNominationRecordWrapper) -> Self {
        match record_wrapper.deposit_vote_weight_wrapper {
            Ok(r) => r.into(),
            Err(r1) => r1,
        }
    }
}

#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PseduIntentionInfoCommon {
    /// name of intention
    pub id: String,
    /// circulation of id
    pub circulation: Balance,
    pub price: Balance,
    pub discount: u32,
    pub power: Balance,
    /// jackpot
    pub jackpot: Balance,
    /// jackpot account
    pub jackpot_account: AccountIdForRpc,
}
