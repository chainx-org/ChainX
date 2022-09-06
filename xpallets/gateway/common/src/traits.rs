// Copyright 2019-2022 ChainX Project Authors. Licensed under GPL-3.0.

use frame_support::dispatch::DispatchError;
use sp_std::{convert::TryFrom, prelude::Vec};

use chainx_primitives::{AssetId, ReferralId};
use xp_gateway_common::DstChain;
use xpallet_assets::Chain;

use crate::types::{ScriptInfo, TrusteeInfoConfig, TrusteeIntentionProps, TrusteeSessionInfo};
use xp_gateway_bitcoin::OpReturnAccount;

pub trait BytesLike: Into<Vec<u8>> + TryFrom<Vec<u8>> {}
impl<T: Into<Vec<u8>> + TryFrom<Vec<u8>>> BytesLike for T {}

pub trait ChainProvider {
    fn chain() -> Chain;
}

pub trait ProposalProvider {
    type WithdrawalProposal;

    fn get_withdrawal_proposal() -> Option<Self::WithdrawalProposal>;
}

impl ProposalProvider for () {
    type WithdrawalProposal = ();

    fn get_withdrawal_proposal() -> Option<Self::WithdrawalProposal> {
        None
    }
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
    fn update_transition_status(chain: Chain, status: bool, trans_amount: Option<u64>);
    /// Each withdrawal is completed to record the weight of the signer
    fn update_trustee_sig_record(chain: Chain, script: &[u8], withdraw_amout: u64);
}

impl TrusteeInfoUpdate for () {
    fn update_transition_status(_: Chain, _: bool, _: Option<u64>) {}

    fn update_trustee_sig_record(_: Chain, _: &[u8], _: u64) {}
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
    fn update_binding(chain: Chain, address: Address, who: OpReturnAccount<AccountId>);
    fn address(chain: Chain, address: Address) -> Option<OpReturnAccount<AccountId>>;
}

impl<AccountId, Address: Into<Vec<u8>>> AddressBinding<AccountId, Address> for () {
    fn update_binding(_: Chain, _: Address, _: OpReturnAccount<AccountId>) {}
    fn address(_: Chain, _: Address) -> Option<OpReturnAccount<AccountId>> {
        None
    }
}
