// Copyright 2019-2022 ChainX Project Authors. Licensed under GPL-3.0.

pub trait MultisigAddressFor<AccountId> {
    fn calc_multisig(accounts: &[AccountId], threshold: u16) -> AccountId;
}

impl<AccountId: Default> MultisigAddressFor<AccountId> for () {
    fn calc_multisig(_: &[AccountId], _: u16) -> AccountId {
        Default::default()
    }
}

pub trait MultiSig<AccountId: PartialEq> {
    fn multisig() -> AccountId;
}

pub trait Validator<AccountId> {
    fn is_validator(who: &AccountId) -> bool;

    fn validator_for(name: &[u8]) -> Option<AccountId>;
}

impl<AccountId> Validator<AccountId> for () {
    fn is_validator(_: &AccountId) -> bool {
        false
    }

    fn validator_for(_: &[u8]) -> Option<AccountId> {
        None
    }
}

/// This trait provides a simple way to get the treasury account.
pub trait TreasuryAccount<AccountId> {
    fn treasury_account() -> AccountId;
}

impl<AccountId: Default> TreasuryAccount<AccountId> for () {
    fn treasury_account() -> AccountId {
        Default::default()
    }
}
