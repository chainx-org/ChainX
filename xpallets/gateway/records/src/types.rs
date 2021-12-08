// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use codec::{Codec, Decode, Encode};
use scale_info::TypeInfo;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

use sp_runtime::RuntimeDebug;

use chainx_primitives::AddrStr;
use xp_runtime::Memo;

/// The id of withdrawal record (u32 is enough).
pub type WithdrawalRecordId = u32;

/// The state machine of WithdrawState:
///
/// Applying (lock token) <---> Processing (can't cancel, but can be recovered to `Applying`)
///     |                           |
///     |                           +----> NormalFinish|RootFinish (destroy token)
///     |                           |
///     |                           +----> RootCancel (unlock token)
///     |                           |
///     +---------------------------+----> NormalCancel (unlock token)
///
#[derive(PartialEq, Eq, Clone, Copy, Encode, Decode, RuntimeDebug, TypeInfo)]
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
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct WithdrawalRecord<AccountId, AssetId, Balance, BlockNumber> {
    asset_id: AssetId,
    applicant: AccountId,
    balance: Balance,
    addr: AddrStr,
    ext: Memo,
    height: BlockNumber,
}

impl<AccountId, AssetId, Balance, BlockNumber>
    WithdrawalRecord<AccountId, AssetId, Balance, BlockNumber>
where
    AccountId: Codec + Clone,
    AssetId: Codec + Copy + Clone,
    Balance: Codec + Copy + Clone,
    BlockNumber: Codec + Copy + Clone,
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
    pub fn height(&self) -> BlockNumber {
        self.height
    }
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, TypeInfo)]
pub struct Withdrawal<AccountId, AssetId, Balance, BlockNumber> {
    pub asset_id: AssetId,
    pub applicant: AccountId,
    pub balance: Balance,
    pub addr: AddrStr,
    pub ext: Memo,
    pub height: BlockNumber,
    pub state: WithdrawalState,
}

impl<AccountId, AssetId, Balance, BlockNumber>
    Withdrawal<AccountId, AssetId, Balance, BlockNumber>
{
    pub fn new(
        record: WithdrawalRecord<AccountId, AssetId, Balance, BlockNumber>,
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

#[derive(PartialEq, Eq, Clone, Default, Encode, Decode, RuntimeDebug, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct WithdrawalLimit<Balance> {
    pub minimal_withdrawal: Balance,
    pub fee: Balance,
}
