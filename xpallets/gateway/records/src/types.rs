// Copyright 2018-2019 Chainpool.

use codec::{Codec, Decode, Encode};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

// Substrate
use sp_runtime::RuntimeDebug;

use chainx_primitives::{AddrStr, AssetId};
use xp_runtime::Memo;

/// state machine for state is:
/// Applying(lock token) => Processing(can't cancel) =>
///        destroy token => NormalFinish|RootFinish (final state)
///        release token => NormalCancel(can from Applying directly)|RootCancel (final state)
#[derive(PartialEq, Eq, Clone, Copy, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum WithdrawalState {
    Applying,
    Processing,
    NormalFinish,
    RootFinish,
    NormalCancel,
    RootCancel,
}

impl Default for WithdrawalState {
    fn default() -> Self {
        WithdrawalState::Applying
    }
}

/// WithdrawalRecord for withdrawal
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct WithdrawalRecord<AccountId, Balance, BlockNumber> {
    asset_id: AssetId,
    applicant: AccountId,
    balance: Balance,
    addr: AddrStr,
    ext: Memo,
    height: BlockNumber,
}

impl<AccountId, Balance, BlockNumber> WithdrawalRecord<AccountId, Balance, BlockNumber>
where
    AccountId: Codec + Clone,
    Balance: Codec + Copy + Clone,
    BlockNumber: Codec + Clone,
{
    pub fn new(
        applicant: AccountId,
        asset_id: AssetId,
        balance: Balance,
        addr: AddrStr,
        ext: Memo,
        height: BlockNumber,
    ) -> Self {
        Self {
            asset_id,
            applicant,
            balance,
            addr,
            ext,
            height,
        }
    }
    pub fn applicant(&self) -> &AccountId {
        &self.applicant
    }
    pub fn asset_id(&self) -> AssetId {
        self.asset_id
    }
    pub fn balance(&self) -> Balance {
        self.balance
    }
    pub fn addr(&self) -> &AddrStr {
        &self.addr
    }
    pub fn ext(&self) -> &Memo {
        &self.ext
    }
    pub fn height(&self) -> &BlockNumber {
        &self.height
    }
}

#[derive(PartialEq, Eq, Clone, Encode, Decode)]
pub struct Withdrawal<AccountId, Balance, BlockNumber> {
    pub asset_id: AssetId,
    pub applicant: AccountId,
    pub balance: Balance,
    pub addr: AddrStr,
    pub ext: Memo,
    pub height: BlockNumber,
    pub state: WithdrawalState,
}

impl<AccountId, Balance, BlockNumber> Withdrawal<AccountId, Balance, BlockNumber> {
    pub fn new(
        record: WithdrawalRecord<AccountId, Balance, BlockNumber>,
        state: WithdrawalState,
    ) -> Self {
        Self {
            asset_id: record.asset_id,
            applicant: record.applicant,
            balance: record.balance,
            addr: record.addr,
            ext: record.ext,
            height: record.height,
            state,
        }
    }
}
