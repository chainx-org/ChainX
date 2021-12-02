// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use frame_support::dispatch::DispatchError;
use sp_std::{convert::TryFrom, prelude::Vec};

use chainx_primitives::{AssetId, ReferralId};
use xpallet_assets::Chain;

use crate::types::{ScriptInfo, TrusteeInfoConfig, TrusteeIntentionProps, TrusteeSessionInfo};

pub trait BytesLike: Into<Vec<u8>> + TryFrom<Vec<u8>> {}
impl<T: Into<Vec<u8>> + TryFrom<Vec<u8>>> BytesLike for T {}

pub trait ChainProvider {
    fn chain() -> Chain;
}

pub trait TrusteeForChain<AccountId, TrusteeEntity: BytesLike, TrusteeAddress: BytesLike> {
    fn check_trustee_entity(raw_addr: &[u8]) -> Result<TrusteeEntity, DispatchError>;

    fn generate_trustee_session_info(
        props: Vec<(AccountId, TrusteeIntentionProps<AccountId, TrusteeEntity>)>,
        config: TrusteeInfoConfig,
    ) -> Result<
        (
            TrusteeSessionInfo<AccountId, TrusteeAddress>,
            ScriptInfo<AccountId>,
        ),
        DispatchError,
    >;
}

pub trait TrusteeSession<AccountId, TrusteeAddress: BytesLike> {
    fn trustee_session(
        number: u32,
    ) -> Result<TrusteeSessionInfo<AccountId, TrusteeAddress>, DispatchError>;

    fn current_trustee_session(
    ) -> Result<TrusteeSessionInfo<AccountId, TrusteeAddress>, DispatchError>;

    fn last_trustee_session() -> Result<TrusteeSessionInfo<AccountId, TrusteeAddress>, DispatchError>;

    #[cfg(feature = "std")]
    fn genesis_trustee(chain: Chain, init: &[AccountId]);
}

impl<AccountId, TrusteeAddress: BytesLike> TrusteeSession<AccountId, TrusteeAddress> for () {
    fn trustee_session(
        _: u32,
    ) -> Result<TrusteeSessionInfo<AccountId, TrusteeAddress>, DispatchError> {
        Err("NoTrustee".into())
    }

    fn current_trustee_session(
    ) -> Result<TrusteeSessionInfo<AccountId, TrusteeAddress>, DispatchError> {
        Err("NoTrustee".into())
    }

    fn last_trustee_session() -> Result<TrusteeSessionInfo<AccountId, TrusteeAddress>, DispatchError>
    {
        Err("NoTrustee".into())
    }

    #[cfg(feature = "std")]
    fn genesis_trustee(_: Chain, _: &[AccountId]) {}
}

pub trait ReferralBinding<AccountId> {
    fn update_binding(asset_id: &AssetId, who: &AccountId, referral_name: Option<ReferralId>);
    fn referral(asset_id: &AssetId, who: &AccountId) -> Option<AccountId>;
}

impl<AccountId> ReferralBinding<AccountId> for () {
    fn update_binding(_: &AssetId, _: &AccountId, _: Option<ReferralId>) {}
    fn referral(_: &AssetId, _: &AccountId) -> Option<AccountId> {
        None
    }
}

pub trait AddressBinding<AccountId, Address: Into<Vec<u8>>> {
    fn update_binding(chain: Chain, address: Address, who: AccountId);
    fn address(chain: Chain, address: Address) -> Option<AccountId>;
}

impl<AccountId, Address: Into<Vec<u8>>> AddressBinding<AccountId, Address> for () {
    fn update_binding(_: Chain, _: Address, _: AccountId) {}
    fn address(_: Chain, _: Address) -> Option<AccountId> {
        None
    }
}
