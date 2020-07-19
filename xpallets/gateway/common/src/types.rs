// Copyright 2018-2019 Chainpool.

use sp_runtime::RuntimeDebug;
use sp_std::prelude::Vec;

use codec::{Decode, Encode};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

// ChainX
use chainx_primitives::Text;

use crate::utils::two_thirds_unsafe;

#[derive(PartialEq, Clone, Encode, Decode, Default, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct TrusteeInfoConfig {
    pub min_trustee_count: u32,
    pub max_trustee_count: u32,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct TrusteeIntentionProps<TrusteeEntity: Into<Vec<u8>>> {
    pub about: Text,
    pub hot_entity: TrusteeEntity,
    pub cold_entity: TrusteeEntity,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct TrusteeSessionInfo<AccountId, TrusteeAddress: Into<Vec<u8>>> {
    pub trustee_list: Vec<AccountId>,
    pub hot_address: TrusteeAddress,
    pub cold_address: TrusteeAddress,
}

// generic
#[derive(PartialEq, Clone, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct GenericTrusteeIntentionProps(pub TrusteeIntentionProps<Vec<u8>>);

#[derive(PartialEq, Clone, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct GenericTrusteeSessionInfo<AccountId>(pub TrusteeSessionInfo<AccountId, Vec<u8>>);

#[derive(PartialEq, Clone, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct GenericAllSessionInfo<AccountId> {
    pub hot_entity: Vec<u8>,
    pub cold_entity: Vec<u8>,
    pub counts: Counts,
    pub trustees_info: Vec<(AccountId, GenericTrusteeIntentionProps)>,
}

#[derive(PartialEq, Clone, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct Counts {
    pub required: u32,
    pub total: u32,
}

impl<TrusteeEntity: Into<Vec<u8>>> Into<GenericTrusteeIntentionProps>
    for TrusteeIntentionProps<TrusteeEntity>
{
    fn into(self) -> GenericTrusteeIntentionProps {
        GenericTrusteeIntentionProps(TrusteeIntentionProps {
            about: self.about,
            hot_entity: self.hot_entity.into(),
            cold_entity: self.cold_entity.into(),
        })
    }
}

impl<AccountId, TrusteeAddress: Into<Vec<u8>>> Into<GenericTrusteeSessionInfo<AccountId>>
    for TrusteeSessionInfo<AccountId, TrusteeAddress>
{
    fn into(self) -> GenericTrusteeSessionInfo<AccountId> {
        GenericTrusteeSessionInfo(TrusteeSessionInfo {
            trustee_list: self.trustee_list,
            hot_address: self.hot_address.into(),
            cold_address: self.cold_address.into(),
        })
    }
}

pub fn into_generic_all_info<
    AccountId: Clone,
    TrusteeEntity: Into<Vec<u8>>,
    TrusteeAddress: Into<Vec<u8>>,
    F,
>(
    session_info: TrusteeSessionInfo<AccountId, TrusteeAddress>,
    get_props: F,
) -> GenericAllSessionInfo<AccountId>
where
    F: Fn(&AccountId) -> Option<TrusteeIntentionProps<TrusteeEntity>>,
{
    let session_info: GenericTrusteeSessionInfo<AccountId> = session_info.into();

    let total = session_info.0.trustee_list.len() as u32;
    let required = two_thirds_unsafe(total);

    let mut trustees_info: Vec<(AccountId, GenericTrusteeIntentionProps)> = Vec::new();
    for accountid in session_info.0.trustee_list {
        if let Some(props) = get_props(&accountid) {
            trustees_info.push((accountid, props.into()))
        }
    }

    GenericAllSessionInfo {
        hot_entity: session_info.0.hot_address,
        cold_entity: session_info.0.cold_address,
        counts: Counts { required, total },
        trustees_info,
    }
}
