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
}

impl<AccountId> Validator<AccountId> for () {
    fn is_validator(_: &AccountId) -> bool {
        false
    }
}
