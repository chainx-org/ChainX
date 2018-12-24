// Copyright 2018 Chainpool.

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

// for encode/decode
// Needed for deriving `Serialize` and `Deserialize` for various types.
// We only implement the serde traits for std builds - they're unneeded
// in the wasm runtime.
#[cfg(feature = "std")]
#[macro_use]
extern crate serde_derive;

// Needed for deriving `Encode` and `Decode` for `RawEvent`.
#[macro_use]
extern crate parity_codec_derive;
extern crate parity_codec as codec;

// for substrate
// Needed for the set of mock primitives used in our tests.
#[cfg(test)]
extern crate substrate_primitives;

// for substrate runtime
// map!, vec! marco.
extern crate sr_std as rstd;

extern crate sr_io as runtime_io;
extern crate sr_primitives as runtime_primitives;

// for substrate runtime module lib
// Needed for type-safe access to storage DB.
#[macro_use]
extern crate srml_support as runtime_support;
extern crate srml_balances as balances;
extern crate srml_system as system;

// for chainx runtime module lib
extern crate xrml_xsupport as xsupport;
extern crate xrml_xassets_assets as xassets;

//#[cfg(test)]
//mod tests;

mod withdrawal;

pub use withdrawal::WithdrawLog;

use codec::Encode;
use rstd::prelude::*;
use rstd::result::Result as StdResult;
use runtime_primitives::traits::As;
use runtime_support::dispatch::Result;
use runtime_support::StorageMap;

use xsupport::storage::linked_node::{LinkedNodeCollection, MultiNodeIndex, Node};
use xassets::{ReservedType, Token, ChainT};


pub trait Trait: system::Trait + balances::Trait + xassets::Trait {
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event() = default;
    }
}

decl_event!(
    pub enum Event<T> where
        <T as system::Trait>::AccountId,
        <T as balances::Trait>::Balance
//        <T as system::Trait>::BlockNumber
    {
        Deposit(AccountId, u32, Token, Balance, Option<Vec<u8>>),
        Withdrawal(AccountId, u32, Token, Balance, Option<Vec<u8>>, Vec<u8>, Vec<u8>),
    }
);

#[derive(PartialEq, Eq, Clone, Copy, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub enum DepositState {
    Invalid,
    Success,
    Failed,
}

