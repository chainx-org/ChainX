// Copyright 2018 Chainpool.
use codec::Codec;

use xsupport::storage::linked_node::NodeT;

#[derive(PartialEq, Eq, Clone, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub struct WithdrawLog<AccountId>
where
    AccountId: Codec + Clone + Ord + Default,
{
    accountid: AccountId,
    index: u32,
}

impl<AccountId> NodeT for WithdrawLog<AccountId>
where
    AccountId: Codec + Clone + Ord + Default,
{
    type Index = (AccountId, u32);

    fn index(&self) -> Self::Index {
        (self.accountid.clone(), self.index)
    }
}

impl<AccountId> WithdrawLog<AccountId>
where
    AccountId: Codec + Clone + Ord + Default,
{
    pub fn new(accountid: AccountId, index: u32) -> WithdrawLog<AccountId> {
        WithdrawLog { accountid, index }
    }
    pub fn accountid(&self) -> AccountId {
        self.accountid.clone()
    }
    pub fn index(&self) -> u32 {
        self.index
    }
}
