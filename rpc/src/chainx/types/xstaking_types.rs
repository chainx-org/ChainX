// Copyright 2018-2019 Chainpool.

use super::*;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Revocation {
    pub block_number: BlockNumber,
    pub value: Balance,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NominationRecordForRpc {
    pub nomination: Balance,
    pub last_vote_weight: u64,
    pub last_vote_weight_update: BlockNumber,
    pub revocations: Vec<Revocation>,
}

impl From<NominationRecordWrapper> for NominationRecordForRpc {
    fn from(w: NominationRecordWrapper) -> Self {
        let record: xstaking::NominationRecord<Balance, BlockNumber> =
            w.0.map(Into::into).expect("Ensured it's Ok");
        record.into()
    }
}

#[inline]
fn to_revocation_struct(revocations: Vec<(BlockNumber, Balance)>) -> Vec<Revocation> {
    revocations
        .iter()
        .map(|x| Revocation {
            block_number: x.0,
            value: x.1,
        })
        .collect::<Vec<_>>()
}

impl From<xstaking::NominationRecord<Balance, BlockNumber>> for NominationRecordForRpc {
    fn from(record: xstaking::NominationRecord<Balance, BlockNumber>) -> Self {
        Self {
            nomination: record.nomination,
            last_vote_weight: record.last_vote_weight,
            last_vote_weight_update: record.last_vote_weight_update,
            revocations: to_revocation_struct(record.revocations),
        }
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NominationRecordV1ForRpc {
    pub nomination: Balance,
    pub last_vote_weight: String,
    pub last_vote_weight_update: BlockNumber,
    pub revocations: Vec<Revocation>,
}

impl From<xstaking::NominationRecordV1<Balance, BlockNumber>> for NominationRecordV1ForRpc {
    fn from(record: xstaking::NominationRecordV1<Balance, BlockNumber>) -> Self {
        Self {
            nomination: record.nomination,
            last_vote_weight: format!("{}", record.last_vote_weight),
            last_vote_weight_update: record.last_vote_weight_update,
            revocations: to_revocation_struct(record.revocations),
        }
    }
}

impl From<NominationRecordWrapper> for NominationRecordV1ForRpc {
    fn from(w: NominationRecordWrapper) -> Self {
        let record_v1: xstaking::NominationRecordV1<Balance, BlockNumber> = w.into();
        record_v1.into()
    }
}

impl From<NominationRecordWrapper> for xstaking::NominationRecordV1<Balance, BlockNumber> {
    fn from(w: NominationRecordWrapper) -> Self {
        match w.0 {
            Ok(r) => r.into(),
            Err(r1) => r1,
        }
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct NominationRecordWrapper(
    pub  result::Result<
        xstaking::NominationRecord<Balance, BlockNumber>,
        xstaking::NominationRecordV1<Balance, BlockNumber>,
    >,
);

/// Intention properties
#[derive(Debug, Default, PartialEq, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct IntentionPropsForRpc {
    /// url
    pub url: String,
    /// is running for the validators
    pub is_active: bool,
    /// about
    pub about: String,
    /// session key for block authoring
    pub session_key: AccountIdForRpc,
}

impl IntentionPropsForRpc {
    pub fn new(
        props: xaccounts::IntentionProps<AuthorityId, BlockNumber>,
        intention: AccountId,
    ) -> Self {
        Self {
            url: to_string!(&props.url),
            is_active: props.is_active,
            about: to_string!(&props.about),
            session_key: props.session_key.unwrap_or(intention).into(),
        }
    }
}

#[derive(Debug, Default, PartialEq, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct IntentionInfoCommon {
    #[serde(flatten)]
    pub common: IntentionInfoCommonForRpc,
    #[serde(flatten)]
    pub intention_props: IntentionPropsForRpc,
    /// is trustee
    pub is_trustee: Vec<Chain>,
}

#[derive(Debug, Default, PartialEq, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct IntentionInfoCommonForRpc {
    /// account id of intention
    pub account: AccountIdForRpc,
    /// name of intention
    pub name: String,
    /// is validator
    pub is_validator: bool,
    /// how much has intention voted for itself
    pub self_vote: Balance,
    /// jackpot
    pub jackpot: Balance,
    /// jackpot account
    pub jackpot_account: AccountIdForRpc,
}

impl From<xstaking::IntentionInfoCommon<AccountId, Balance, AuthorityId, BlockNumber>>
    for IntentionInfoCommonForRpc
{
    fn from(
        common: xstaking::IntentionInfoCommon<AccountId, Balance, AuthorityId, BlockNumber>,
    ) -> Self {
        Self {
            account: common.account.clone().into(),
            name: to_string!(&common.name.unwrap_or_default()),
            is_validator: common.is_validator,
            self_vote: common.self_bonded,
            jackpot: common.jackpot_balance,
            jackpot_account: common.jackpot_account.into(),
        }
    }
}

/// Due to the serde inability about u128, we use String instead of u128 here.
#[derive(Debug, Default, PartialEq, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct IntentionProfsV1ForRpc {
    /// total nomination from all nominators
    pub total_nomination: Balance,
    /// vote weight at last update
    pub last_total_vote_weight: String,
    /// last update time of vote weight
    pub last_total_vote_weight_update: BlockNumber,
}

impl From<xstaking::IntentionProfsV1<Balance, BlockNumber>> for IntentionProfsV1ForRpc {
    fn from(iprof_v1: xstaking::IntentionProfsV1<Balance, BlockNumber>) -> Self {
        Self {
            total_nomination: iprof_v1.total_nomination,
            last_total_vote_weight: format!("{}", iprof_v1.last_total_vote_weight),
            last_total_vote_weight_update: iprof_v1.last_total_vote_weight_update,
        }
    }
}

/// Intention info
#[derive(Debug, Default, PartialEq, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct IntentionInfo {
    #[serde(flatten)]
    pub intention_common: IntentionInfoCommon,
    #[serde(flatten)]
    pub intention_profs: xstaking::IntentionProfs<Balance, BlockNumber>,
}

