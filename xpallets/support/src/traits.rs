use crate::error;
use sp_std::fmt::Debug;

pub trait MultiSig<AccountId: PartialEq + Debug> {
    fn multisig() -> AccountId;

    fn check_multisig(who: &AccountId) -> bool {
        let current_multisig_addr = Self::multisig();
        if current_multisig_addr != *who {
            error!("[check_multisig]|the account not match current multisig addr|current:{:?}|who:{:?}", current_multisig_addr, who);
            false
        } else {
            true
        }
    }
}

pub trait Validator<AccountId> {
    fn is_validator(who: &AccountId) -> bool;
    fn validator_for(_: &[u8]) -> Option<AccountId>;
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
