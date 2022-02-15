// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use frame_support::dispatch::DispatchError;
use sp_std::{convert::TryFrom, prelude::Vec};

use crate::types::{ScriptInfo, TrusteeInfoConfig, TrusteeIntentionProps, TrusteeSessionInfo};
use chainx_primitives::ReferralId;
use xp_assets_registrar::Chain;

pub trait BytesLike: Into<Vec<u8>> + TryFrom<Vec<u8>> {}
impl<T: Into<Vec<u8>> + TryFrom<Vec<u8>>> BytesLike for T {}

pub trait ChainProvider {
    fn chain() -> Chain;
}

pub trait TotalSupply<Balance> {
    fn total_supply() -> Balance;
}

pub trait TrusteeForChain<
    AccountId,
    BlockNumber,
    TrusteeEntity: BytesLike,
    TrusteeAddress: BytesLike,
>
{
    fn check_trustee_entity(raw_addr: &[u8]) -> Result<TrusteeEntity, DispatchError>;

    fn generate_trustee_session_info(
        props: Vec<(AccountId, TrusteeIntentionProps<AccountId, TrusteeEntity>)>,
        config: TrusteeInfoConfig,
    ) -> Result<
        (
            TrusteeSessionInfo<AccountId, BlockNumber, TrusteeAddress>,
            ScriptInfo<AccountId>,
        ),
        DispatchError,
    >;
}

pub trait TrusteeSession<AccountId, BlockNumber, TrusteeAddress: BytesLike> {
    fn trustee_session(
        number: u32,
    ) -> Result<TrusteeSessionInfo<AccountId, BlockNumber, TrusteeAddress>, DispatchError>;

    fn current_trustee_session(
    ) -> Result<TrusteeSessionInfo<AccountId, BlockNumber, TrusteeAddress>, DispatchError>;

    fn current_proxy_account() -> Result<Vec<AccountId>, DispatchError>;

    fn last_trustee_session(
    ) -> Result<TrusteeSessionInfo<AccountId, BlockNumber, TrusteeAddress>, DispatchError>;

    fn trustee_transition_state() -> bool;

    #[cfg(feature = "std")]
    fn genesis_trustee(chain: Chain, init: &[AccountId]);
}

impl<AccountId, BlockNumber, TrusteeAddress: BytesLike>
    TrusteeSession<AccountId, BlockNumber, TrusteeAddress> for ()
{
    fn trustee_session(
        _: u32,
    ) -> Result<TrusteeSessionInfo<AccountId, BlockNumber, TrusteeAddress>, DispatchError> {
        Err("NoTrustee".into())
    }

    fn current_trustee_session(
    ) -> Result<TrusteeSessionInfo<AccountId, BlockNumber, TrusteeAddress>, DispatchError> {
        Err("NoTrustee".into())
    }

    fn current_proxy_account() -> Result<Vec<AccountId>, DispatchError> {
        Err("NoTrustee".into())
    }

    fn last_trustee_session(
    ) -> Result<TrusteeSessionInfo<AccountId, BlockNumber, TrusteeAddress>, DispatchError> {
        Err("NoTrustee".into())
    }

    fn trustee_transition_state() -> bool {
        false
    }

    #[cfg(feature = "std")]
    fn genesis_trustee(_: Chain, _: &[AccountId]) {}
}

pub trait TrusteeInfoUpdate {
    /// Update the trustee trasition status when the renewal of the trustee is completed
    fn update_transition_status(status: bool, trans_amount: Option<u64>);
    /// Each withdrawal is completed to record the weight of the signer
    fn update_trustee_sig_record(script: &[u8], withdraw_amout: u64);
}

impl TrusteeInfoUpdate for () {
    fn update_transition_status(_: bool, _: Option<u64>) {}

    fn update_trustee_sig_record(_: &[u8], _: u64) {}
}

pub trait RelayerInfo<AccountId: Default> {
    fn current_relayer() -> AccountId;
}

impl<AccountId: Default> RelayerInfo<AccountId> for () {
    fn current_relayer() -> AccountId {
        Default::default()
    }
}

pub trait ReferralBinding<AccountId, AssetId> {
    fn update_binding(asset_id: &AssetId, who: &AccountId, referral_name: Option<ReferralId>);
    fn referral(asset_id: &AssetId, who: &AccountId) -> Option<AccountId>;
}

impl<AccountId, AssetId> ReferralBinding<AccountId, AssetId> for () {
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
