// Copyright 2018-2019 Chainpool.

use frame_support::dispatch::DispatchError;
use sp_std::{convert::TryFrom, fmt::Debug, prelude::Vec, result};

use chainx_primitives::{AssetId, Name};
use xpallet_support::error;

use crate::types::{TrusteeInfoConfig, TrusteeIntentionProps, TrusteeSessionInfo};

pub trait Extractable<AccountId> {
    fn account_info(data: &[u8]) -> Option<(AccountId, Option<Name>)>;
}

pub trait BytesLike: Into<Vec<u8>> + TryFrom<Vec<u8>> {}

impl<T: Into<Vec<u8>> + TryFrom<Vec<u8>>> BytesLike for T {}

pub trait TrusteeForChain<AccountId, TrusteeEntity: BytesLike, TrusteeAddress: BytesLike> {
    fn check_trustee_entity(raw_addr: &[u8]) -> result::Result<TrusteeEntity, DispatchError>;

    fn generate_trustee_session_info(
        props: Vec<(AccountId, TrusteeIntentionProps<TrusteeEntity>)>,
        config: TrusteeInfoConfig,
    ) -> result::Result<TrusteeSessionInfo<AccountId, TrusteeAddress>, DispatchError>;
}

pub trait TrusteeSession<AccountId, TrusteeAddress: BytesLike> {
    fn trustee_session(
        number: u32,
    ) -> result::Result<TrusteeSessionInfo<AccountId, TrusteeAddress>, DispatchError>;

    fn current_trustee_session(
    ) -> result::Result<TrusteeSessionInfo<AccountId, TrusteeAddress>, DispatchError>;

    fn last_trustee_session(
    ) -> result::Result<TrusteeSessionInfo<AccountId, TrusteeAddress>, DispatchError>;
}

pub trait TrusteeMultiSig<AccountId: PartialEq + Debug> {
    fn multisig_for_trustees() -> AccountId;

    fn check_multisig(who: &AccountId) -> bool {
        let current_multisig_addr = Self::multisig_for_trustees();
        if current_multisig_addr != *who {
            error!("[check_multisig]|the account not match current trustee multisig addr for this chain|current:{:?}|who:{:?}", current_multisig_addr, who);
            false
        } else {
            true
        }
    }
}

pub trait ChannelBinding<AccountId> {
    fn update_binding(asset_id: &AssetId, who: &AccountId, channel_name: Option<Name>);
    fn get_binding_info(asset_id: &AssetId, who: &AccountId) -> Option<AccountId>;
}
