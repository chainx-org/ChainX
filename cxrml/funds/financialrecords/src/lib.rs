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
// Needed for tests (`with_externalities`).
#[cfg(test)]
extern crate sr_io as runtime_io;
extern crate sr_primitives as runtime_primitives;

// for substrate runtime module lib
// Needed for type-safe access to storage DB.
#[macro_use]
extern crate srml_support as runtime_support;
extern crate srml_balances as balances;
extern crate srml_system as system;

// for chainx runtime module lib
#[cfg(test)]
extern crate cxrml_associations as associations;
extern crate cxrml_support as cxsupport;
#[cfg(test)]
extern crate cxrml_system as cxsystem;
extern crate cxrml_tokenbalances as tokenbalances;

#[cfg(test)]
mod tests;

use codec::Codec;
use rstd::prelude::*;
use rstd::result::Result as StdResult;
use runtime_primitives::traits::OnFinalise;
use runtime_support::dispatch::Result;
use runtime_support::{StorageMap, StorageValue};

use cxsupport::storage::linked_node::{LinkedNodeCollection, MultiNodeIndex, Node, NodeT};
pub use tokenbalances::{ReservedType, Symbol};

pub trait OnDepositToken<AccountId, TokenBalance> {
    fn on_deposit_token(who: &AccountId, sym: &Symbol, value: TokenBalance);
}

pub trait OnWithdrawToken<AccountId, TokenBalance> {
    fn on_withdraw_token(who: &AccountId, sym: &Symbol, value: TokenBalance);
}

impl<AccountId, TokenBalance> OnDepositToken<AccountId, TokenBalance> for () {
    fn on_deposit_token(_: &AccountId, _: &Symbol, _: TokenBalance) {}
}

impl<AccountId, TokenBalance> OnWithdrawToken<AccountId, TokenBalance> for () {
    fn on_withdraw_token(_: &AccountId, _: &Symbol, _: TokenBalance) {}
}

pub trait Trait: tokenbalances::Trait {
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
    /// deposit trigger
    type OnDepositToken: OnDepositToken<Self::AccountId, Self::TokenBalance>;
    /// withdraw trigger
    type OnWithdrawToken: OnWithdrawToken<Self::AccountId, Self::TokenBalance>;
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
    }
}

impl<T: Trait> OnFinalise<T::BlockNumber> for Module<T> {
    fn on_finalise(_: T::BlockNumber) {
        // do nothing
    }
}

