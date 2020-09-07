// Copyright 2018-2019 Chainpool.

use frame_support::dispatch::DispatchError;
use sp_std::{convert::TryFrom, prelude::Vec, result};

use chainx_primitives::{AssetId, ReferralId};
use xpallet_assets::Chain;

use crate::types::{TrusteeInfoConfig, TrusteeIntentionProps, TrusteeSessionInfo};

/// Trait for extracting the deposit info from op_return.
pub trait Extractable<AccountId> {
    /// Returns the target deposit account and possible referral id.
    fn account_info(data: &[u8]) -> Option<(AccountId, Option<ReferralId>)>;
}

impl<AccountId> Extractable<AccountId> for () {
    fn account_info(_data: &[u8]) -> Option<(AccountId, Option<ReferralId>)> {
        None
    }
}

pub trait BytesLike: Into<Vec<u8>> + TryFrom<Vec<u8>> {}

impl<T: Into<Vec<u8>> + TryFrom<Vec<u8>>> BytesLike for T {}

pub trait ChainProvider {
    fn chain() -> Chain;
}

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

impl<AccountId, TrusteeAddress: BytesLike> TrusteeSession<AccountId, TrusteeAddress> for () {
    fn trustee_session(
        _: u32,
    ) -> result::Result<TrusteeSessionInfo<AccountId, TrusteeAddress>, DispatchError> {
        Err("NoTrustee".into())
    }

    fn current_trustee_session(
    ) -> result::Result<TrusteeSessionInfo<AccountId, TrusteeAddress>, DispatchError> {
        Err("NoTrustee".into())
    }

    fn last_trustee_session(
    ) -> result::Result<TrusteeSessionInfo<AccountId, TrusteeAddress>, DispatchError> {
        Err("NoTrustee".into())
    }
}

pub trait ChannelBinding<AccountId> {
    fn update_binding(asset_id: &AssetId, who: &AccountId, channel_name: Option<ReferralId>);
    fn get_binding_info(asset_id: &AssetId, who: &AccountId) -> Option<AccountId>;
}

impl<AccountId> ChannelBinding<AccountId> for () {
    fn update_binding(_: &AssetId, _: &AccountId, _: Option<ReferralId>) {}
    fn get_binding_info(_: &AssetId, _: &AccountId) -> Option<AccountId> {
        None
    }
}

pub trait AddrBinding<AccountId, Addr: Into<Vec<u8>>> {
    fn update_binding(chain: Chain, addr: Addr, who: AccountId);
    fn get_binding(chain: Chain, addr: Addr) -> Option<AccountId>;
}

impl<AccountId, Addr: Into<Vec<u8>>> AddrBinding<AccountId, Addr> for () {
    fn update_binding(_: Chain, _: Addr, _: AccountId) {}
    fn get_binding(_: Chain, _: Addr) -> Option<AccountId> {
        None
    }
}
