// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use crate::{
    traits::BytesLike, Config, GenericTrusteeIntentionProps, GenericTrusteeSessionInfo,
    TrusteeIntentionPropertiesOf, TrusteeIntentionProps, TrusteeSessionInfo, TrusteeSessionInfoLen,
    TrusteeSessionInfoOf,
};
use chainx_primitives::Text;
use codec::{Decode, Encode};
use frame_support::{log::info, traits::Get, weights::Weight, RuntimeDebug};
use scale_info::TypeInfo;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_std::prelude::*;
use xp_assets_registrar::Chain;

/// The trustee session info.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
struct OldTrusteeSessionInfo<AccountId, TrusteeAddress: BytesLike> {
    /// Trustee account
    pub trustee_list: Vec<AccountId>,
    /// Threshold value
    pub threshold: u16,
    /// Hot address
    pub hot_address: TrusteeAddress,
    /// Cold address
    pub cold_address: TrusteeAddress,
}

/// The generic trustee session info.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
struct OldGenericTrusteeSessionInfo<AccountId>(pub OldTrusteeSessionInfo<AccountId, Vec<u8>>);

/// The trustee intention properties.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct OldTrusteeIntentionProps<TrusteeEntity: BytesLike> {
    #[cfg_attr(feature = "std", serde(with = "xp_rpc::serde_text"))]
    pub about: Text,
    pub hot_entity: TrusteeEntity,
    pub cold_entity: TrusteeEntity,
}
/// The generic trustee intention properties.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct OldGenericTrusteeIntentionProps(pub OldTrusteeIntentionProps<Vec<u8>>);

/// Apply all of the migrations due to taproot.
///
/// ### Warning
///
/// Use with care and run at your own risk.
pub fn apply<T: Config>() -> Weight {
    info!(
        target: "runtime::gateway::common",
        "Running migration for gateway common pallet"
    );

    migrate_trustee_session_info::<T>().saturating_add(migrate_trustee_intention_properties::<T>())
}

/// Migrate from the old trustee session info.
pub fn migrate_trustee_session_info<T: Config>() -> Weight {
    TrusteeSessionInfoLen::<T>::mutate(Chain::Bitcoin, |l| *l = l.saturating_sub(1));
    TrusteeSessionInfoOf::<T>::translate::<OldGenericTrusteeSessionInfo<T::AccountId>, _>(
        |_, _, trustee_info| {
            Some(GenericTrusteeSessionInfo(TrusteeSessionInfo {
                trustee_list: trustee_info
                    .0
                    .trustee_list
                    .iter()
                    .map(|n| (n.clone(), 0))
                    .collect::<Vec<_>>(),
                threshold: trustee_info.0.threshold,
                hot_address: trustee_info.0.hot_address,
                cold_address: trustee_info.0.cold_address,
                multi_account: None,
                start_height: None,
                end_height: None,
            }))
        },
    );
    let count = TrusteeSessionInfoOf::<T>::iter_values().count();
    info!(
        target: "runtime::gateway::common",
        "migrated {} trustee session infos.",
        count,
    );
    <T as frame_system::Config>::DbWeight::get()
        .reads_writes(count as Weight + 1, count as Weight + 1)
}

/// Migrate from the old trustee intention properties.
pub fn migrate_trustee_intention_properties<T: Config>() -> Weight {
    TrusteeIntentionPropertiesOf::<T>::translate::<OldGenericTrusteeIntentionProps, _>(
        |_, _, props| {
            Some(GenericTrusteeIntentionProps(TrusteeIntentionProps {
                proxy_account: None,
                about: props.0.about,
                hot_entity: props.0.hot_entity,
                cold_entity: props.0.cold_entity,
            }))
        },
    );
    let count = TrusteeIntentionPropertiesOf::<T>::iter_values().count();
    info!(
        target: "runtime::gateway::common",
        "migrated {} trustee_intention_properties.",
        count,
    );
    <T as frame_system::Config>::DbWeight::get()
        .reads_writes(count as Weight + 1, count as Weight + 1)
}