decl_event!(
    pub enum Event<T> where
        <T as system::Trait>::AccountId,
        <T as tokenbalances::Trait>::TokenBalance,
        <T as system::Trait>::BlockNumber,
        <T as balances::Trait>::Balance
    {
        /// deposit init event, record init blocknumber
        DepositInit(AccountId, u32, Symbol, TokenBalance, BlockNumber),
        /// deposit success event, record success blocknumber
        DepositSuccess(AccountId, u32, Symbol, TokenBalance, BlockNumber),
        /// deposit failed event, record failed blocknumber, for example meet chain fork
        DepositFailed(AccountId, u32, Symbol, TokenBalance, BlockNumber),

        /// withdraw init, record init blocknumber
        WithdrawalInit(AccountId, u32, Symbol, TokenBalance, BlockNumber),
        /// withdraw locking, record locking blocknumber
        WithdrawalLocking(AccountId, u32, Symbol, TokenBalance, BlockNumber),
        /// withdraw success, record release blocknumber
        WithdrawalSuccess(AccountId, u32, Symbol, TokenBalance, BlockNumber),
        /// withdraw failed, record failed blocknumber, for example meet not collect enough sign
        WithdrawalFailed(AccountId, u32, Symbol, TokenBalance, BlockNumber),

        /// set withdrawal fee, by Root
        SetWithdrawalFee(Balance),
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
pub struct Record<Symbol, TokenBalance, BlockNumber>
where
    Symbol: Clone,
    TokenBalance: Copy,
    BlockNumber: Copy,
{
    action: Action,
    symbol: Symbol,
    balance: TokenBalance,
    init_blocknum: BlockNumber,
    txid: Vec<u8>,
    addr: Vec<u8>,
    ext: Vec<u8>,
}

type RecordT<T> =
    Record<Symbol, <T as tokenbalances::Trait>::TokenBalance, <T as system::Trait>::BlockNumber>;

impl<Symbol, TokenBalance, BlockNumber> Record<Symbol, TokenBalance, BlockNumber>
where
    Symbol: Clone,
    TokenBalance: Copy,
    BlockNumber: Copy,
{
    pub fn action(&self) -> Action {
        self.action
    }
    pub fn mut_action(&mut self) -> &mut Action {
        &mut self.action
    }
    pub fn symbol(&self) -> Symbol {
        self.symbol.clone()
    }
    pub fn balance(&self) -> TokenBalance {
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

impl<Symbol, TokenBalance, BlockNumber> Record<Symbol, TokenBalance, BlockNumber>
where
    Symbol: Clone,
    TokenBalance: Copy,
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
    pub fn accountid(&self) -> AccountId {
        self.accountid.clone()
    }
    pub fn index(&self) -> u32 {
        self.index
    }
}

pub struct LinkedMultiKey<T: Trait>(runtime_support::storage::generator::PhantomData<T>);

impl<T: Trait> LinkedNodeCollection for LinkedMultiKey<T> {
    type Header = LogHeaderFor<T>;
    type NodeMap = WithdrawLogCache<T>;
    type Tail = LogTailFor<T>;
}

decl_storage! {
    trait Store for Module<T: Trait> as FinancialRecords {
        /// Record list length of every account
        pub RecordsLenOf get(records_len_of): map T::AccountId => u32;
        /// Record list for every account, use accountid and index to index the record of account
        pub RecordsOf: map (T::AccountId, u32) => Option<Record<Symbol, T::TokenBalance, T::BlockNumber>>;
        /// Last deposit index of a account and related symbol
        pub LastDepositIndexOf get(last_deposit_index_of): map (T::AccountId, Symbol) => Option<u32>;
        /// Last withdrawal index of a account and related symbol
        pub LastWithdrawalIndexOf get(last_withdrawal_index_of): map (T::AccountId, Symbol) => Option<u32>;

        /// withdraw log linked node header
        pub LogHeaderFor get(log_header_for): map Symbol => Option<MultiNodeIndex<Symbol, WithdrawLog<T::AccountId>>>;
        /// withdraw log linked node tail
        pub LogTailFor get(log_tail_for): map Symbol => Option<MultiNodeIndex<Symbol, WithdrawLog<T::AccountId>>>;
        /// withdraw log linked node collection
        pub WithdrawLogCache get(withdraw_log_cache): map (T::AccountId, u32) => Option<Node<WithdrawLog<T::AccountId>>>;

        /// Fee for withdrawal, can change by Root
        pub WithdrawalFee get(withdrawal_fee) config(): T::Balance;
    }
}

impl<T: Trait> Module<T> {
    /// Deposit one of this module's events.
    fn deposit_event(event: Event<T>) {
        <system::Module<T>>::deposit_event(<T as Trait>::Event::from(event).into());
    }
}

impl<T: Trait> Module<T> {
    /// get the record list for a account
    pub fn records_of(who: &T::AccountId) -> Vec<RecordT<T>> {
        let mut records: Vec<RecordT<T>> = Vec::new();
        let len: u32 = Self::records_len_of(who);
        for i in 0..len {
            if let Some(r) = <RecordsOf<T>>::get(&(who.clone(), i)) {
                records.push(r);
            }
        }
        records
    }
    /// get the last record of a account, return a Option, None for the account have not any deposit/withdrawal record
    pub fn last_record_of(who: &T::AccountId) -> Option<(u32, RecordT<T>)> {
        let len: u32 = Self::records_len_of(who);
        if len == 0 {
            None
        } else {
            let index = len - 1;
            <RecordsOf<T>>::get(&(who.clone(), index)).map(|r| (index, r))
        }
    }

    pub fn last_deposit_of(who: &T::AccountId, sym: &Symbol) -> Option<(u32, RecordT<T>)> {
        Self::last_deposit_index_of((who.clone(), sym.clone()))
            .and_then(|index| <RecordsOf<T>>::get(&(who.clone(), index)).map(|r| (index, r)))
    }

    pub fn last_withdrawal_of(who: &T::AccountId, sym: &Symbol) -> Option<(u32, RecordT<T>)> {
        Self::last_withdrawal_index_of((who.clone(), sym.clone()))
            .and_then(|index| <RecordsOf<T>>::get(&(who.clone(), index)).map(|r| (index, r)))
    }

    /// deposit/withdrawal pre-process
    fn before(who: &T::AccountId, sym: &Symbol, is_withdrawal: bool) -> Result {
        if sym.as_slice() == T::CHAINX_SYMBOL {
            return Err("can't deposit/withdrawal chainx token");
        }

        let r = if is_withdrawal {
            match Self::last_deposit_of(who, sym) {
                None => return Err("the account has no deposit record for this token yet"),
                Some((_, record)) => {
                    if !record.is_finish() {
                        return Err("the account has no deposit record for this token yet");
                    }
                }
            }

            Self::last_withdrawal_of(who, sym)
        } else {
            Self::last_deposit_of(who, sym)
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
        let len: u32 = Self::records_len_of(who);
        <RecordsOf<T>>::insert(&(who.clone(), len), record.clone());
        <RecordsLenOf<T>>::insert(who, len + 1); // len is more than 1 to max index
        let key = (who.clone(), record.symbol());
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
        sym: &Symbol,
        balance: T::TokenBalance,
        txid: Option<Vec<u8>>,
    ) -> Result {
        let index = Self::deposit_with_index(who, sym, balance)?;
        Self::deposit_finish_with_index(who, index, txid).map(|_| ())
    }

    /// withdrawal, notice this func has include withdrawal_init and withdrawal_locking
    pub fn withdrawal(
        who: &T::AccountId,
        sym: &Symbol,
        balance: T::TokenBalance,
        addr: Vec<u8>,
        ext: Vec<u8>,
    ) -> Result {
        let index = Self::withdrawal_with_index(who, sym, balance, addr, ext)?;

        // set to withdraw cache
        let n = Node::new(WithdrawLog::<T::AccountId> {
            accountid: who.clone(),
            index,
        });
        n.init_storage_withkey::<LinkedMultiKey<T>, Symbol>(sym.clone());

        if let Some(tail_index) = Self::log_tail_for(sym) {
            if let Some(mut tail_node) = Self::withdraw_log_cache(tail_index.index()) {
                tail_node
                    .add_option_node_after_withkey::<LinkedMultiKey<T>, Symbol>(n, sym.clone())?;
            }
        }

        Self::withdrawal_locking_with_index(who, index).map(|_| ())
    }

    /// withdrawal finish, let the locking token destroy
    pub fn withdrawal_finish(who: &T::AccountId, sym: &Symbol, txid: Option<Vec<u8>>) -> Result {
        let r = Self::last_withdrawal_index_of(&(who.clone(), sym.clone()));
        if r.is_none() {
            return Err("have not executed withdrawal() or withdrawal_init() yet for this record");
        }

        // remove withdraw cache
        if let Some(header) = Self::log_header_for(sym) {
            let mut index = header.index();

            while let Some(mut node) = Self::withdraw_log_cache(&index) {
                if node.data.accountid() == *who && node.data.index() == r.unwrap() {
                    // remove cache
                    node.remove_option_node_withkey::<LinkedMultiKey<T>, Symbol>(sym.clone())?;
                    break;
                }
                if let Some(next) = node.next() {
                    index = next;
                } else {
                    return Err("not found this withdraw log in cache");
                }
            }
        } else {
            return Err("the withdraw log node header not exist for this symbol");
        }

        Self::withdrawal_finish_with_index(who, r.unwrap(), txid).map(|_| ())
    }

    pub fn get_withdraw_cache(sym: &Symbol) -> Option<Vec<(T::AccountId, T::TokenBalance)>> {
        let mut vec = Vec::new();
        if let Some(header) = Self::log_header_for(sym) {
            let mut index = header.index();

            while let Some(node) = Self::withdraw_log_cache(&index) {
                let key = (node.data.accountid().clone(), node.data.index());
                if let Some(r) = <RecordsOf<T>>::get(&key) {
                    vec.push((node.data.accountid().clone(), r.balance()));
                }
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
    pub fn deposit_init(who: &T::AccountId, sym: &Symbol, balance: T::TokenBalance) -> Result {
        Self::deposit_with_index(who, sym, balance).map(|_| ())
    }
    /// deposit finish, use for change the deposit record to final, success mark the deposit if success
    pub fn deposit_finish(who: &T::AccountId, sym: &Symbol, txid: Option<Vec<u8>>) -> Result {
        let r = Self::last_deposit_index_of(&(who.clone(), sym.clone()));
        if r.is_none() {
            return Err("have not executed deposit_init() yet for this record");
        }
        Self::deposit_finish_with_index(who, r.unwrap(), txid).map(|_| ())
    }

    //    /// withdrawal init, use for record a withdrawal start, should call withdrawal_locking after it
    //    fn withdrawal_init(who: &T::AccountId, sym: &Symbol, balance: T::TokenBalance) -> Result {
    //        Self::withdrawal_with_index(who, sym, balance).map(|_| ())
    //    }
    //    /// change the free token to locking state
    //    fn withdrawal_locking(who: &T::AccountId, sym: &Symbol) -> Result {
    //        let r = Self::last_withdrawal_index_of(&(who.clone(), sym.clone()));
    //        if r.is_none() {
    //            return Err("have not executed withdrawal() or withdrawal_init() yet for this record");
    //        }
    //        Self::withdrawal_locking_with_index(who, r.unwrap()).map(|_| ())
    //    }

    /// deposit init, notice this func return index to show the index of records for this account
    pub fn deposit_with_index(
        who: &T::AccountId,
        sym: &Symbol,
        balance: T::TokenBalance,
    ) -> StdResult<u32, &'static str> {
        Self::before(who, sym, false)?;

        <tokenbalances::Module<T>>::is_valid_token(sym)?;

        let r = Record {
            action: Action::Deposit(Default::default()),
            symbol: sym.clone(),
            balance,
            init_blocknum: <system::Module<T>>::block_number(),
            txid: Vec::new(),
            addr: Vec::new(),
            ext: Vec::new(),
        };
        let index = Self::new_record(who, &r)?;
        Self::deposit_event(RawEvent::DepositInit(
            who.clone(),
            index,
            r.symbol(),
            r.balance(),
            r.blocknum(),
        ));

        Ok(index)
    }
    /// deposit finish, should use index to find the old deposit record, success flag mark the success
    fn deposit_finish_with_index(
        who: &T::AccountId,
        index: u32,
        txid: Option<Vec<u8>>,
    ) -> StdResult<u32, &'static str> {
        let key = (who.clone(), index);
        if let Some(mut r) = <RecordsOf<T>>::get(&key) {
            if r.is_finish() {
                return Err("the deposit record should not be a finish state");
            }

            let deposit_txid: Vec<u8>;
            let sym = r.symbol();
            let bal = r.balance();
            // change state
            match r.mut_action() {
                Action::Deposit(ref mut state) => {
                    if let Some(txid) = txid {
                        deposit_txid = txid;
                        *state = DepositState::Success;
                        // call tokenbalances to issue token for this accountid
                        <tokenbalances::Module<T>>::issue(who, &sym, bal)?;

                        // withdraw trigger
                        T::OnDepositToken::on_deposit_token(who, &sym, bal);

                        Self::deposit_event(RawEvent::DepositSuccess(
                            who.clone(),
                            index,
                            sym,
                            bal,
                            <system::Module<T>>::block_number(),
                        ));
                    } else {
                        deposit_txid = b"".to_vec();
                        *state = DepositState::Failed;

                        Self::deposit_event(RawEvent::DepositFailed(
                            who.clone(),
                            index,
                            sym,
                            bal,
                            <system::Module<T>>::block_number(),
                        ));
                    }
                }
                _ => return Err("err action type in deposit_finish"),
            }
            r.txid = deposit_txid;
            <RecordsOf<T>>::insert(&key, r);
            Ok(index)
        } else {
            return Err("the deposit record for this (accountid, index) not exist");
        }
    }
    /// withdrawal init, notice this func return index to show the index of records for this account
    fn withdrawal_with_index(
        who: &T::AccountId,
        sym: &Symbol,
        balance: T::TokenBalance,
        addr: Vec<u8>,
        ext: Vec<u8>,
    ) -> StdResult<u32, &'static str> {
        Self::before(who, sym, true)?;

        <tokenbalances::Module<T>>::is_valid_token_for(who, sym)?;
        // check token balance
        if <tokenbalances::Module<T>>::free_token(&(who.clone(), sym.clone())) < balance {
            return Err("not enough free token to withdraw");
        }

        let r = Record {
            action: Action::Withdrawal(Default::default()),
            symbol: sym.clone(),
            balance,
            init_blocknum: <system::Module<T>>::block_number(),
            txid: Vec::new(),
            addr,
            ext,
        };
        let index = Self::new_record(who, &r)?;
        Self::deposit_event(RawEvent::WithdrawalInit(
            who.clone(),
            index,
            r.symbol(),
            r.balance(),
            r.blocknum(),
        ));
        Ok(index)
    }
    /// withdrawal lock, should use index to find out which record to change to locking state
    fn withdrawal_locking_with_index(
        who: &T::AccountId,
        index: u32,
    ) -> StdResult<u32, &'static str> {
        let key = (who.clone(), index);
        if let Some(ref mut r) = <RecordsOf<T>>::get(&key) {
            if r.is_finish() {
                return Err("the deposit record should not be a finish state");
            }

            let sym = r.symbol();
            let bal = r.balance();
            // change state
            match r.mut_action() {
                Action::Withdrawal(ref mut state) => match state {
                    WithdrawalState::Invalid => {
                        *state = WithdrawalState::Locking;

                        <tokenbalances::Module<T>>::reserve(who, &sym, bal, ReservedType::Funds)?;

                        Self::deposit_event(RawEvent::WithdrawalLocking(
                            who.clone(),
                            index,
                            sym,
                            bal,
                            <system::Module<T>>::block_number(),
                        ));
                    }
                    _ => return Err("the withdrawal state must be Invalid."),
                },
                _ => return Err("err action type in deposit_finish"),
            }
            <RecordsOf<T>>::insert(&key, r.clone());

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
        if let Some(mut r) = <RecordsOf<T>>::get(&key) {
            if r.is_finish() {
                return Err("the deposit record should not be a finish state");
            }

            let withdraw_txid: Vec<u8>;
            let sym = r.symbol();
            let bal = r.balance();

            // change state
            match r.mut_action() {
                Action::Withdrawal(ref mut state) => match state {
                    WithdrawalState::Locking => {
                        if let Some(txid) = txid {
                            withdraw_txid = txid;
                            *state = WithdrawalState::Success;

                            <tokenbalances::Module<T>>::destroy(
                                who,
                                &sym,
                                bal,
                                ReservedType::Funds,
                            )?;
                            // withdraw trigger
                            T::OnWithdrawToken::on_withdraw_token(who, &sym, bal);

                            Self::deposit_event(RawEvent::WithdrawalSuccess(
                                who.clone(),
                                index,
                                sym,
                                bal,
                                <system::Module<T>>::block_number(),
                            ));
                        } else {
                            withdraw_txid = b"".to_vec();
                            *state = WithdrawalState::Failed;

                            <tokenbalances::Module<T>>::unreserve(
                                who,
                                &sym,
                                bal,
                                ReservedType::Funds,
                            )?;

                            Self::deposit_event(RawEvent::WithdrawalFailed(
                                who.clone(),
                                index,
                                sym,
                                bal,
                                <system::Module<T>>::block_number(),
                            ));
                        }
                    }
                    _ => return Err("the withdrawal state must be Locking."),
                },
                _ => return Err("err action type in deposit_finish"),
            }
            r.txid = withdraw_txid;
            <RecordsOf<T>>::insert(&key, r);

            Ok(index)
        } else {
            return Err("the withdrawal record for this (accountid, index) not exist");
        }
    }
}
