// Copyright 2018-2019 Chainpool.

use sp_runtime::RuntimeDebug;
use sp_std::{convert::TryFrom, prelude::Vec};

use codec::{Decode, Encode};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

// ChainX
use chainx_primitives::Text;

use crate::traits::BytesLike;
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
pub struct TrusteeIntentionProps<TrusteeEntity: BytesLike> {
    pub about: Text,
    pub hot_entity: TrusteeEntity,
    pub cold_entity: TrusteeEntity,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct TrusteeSessionInfo<AccountId, TrusteeAddress: BytesLike> {
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

impl<TrusteeEntity: BytesLike> Into<GenericTrusteeIntentionProps>
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

impl<TrusteeEntity: BytesLike> TryFrom<GenericTrusteeIntentionProps>
    for TrusteeIntentionProps<TrusteeEntity>
{
    // TODO, may use a better error
    type Error = ();

    fn try_from(value: GenericTrusteeIntentionProps) -> Result<Self, Self::Error> {
        let hot = TrusteeEntity::try_from(value.0.hot_entity).map_err(|_| ())?;
        let cold = TrusteeEntity::try_from(value.0.cold_entity).map_err(|_| ())?;
        Ok(TrusteeIntentionProps::<TrusteeEntity> {
            about: value.0.about,
            hot_entity: hot,
            cold_entity: cold,
        })
    }
}

impl<AccountId, TrusteeAddress: BytesLike> Into<GenericTrusteeSessionInfo<AccountId>>
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

impl<AccountId, TrusteeAddress: BytesLike> TryFrom<GenericTrusteeSessionInfo<AccountId>>
    for TrusteeSessionInfo<AccountId, TrusteeAddress>
{
    // TODO, may use a better error
    type Error = ();

    fn try_from(value: GenericTrusteeSessionInfo<AccountId>) -> Result<Self, Self::Error> {
        let hot = TrusteeAddress::try_from(value.0.hot_address).map_err(|_| ())?;
        let cold = TrusteeAddress::try_from(value.0.cold_address).map_err(|_| ())?;
        Ok(TrusteeSessionInfo::<AccountId, TrusteeAddress> {
            trustee_list: value.0.trustee_list,
            hot_address: hot,
            cold_address: cold,
        })
    }
}

pub fn into_generic_all_info<
    AccountId: Clone,
    TrusteeEntity: BytesLike,
    TrusteeAddress: BytesLike,
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