impl Default for DepositState {
    fn default() -> Self {
        DepositState::Invalid
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub enum WithdrawalState {
    Invalid,
    Locking,
    Success,
    Failed,
}

impl Default for WithdrawalState {
    fn default() -> Self {
        WithdrawalState::Invalid
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub enum Action {
    Deposit(DepositState),
    Withdrawal(WithdrawalState),
}

impl Default for Action {
    /// default not use for Action enum, it's just for the trait
    fn default() -> Self {
        Action::Deposit(Default::default())
    }
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub struct Record<Token, Balance, BlockNumber>
    where
        Token: Clone,
        Balance: Copy,
        BlockNumber: Copy,
{
    action: Action,
    token: Token,
    balance: Balance,
    init_blocknum: BlockNumber,
    txid: Vec<u8>,
    addr: Vec<u8>,
    ext: Vec<u8>,
}

type RecordT<T> = Record<Token, <T as balances::Trait>::Balance, <T as system::Trait>::BlockNumber>;

impl<Token, Balance, BlockNumber> Record<Token, Balance, BlockNumber>
    where
        Token: Clone,
        Balance: Copy,
        BlockNumber: Copy,
{
    pub fn action(&self) -> Action {
        self.action
    }
    pub fn mut_action(&mut self) -> &mut Action {
        &mut self.action
    }
    pub fn token(&self) -> Token {
        self.token.clone()
    }
    pub fn balance(&self) -> Balance {
        self.balance
    }
    pub fn txid(&self) -> Vec<u8> {
        self.txid.clone()
    }
    pub fn addr(&self) -> Vec<u8> {
        self.addr.clone()
    }
    pub fn ext(&self) -> Vec<u8> {
        self.ext.clone()
    }
    /// block num for the record init time.
    pub fn blocknum(&self) -> BlockNumber {
        self.init_blocknum
    }
}

impl<Token, Balance, BlockNumber> Record<Token, Balance, BlockNumber>
    where
        Token: Clone,
        Balance: Copy,
        BlockNumber: Copy,
{
    fn is_init(&self) -> bool {
        match self.action {
            Action::Deposit(ref state) => {
                if let DepositState::Invalid = state {
                    true
                } else {
                    false
                }
            }
            Action::Withdrawal(ref state) => {
                if let WithdrawalState::Invalid = state {
                    true
                } else {
                    false
                }
            }
        }
    }

    fn is_finish(&self) -> bool {
        match self.action {
            Action::Deposit(ref state) => match state {
                DepositState::Success => true,
                DepositState::Failed => true,
                _ => false,
            },
            Action::Withdrawal(ref state) => match state {
                WithdrawalState::Success => true,
                WithdrawalState::Failed => true,
                _ => false,
            },
        }
    }
}


pub struct LinkedMultiKey<T: Trait>(runtime_support::storage::generator::PhantomData<T>);

impl<T: Trait> LinkedNodeCollection for LinkedMultiKey<T> {
    type Header = LogCacheMHeader<T>;
    type NodeMap = WithdrawLogCache<T>;
    type Tail = LogCacheMTail<T>;
}

decl_storage! {
    trait Store for Module<T: Trait> as FinancialRecords {
        /// Record list length of every account
        pub RecordListLenOf get(record_list_len_of): map T::AccountId => u32;
        /// Record list for every account, use accountid and index to index the record of account
        pub RecordListOf get(record_of): map (T::AccountId, u32) => Option<Record<Token, T::Balance, T::BlockNumber>>;
        /// Last deposit index of a account and related Token
        pub LastDepositIndexOf get(last_deposit_index_of): map (T::AccountId, Token) => Option<u32>;
        /// Last withdrawal index of a account and related Token
        pub LastWithdrawalIndexOf get(last_withdrawal_index_of): map (T::AccountId, Token) => Option<u32>;

        /// withdraw log linked node header
        pub LogCacheMHeader get(log_cache_mheader): map Token => Option<MultiNodeIndex<Token, WithdrawLog<T::AccountId>>>;
        /// withdraw log linked node tail
        pub LogCacheMTail get(log_cache_mtail): map Token => Option<MultiNodeIndex<Token, WithdrawLog<T::AccountId>>>;
        /// withdraw log linked node collection
        pub WithdrawLogCache get(withdraw_log_cache): map (T::AccountId, u32) => Option<Node<WithdrawLog<T::AccountId>>>;
    }
}


impl<T: Trait> Module<T> {
    /// get the record list for a account
    pub fn record_list_of(who: &T::AccountId) -> Vec<RecordT<T>> {
        let mut records: Vec<RecordT<T>> = Vec::new();
        let len: u32 = Self::record_list_len_of(who);
        for i in 0..len {
            if let Some(r) = <RecordListOf<T>>::get(&(who.clone(), i)) {
                records.push(r);
            }
        }
        records
    }
    /// get the last record of a account, return a Option, None for the account have not any deposit/withdrawal record
    pub fn last_record_of(who: &T::AccountId) -> Option<(u32, RecordT<T>)> {
        let len: u32 = Self::record_list_len_of(who);
        if len == 0 {
            None
        } else {
            let index = len - 1;
            <RecordListOf<T>>::get(&(who.clone(), index)).map(|r| (index, r))
        }
    }

    pub fn last_deposit_of(who: &T::AccountId, token: &Token) -> Option<(u32, RecordT<T>)> {
        Self::last_deposit_index_of((who.clone(), token.clone()))
            .and_then(|index| <RecordListOf<T>>::get(&(who.clone(), index)).map(|r| (index, r)))
    }

    pub fn last_withdrawal_of(who: &T::AccountId, token: &Token) -> Option<(u32, RecordT<T>)> {
        Self::last_withdrawal_index_of((who.clone(), token.clone()))
            .and_then(|index| <RecordListOf<T>>::get(&(who.clone(), index)).map(|r| (index, r)))
    }

    /// deposit/withdrawal pre-process
    fn before(who: &T::AccountId, token: &Token, is_withdrawal: bool) -> Result {
        if token.as_slice() == <xassets::Module<T> as ChainT>::TOKEN {
            return Err("can't deposit/withdrawal chainx token");
        }

        let r = if is_withdrawal {
            match Self::last_deposit_of(who, token) {
                None => return Err("the account has no deposit record for this token yet"),
                Some((_, record)) => {
                    if !record.is_finish() {
                        return Err("the account has no deposit record for this token yet");
                    }
                }
            }

            Self::last_withdrawal_of(who, token)
        } else {
            Self::last_deposit_of(who, token)
        };

        if let Some((_, record)) = r {
            if !record.is_finish() {
                return Err("the last action have not finished yet! only if the last deposit/withdrawal have finished you can do a new action.");
            }
        }
        Ok(())
    }

    /// insert a new record for a account, notice the record state must be init state
    fn new_record(who: &T::AccountId, record: &RecordT<T>) -> StdResult<u32, &'static str> {
        if !record.is_init() {
            return Err("new record should be Invalid state first");
        }
        let len: u32 = Self::record_list_len_of(who);
        <RecordListOf<T>>::insert(&(who.clone(), len), record.clone());
        <RecordListLenOf<T>>::insert(who, len + 1); // len is more than 1 to max index
        let key = (who.clone(), record.token());
        match record.action() {
            Action::Deposit(_) => <LastDepositIndexOf<T>>::insert(&key, len),
            Action::Withdrawal(_) => <LastWithdrawalIndexOf<T>>::insert(&key, len),
        }
        Ok(len)
    }
}

impl<T: Trait> Module<T> {
    /// deposit, notice this func has include deposit_init and deposit_finish (not wait for block confirm process)
    pub fn deposit(
        who: &T::AccountId,
        token: &Token,
        balance: T::Balance,
        txid: Option<Vec<u8>>,
    ) -> Result {
        let index = Self::deposit_with_index(who, token, balance)?;

        runtime_io::print("deposit ---who---token---balance--index");
        runtime_io::print(who.encode().as_slice());
        runtime_io::print(token.as_slice());
        runtime_io::print(balance.as_() as u64);
        runtime_io::print(index as u64);

        Self::deposit_finish_with_index(who, index, txid).map(|_| ())
    }

    /// withdrawal, notice this func has include withdrawal_init and withdrawal_locking
    pub fn withdrawal(
        who: &T::AccountId,
        token: &Token,
        balance: T::Balance,
        addr: Vec<u8>,
        ext: Vec<u8>,
    ) -> Result {
        let index = Self::withdrawal_with_index(who, token, balance, addr, ext)?;
        runtime_io::print("withdrawal ---who---token---balance--index");
        runtime_io::print(who.encode().as_slice());
        runtime_io::print(token.as_slice());
        runtime_io::print(balance.as_() as u64);
        runtime_io::print(index as u64);

        // set to withdraw cache
        let n = Node::new(WithdrawLog::<T::AccountId>::new(who.clone(), index));
        n.init_storage_withkey::<LinkedMultiKey<T>, Token>(token.clone());

        if let Some(tail_index) = Self::log_cache_mtail(token) {
            if let Some(mut tail_node) = Self::withdraw_log_cache(tail_index.index()) {
                tail_node
                    .add_option_node_after_withkey::<LinkedMultiKey<T>, Token>(n, token.clone())?;
            }
        }

        Self::withdrawal_locking_with_index(who, index).map(|_| ())
    }

    /// withdrawal finish, let the locking token destroy
    pub fn withdrawal_finish(who: &T::AccountId, token: &Token, txid: Option<Vec<u8>>) -> Result {
        let r = Self::last_withdrawal_index_of(&(who.clone(), token.clone()));
        if r.is_none() {
            return Err("have not executed withdrawal() or withdrawal_init() yet for this record");
        }

        // remove withdraw cache
        if let Some(header) = Self::log_cache_mheader(token) {
            let mut index = header.index();

            while let Some(mut node) = Self::withdraw_log_cache(&index) {
                if node.data.accountid() == *who && node.data.index() == r.unwrap() {
                    // remove cache
                    node.remove_option_node_withkey::<LinkedMultiKey<T>, Token>(token.clone())?;
                    break;
                }
                if let Some(next) = node.next() {
                    index = next;
                } else {
                    return Err("not found this withdraw log in cache");
                }
            }
        } else {
            return Err("the withdraw log node header not exist for this Token");
        }

        Self::withdrawal_finish_with_index(who, r.unwrap(), txid).map(|_| ())
    }

    pub fn withdrawal_cache_indexs(token: &Token) -> Option<Vec<(T::AccountId, u32)>> {
        let mut vec = Vec::new();
        if let Some(header) = Self::log_cache_mheader(token) {
            let mut index = header.index();

            while let Some(node) = Self::withdraw_log_cache(&index) {
                //                let key = (node.data.accountid(), node.data.index());
                //                if let Some(r) = <RecordListOf<T>>::get(&key) {
                //                    vec.push((node.data.accountid(), r.balance(), r.addr(), r.ext()));
                //                }
                vec.push(node.index());
                if let Some(next) = node.next() {
                    index = next;
                } else {
                    return Some(vec);
                }
            }
        }
        None
    }

    /// deposit init, use for record a deposit process begin, usually for start block confirm process
    pub fn deposit_init(who: &T::AccountId, token: &Token, balance: T::Balance) -> Result {
        Self::deposit_with_index(who, token, balance).map(|_| ())
    }
    /// deposit finish, use for change the deposit record to final, success mark the deposit if success
    pub fn deposit_finish(who: &T::AccountId, token: &Token, txid: Option<Vec<u8>>) -> Result {
        let r = Self::last_deposit_index_of(&(who.clone(), token.clone()));
        if r.is_none() {
            return Err("have not executed deposit_init() yet for this record");
        }
        Self::deposit_finish_with_index(who, r.unwrap(), txid).map(|_| ())
    }

    //    /// withdrawal init, use for record a withdrawal start, should call withdrawal_locking after it
    //    fn withdrawal_init(who: &T::AccountId, token: &Token, balance: T::Balance) -> Result {
    //        Self::withdrawal_with_index(who, token, balance).map(|_| ())
    //    }
    //    /// change the free token to locking state
    //    fn withdrawal_locking(who: &T::AccountId, token: &Token) -> Result {
    //        let r = Self::last_withdrawal_index_of(&(who.clone(), token.clone()));
    //        if r.is_none() {
    //            return Err("have not executed withdrawal() or withdrawal_init() yet for this record");
    //        }
    //        Self::withdrawal_locking_with_index(who, r.unwrap()).map(|_| ())
    //    }

    /// deposit init, notice this func return index to show the index of records for this account
    pub fn deposit_with_index(
        who: &T::AccountId,
        token: &Token,
        balance: T::Balance,
    ) -> StdResult<u32, &'static str> {
        Self::before(who, token, false)?;

        <xassets::Module<T>>::is_valid_asset(token)?;

        let r = Record {
            action: Action::Deposit(Default::default()),
            token: token.clone(),
            balance,
            init_blocknum: <system::Module<T>>::block_number(),
            txid: Vec::new(),
            addr: Vec::new(),
            ext: Vec::new(),
        };
        let index = Self::new_record(who, &r)?;
        Ok(index)
    }
    /// deposit finish, should use index to find the old deposit record, success flag mark the success
    fn deposit_finish_with_index(
        who: &T::AccountId,
        index: u32,
        txid: Option<Vec<u8>>,
    ) -> StdResult<u32, &'static str> {
        let key = (who.clone(), index);
        if let Some(mut r) = <RecordListOf<T>>::get(&key) {
            if r.is_finish() {
                return Err("the deposit record should not be a finish state");
            }

            let deposit_txid: Vec<u8>;
            let token = r.token();
            let bal = r.balance();
            // change state
            match r.mut_action() {
                Action::Deposit(ref mut state) => {
                    if let Some(txid) = txid {
                        deposit_txid = txid;
                        *state = DepositState::Success;
                        // call xassets to issue token for this accountid
                        <xassets::Module<T>>::issue(who, &token, bal)?;

                        Self::deposit_event(RawEvent::Deposit(
                            who.clone(),
                            index,
                            token,
                            bal,
                            Some(deposit_txid.clone()),
                        ));
                    } else {
                        deposit_txid = b"".to_vec();
                        *state = DepositState::Failed;

                        Self::deposit_event(RawEvent::Deposit(
                            who.clone(),
                            index,
                            token,
                            bal,
                            None,
                        ));
                    }
                }
                _ => return Err("err action type in deposit_finish"),
            }
            r.txid = deposit_txid;
            <RecordListOf<T>>::insert(&key, r);
            Ok(index)
        } else {
            return Err("the deposit record for this (accountid, index) not exist");
        }
    }
    /// withdrawal init, notice this func return index to show the index of records for this account
    fn withdrawal_with_index(
        who: &T::AccountId,
        token: &Token,
        balance: T::Balance,
        addr: Vec<u8>,
        ext: Vec<u8>,
    ) -> StdResult<u32, &'static str> {
        Self::before(who, token, true)?;

        <xassets::Module<T>>::is_valid_asset_for(who, token)?;
        // check token balance
        if <xassets::Module<T>>::free_balance(&(who.clone(), token.clone())) < balance {
            return Err("not enough free token to withdraw");
        }

        let r = Record {
            action: Action::Withdrawal(Default::default()),
            token: token.clone(),
            balance,
            init_blocknum: <system::Module<T>>::block_number(),
            txid: Vec::new(),
            addr,
            ext,
        };
        let index = Self::new_record(who, &r)?;
        Ok(index)
    }
    /// withdrawal lock, should use index to find out which record to change to locking state
    fn withdrawal_locking_with_index(
        who: &T::AccountId,
        index: u32,
    ) -> StdResult<u32, &'static str> {
        let key = (who.clone(), index);
        if let Some(ref mut r) = <RecordListOf<T>>::get(&key) {
            if r.is_finish() {
                return Err("the deposit record should not be a finish state");
            }

            let token = r.token();
            let bal = r.balance();
            // change state
            match r.mut_action() {
                Action::Withdrawal(ref mut state) => match state {
                    WithdrawalState::Invalid => {
                        *state = WithdrawalState::Locking;

                        <xassets::Module<T>>::reserve(who, &token, bal, ReservedType::AssetsWithdrawal)?;
                    }
                    _ => return Err("the withdrawal state must be Invalid."),
                },
                _ => return Err("err action type in deposit_finish"),
            }
            <RecordListOf<T>>::insert(&key, r.clone());

            Ok(index)
        } else {
            return Err("the withdrawal record for this (accountid, index) not exist");
        }
    }
    /// withdrawal finish, should use index to find out which record to changed to final, success flag mark success, if false, release the token to free
    fn withdrawal_finish_with_index(
        who: &T::AccountId,
        index: u32,
        txid: Option<Vec<u8>>,
    ) -> StdResult<u32, &'static str> {
        let key = (who.clone(), index);
        if let Some(mut r) = <RecordListOf<T>>::get(&key) {
            if r.is_finish() {
                return Err("the deposit record should not be a finish state");
            }

            let withdraw_txid: Vec<u8>;
            let token = r.token();
            let bal = r.balance();
            let addr = r.addr();
            let ext = r.ext();

            // change state
            match r.mut_action() {
                Action::Withdrawal(ref mut state) => match state {
                    WithdrawalState::Locking => {
                        if let Some(txid) = txid {
                            withdraw_txid = txid;
                            *state = WithdrawalState::Success;

                            <xassets::Module<T>>::destroy(
                                who,
                                &token,
                                bal,
                                ReservedType::AssetsWithdrawal,
                            )?;

                            Self::deposit_event(RawEvent::Withdrawal(
                                who.clone(),
                                index,
                                token,
                                bal,
                                Some(withdraw_txid.clone()),
                                addr,
                                ext,
                            ));
                        } else {
                            withdraw_txid = b"".to_vec();
                            *state = WithdrawalState::Failed;

                            <xassets::Module<T>>::unreserve(
                                who,
                                &token,
                                bal,
                                ReservedType::AssetsWithdrawal,
                            )?;

                            Self::deposit_event(RawEvent::Withdrawal(
                                who.clone(),
                                index,
                                token,
                                bal,
                                None,
                                addr,
                                ext,
                            ));
                        }
                    }
                    _ => return Err("the withdrawal state must be Locking."),
                },
                _ => return Err("err action type in deposit_finish"),
            }
            r.txid = withdraw_txid;
            <RecordListOf<T>>::insert(&key, r);

            Ok(index)
        } else {
            return Err("the withdrawal record for this (accountid, index) not exist");
        }
    }
}
