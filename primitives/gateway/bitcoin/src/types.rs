// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use sp_runtime::RuntimeDebug;

use chainx_primitives::ReferralId;

use light_bitcoin::keys::Address;

/// The transaction type with deposit info.
#[doc(hidden)]
#[derive(PartialEq, Eq, Clone, RuntimeDebug)]
pub enum MetaTxType<AccountId> {
    Withdrawal,
    Deposit(DepositInfo<AccountId>),
    HotAndCold,
    TrusteeTransition,
    Irrelevance,
}

/// The info of deposit transaction.
#[derive(PartialEq, Eq, Clone, RuntimeDebug)]
pub struct DepositInfo<AccountId> {
    /// The deposit value.
    pub deposit_value: u64,
    /// The parsed op_return data.
    pub op_return: Option<(AccountId, Option<ReferralId>)>,
    /// The input address of deposit transaction.
    pub input_addr: Option<Address>,
}