/// Intention info
#[derive(Debug, Default, PartialEq, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct IntentionInfoV1 {
    #[serde(flatten)]
    pub intention_common: IntentionInfoCommon,
    #[serde(flatten)]
    pub intention_profs: IntentionProfsV1ForRpc,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct IntentionInfoWrapper {
    pub intention_common: IntentionInfoCommon,
    pub intention_profs_wrapper: result::Result<
        xstaking::IntentionProfs<Balance, BlockNumber>,
        xstaking::IntentionProfsV1<Balance, BlockNumber>,
    >,
}

impl From<IntentionInfoWrapper> for IntentionInfo {
    fn from(info_wrapper: IntentionInfoWrapper) -> Self {
        Self {
            intention_common: info_wrapper.intention_common,
            intention_profs: info_wrapper
                .intention_profs_wrapper
                .expect("Ensured it's Ok"),
        }
    }
}

impl From<IntentionInfoWrapper> for IntentionInfoV1 {
    fn from(info_wrapper: IntentionInfoWrapper) -> Self {
        Self {
            intention_common: info_wrapper.intention_common,
            intention_profs: match info_wrapper.intention_profs_wrapper {
                Ok(x) => {
                    let x: xstaking::IntentionProfsV1<Balance, BlockNumber> = x.into();
                    x.into()
                }
                Err(x) => x.into(),
            },
        }
    }
}

impl From<IntentionInfoWrapper> for xstaking::IntentionProfsV1<Balance, BlockNumber> {
    fn from(info_wrapper: IntentionInfoWrapper) -> Self {
        match info_wrapper.intention_profs_wrapper {
            Ok(x) => x.into(),
            Err(x) => x,
        }
    }
}
