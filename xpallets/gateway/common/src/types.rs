// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use codec::{Decode, Encode};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

use sp_runtime::RuntimeDebug;
use sp_std::{convert::TryFrom, prelude::Vec};

use chainx_primitives::Text;

use crate::traits::BytesLike;

/// The config of trustee info.
#[derive(PartialEq, Clone, Encode, Decode, Default, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct TrusteeInfoConfig {
    pub min_trustee_count: u32,
    pub max_trustee_count: u32,
}

/// The trustee session info.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct TrusteeSessionInfo<AccountId, TrusteeAddress: BytesLike> {
    pub trustee_list: Vec<AccountId>,
    pub threshold: u16,
    pub hot_address: TrusteeAddress,
    pub cold_address: TrusteeAddress,
}

/// The generic trustee session info.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct GenericTrusteeSessionInfo<AccountId>(pub TrusteeSessionInfo<AccountId, Vec<u8>>);

impl<AccountId, TrusteeAddress: BytesLike> From<TrusteeSessionInfo<AccountId, TrusteeAddress>>
    for GenericTrusteeSessionInfo<AccountId>
{
    fn from(info: TrusteeSessionInfo<AccountId, TrusteeAddress>) -> Self {
        GenericTrusteeSessionInfo(TrusteeSessionInfo {
            trustee_list: info.trustee_list,
            threshold: info.threshold,
            hot_address: info.hot_address.into(),
            cold_address: info.cold_address.into(),
        })
    }
}

impl<AccountId, TrusteeAddress: BytesLike> TryFrom<GenericTrusteeSessionInfo<AccountId>>
    for TrusteeSessionInfo<AccountId, TrusteeAddress>
{
    // TODO, may use a better error
    type Error = ();

    fn try_from(info: GenericTrusteeSessionInfo<AccountId>) -> Result<Self, Self::Error> {
        Ok(TrusteeSessionInfo::<AccountId, TrusteeAddress> {
            trustee_list: info.0.trustee_list,
            threshold: info.0.threshold,
            hot_address: TrusteeAddress::try_from(info.0.hot_address).map_err(|_| ())?,
            cold_address: TrusteeAddress::try_from(info.0.cold_address).map_err(|_| ())?,
        })
    }
}

/// The trustee intention properties.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct TrusteeIntentionProps<TrusteeEntity: BytesLike> {
    #[cfg_attr(feature = "std", serde(with = "xp_rpc::serde_text"))]
    pub about: Text,
    pub hot_entity: TrusteeEntity,
    pub cold_entity: TrusteeEntity,
}

/// The generic trustee intention properties.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct GenericTrusteeIntentionProps(pub TrusteeIntentionProps<Vec<u8>>);

impl<TrusteeEntity: BytesLike> From<TrusteeIntentionProps<TrusteeEntity>>
    for GenericTrusteeIntentionProps
{
    fn from(props: TrusteeIntentionProps<TrusteeEntity>) -> Self {
        GenericTrusteeIntentionProps(TrusteeIntentionProps {
            about: props.about,
            hot_entity: props.hot_entity.into(),
            cold_entity: props.cold_entity.into(),
        })
    }
}

impl<TrusteeEntity: BytesLike> TryFrom<GenericTrusteeIntentionProps>
    for TrusteeIntentionProps<TrusteeEntity>
{
    // TODO, may use a better error
    type Error = ();

    fn try_from(value: GenericTrusteeIntentionProps) -> Result<Self, Self::Error> {
        Ok(TrusteeIntentionProps::<TrusteeEntity> {
            about: value.0.about,
            hot_entity: TrusteeEntity::try_from(value.0.hot_entity).map_err(|_| ())?,
            cold_entity: TrusteeEntity::try_from(value.0.cold_entity).map_err(|_| ())?,
        })
    }
}
