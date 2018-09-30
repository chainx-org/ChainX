// Copyright 2018 Chainpool.

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

// map!, vec! marco.
#[cfg_attr(feature = "std", macro_use)]
extern crate sr_std as rstd;
// Needed for tests (`with_externalities`).
#[cfg(test)]
extern crate sr_io as runtime_io;

// Needed for the set of mock primitives used in our tests.
#[cfg(test)]
extern crate substrate_primitives;

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

// Needed for type-safe access to storage DB.
#[macro_use]
extern crate srml_support as runtime_support;

extern crate sr_primitives as runtime_primitives;
extern crate srml_system as system;
// for test
extern crate srml_balances as balances;

extern crate cxrml_support as cxrt_support;
extern crate cxrml_tokenbalances as tokenbalances;

#[cfg(test)]
mod tests;

use rstd::prelude::*;
use runtime_support::dispatch::Result as DispatchResult;
use runtime_support::{StorageMap, StorageValue};
use runtime_primitives::traits::OnFinalise;
use rstd::result::Result;

use cxrt_support::StorageDoubleMap;

pub trait Trait: tokenbalances::Trait {
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        // no call for this module
        /// set deposit fee, call by ROOT
        fn set_deposit_fee(val: T::Balance) -> DispatchResult;
        /// set withdrawal fee, call by ROOT
        fn set_withdrawal_fee(val: T::Balance) -> DispatchResult;
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
        <T as tokenbalances::Trait>::Symbol,
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

        /// set deposit fee, by Root
        SetDepositFee(Balance),
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

#[derive(PartialEq, Eq, Clone, Copy, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub struct Record<Symbol, TokenBalance, BlockNumber> where
    Symbol: Copy, TokenBalance: Copy, BlockNumber: Copy,
{
    action: Action,
    symbol: Symbol,
    balance: TokenBalance,
    init_blocknum: BlockNumber,
}

type RecordT<T> = Record<<T as tokenbalances::Trait>::Symbol, <T as tokenbalances::Trait>::TokenBalance, <T as system::Trait>::BlockNumber>;

impl<Symbol, TokenBalance, BlockNumber> Record<Symbol, TokenBalance, BlockNumber> where
    Symbol: Copy, TokenBalance: Copy, BlockNumber: Copy,
{
    pub fn action(&self) -> Action { self.action }
    pub fn mut_action(&mut self) -> &mut Action { &mut self.action }
    pub fn symbol(&self) -> Symbol { self.symbol }
    pub fn balance(&self) -> TokenBalance { self.balance }
    /// block num for the record init time.
    pub fn blocknum(&self) -> BlockNumber { self.init_blocknum }
}

impl<Symbol, TokenBalance, BlockNumber> Record<Symbol, TokenBalance, BlockNumber> where
    Symbol: Copy, TokenBalance: Copy, BlockNumber: Copy,
{
    fn is_init(&self) -> bool {
        match self.action {
            Action::Deposit(ref state) => {
                if let DepositState::Invalid = state { true } else { false }
            }
            Action::Withdrawal(ref state) => {
                if let WithdrawalState::Invalid = state { true } else { false }
            }
        }
    }

    fn is_finish(&self) -> bool {
        match self.action {
            Action::Deposit(ref state) => {
                match state {
                    DepositState::Success => true,
                    DepositState::Failed => true,
                    _ => false,
                }
            }
            Action::Withdrawal(ref state) => {
                match state {
                    WithdrawalState::Success => true,
                    WithdrawalState::Failed => true,
                    _ => false,
                }
            }
        }
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as FinancialRecords {
        /// Record list length of every account
        pub RecordsLenOf get(records_len_of): default map [T::AccountId => u32];
        /// Fee for deposit, can change by Root
        pub DepositFee get(deposit_fee): default T::Balance;
        /// Fee for withdrawal, can change by Root
        pub WithdrawalFee get(withdrawal_fee): default T::Balance;
    }
}
/// Record list for every account, use accountid and index to index the record of account
pub(crate) struct RecordsOf<T>(::rstd::marker::PhantomData<T>);

impl<T: Trait> StorageDoubleMap for RecordsOf<T> {
    type Key1 = T::AccountId;
    type Key2 = u32;
    type Value = Record<T::Symbol, T::TokenBalance, T::BlockNumber>;
    const PREFIX: &'static [u8] = b"FinancialRecords RecordsOf";
}

impl<T: Trait> Module<T> {
    /// Deposit one of this module's events.
    fn deposit_event(event: Event<T>) {
        <system::Module<T>>::deposit_event(<T as Trait>::Event::from(event).into());
    }
    // public call
    fn set_deposit_fee(val: T::Balance) -> DispatchResult {
        <DepositFee<T>>::put(val);
        Self::deposit_event(RawEvent::SetDepositFee(val));
        Ok(())
    }
    fn set_withdrawal_fee(val: T::Balance) -> DispatchResult {
        <WithdrawalFee<T>>::put(val);
        Self::deposit_event(RawEvent::SetWithdrawalFee(val));
        Ok(())
    }
}

impl<T: Trait> Module<T> {
    /// get the record list for a account
    pub fn records_of(who: &T::AccountId) -> Vec<RecordT<T>> {
        let mut records: Vec<RecordT<T>> = Vec::new();
        let len: u32 = Self::records_len_of(who);
        for i in 0..len {
            if let Some(r) = <RecordsOf<T>>::get(who.clone(), i) {
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
            <RecordsOf<T>>::get(who.clone(), index).map(|r| (index, r))
        }
    }
    /// insert a new record for a account, notice the record state must be init state
    fn new_record(who: &T::AccountId, record: &RecordT<T>) -> Result<u32, &'static str> {
        if !record.is_init() {
            return Err("new record should be Invalid state first");
        }
        let len: u32 = Self::records_len_of(who);
        <RecordsOf<T>>::insert(who.clone(), len, *record);
        <RecordsLenOf<T>>::insert(who, len + 1);  // len is more than 1 to max index
        Ok(len)
    }
}

impl<T: Trait> Module<T> {
    /// deposit/withdrawal pre-process
    fn before(who: &T::AccountId, is_withdrawal: bool) -> Result<(), &'static str> {
        match Self::last_record_of(who) {
            Some((_, record)) => {
                if !record.is_finish() {
                    return Err("the last action have not finished yet! only if the last deposit/withdrawal have finished you can do a new action.");
                }
            }
            None => {
                if is_withdrawal {
                    return Err("the account has not deposit record yet")
                }
            }
        }
        Ok(())
    }
    /// deposit, notice this func has include deposit_init and deposit_finish (not wait for block confirm process)
    pub fn deposit(who: &T::AccountId, sym: &T::Symbol, balance: T::TokenBalance) -> DispatchResult {
        let index = Self::deposit_with_index(who, sym, balance)?;
        Self::deposit_finish_with_index(who, index, true).map(|_| ())
    }
    /// withdrawal, notice this func has include withdrawal_init and withdrawal_locking
    pub fn withdrawal(who: &T::AccountId, sym: &T::Symbol, balance: T::TokenBalance) -> DispatchResult {
        let index = Self::withdrawal_with_index(who, sym, balance)?;
        Self::withdrawal_locking_with_index(who, index).map(|_| ())
    }
    /// withdrawal finish, let the locking token destroy
    pub fn withdrawal_finish(who: &T::AccountId, success: bool) -> DispatchResult {
        let last_index = Self::records_len_of(who) - 1;
        Self::withdrawal_finish_with_index(who, last_index, success).map(|_| ())
    }
    /// deposit init, use for record a deposit process begin, usually for start block confirm process
    pub fn deposit_init(who: &T::AccountId, sym: &T::Symbol, balance: T::TokenBalance) -> DispatchResult {
        Self::deposit_with_index(who, sym, balance).map(|_| ())
    }
    /// deposit finish, use for change the deposit record to final, success mark the deposit if success
    pub fn deposit_finish(who: &T::AccountId, success: bool) -> DispatchResult {
        let last_index = Self::records_len_of(who) - 1;
        Self::deposit_finish_with_index(who, last_index, success).map(|_| ())
    }
    /// withdrawal init, use for record a withdrawal start, should call withdrawal_locking after it
    pub fn withdrawal_init(who: &T::AccountId, sym: &T::Symbol, balance: T::TokenBalance) -> DispatchResult {
        Self::withdrawal_with_index(who, sym, balance).map(|_| ())
    }
    /// change the free token to locking state
    pub fn withdrawal_locking(who: &T::AccountId) -> DispatchResult {
        let last_index = Self::records_len_of(who) - 1;
        Self::withdrawal_locking_with_index(who, last_index).map(|_| ())
    }
    /// deposit init, notice this func return index to show the index of records for this account
    pub fn deposit_with_index(who: &T::AccountId, sym: &T::Symbol, balance: T::TokenBalance) -> Result<u32, &'static str> {
        Self::before(who, false)?;

        <tokenbalances::Module<T>>::is_valid_token(sym)?;

        let mut index = 0_u32;
        <tokenbalances::Module<T>>::handle_fee(who, Self::deposit_fee(), true, || {
            let r = Record { action: Action::Deposit(Default::default()), symbol: *sym, balance: balance, init_blocknum: <system::Module<T>>::block_number() };
            index = Self::new_record(who, &r)?;
            Self::deposit_event(RawEvent::DepositInit(who.clone(), index, r.symbol(), r.balance(), r.blocknum()));
            Ok(())
        })?;

        Ok(index)
    }
    /// deposit finish, should use index to find the old deposit record, success flag mark the success
    pub fn deposit_finish_with_index(who: &T::AccountId, index: u32, success: bool) -> Result<u32, &'static str> {
        if let Some(ref mut r) = <RecordsOf<T>>::get(who.clone(), index) {
            if r.is_finish() {
                return Err("the deposit record should not be a finish state");
            }

            let sym = r.symbol();
            let bal = r.balance();
            // change state
            match r.mut_action() {
                Action::Deposit(ref mut state) => {
                    if success {
                        *state = DepositState::Success;
                        // call tokenbalances to issue token for this accountid
                        <tokenbalances::Module<T>>::issue(who, &sym, bal)?;

                        Self::deposit_event(RawEvent::DepositSuccess(who.clone(), index, sym, bal, <system::Module<T>>::block_number()));
                    } else {
                        *state = DepositState::Failed;

                        Self::deposit_event(RawEvent::DepositFailed(who.clone(), index, sym, bal, <system::Module<T>>::block_number()));
                    }
                }
                _ => return Err("err action type in deposit_finish"),
            }
            <RecordsOf<T>>::insert(who.clone(), index, *r);
            Ok(index)
        } else {
            return Err("the deposit record for this (accountid, index) not exist");
        }
    }
    /// withdrawal init, notice this func return index to show the index of records for this account
    pub fn withdrawal_with_index(who: &T::AccountId, sym: &T::Symbol, balance: T::TokenBalance) -> Result<u32, &'static str> {
        Self::before(who, true)?;

        <tokenbalances::Module<T>>::is_valid_token_for(who, sym)?;
        // check token balance
        if <tokenbalances::Module<T>>::free_token_of(who, sym) < balance {
            return Err("not enough free token to withdraw");
        }

        let mut index = 0_u32;
        <tokenbalances::Module<T>>::handle_fee(who, Self::withdrawal_fee(), true, || {
            let r = Record { action: Action::Withdrawal(Default::default()), symbol: *sym, balance: balance, init_blocknum: <system::Module<T>>::block_number() };
            index = Self::new_record(who, &r)?;
            Self::deposit_event(RawEvent::WithdrawalInit(who.clone(), index, r.symbol(), r.balance(), r.blocknum()));
            Ok(())
        })?;

        Ok(index)
    }
    /// withdrawal lock, should use index to find out which record to change to locking state
    pub fn withdrawal_locking_with_index(who: &T::AccountId, index: u32) -> Result<u32, &'static str> {
        if let Some(ref mut r) = <RecordsOf<T>>::get(who.clone(), index) {
            if r.is_finish() {
                return Err("the deposit record should not be a finish state");
            }

            let sym = r.symbol();
            let bal = r.balance();
            // change state
            match r.mut_action() {
                Action::Withdrawal(ref mut state) => {
                    match state {
                        WithdrawalState::Invalid => {
                            *state = WithdrawalState::Locking;

                            <tokenbalances::Module<T>>::lock_destroy_token(who, &sym, bal)?;

                            Self::deposit_event(RawEvent::WithdrawalLocking(who.clone(), index, sym, bal, <system::Module<T>>::block_number()));
                        }
                        _ => return Err("the withdrawal state must be Invalid."),
                    }
                }
                _ => return Err("err action type in deposit_finish"),
            }
            <RecordsOf<T>>::insert(who.clone(), index, *r);

            Ok(index)
        } else {
            return Err("the withdrawal record for this (accountid, index) not exist");
        }
    }
    /// withdrawal finish, should use index to find out which record to changed to final, success flag mark success, if false, release the token to free
    pub fn withdrawal_finish_with_index(who: &T::AccountId, index: u32, success: bool) -> Result<u32, &'static str> {
        if let Some(ref mut r) = <RecordsOf<T>>::get(who.clone(), index) {
            if r.is_finish() {
                return Err("the deposit record should not be a finish state");
            }

            let sym = r.symbol();
            let bal = r.balance();

            // change state
            match r.mut_action() {
                Action::Withdrawal(ref mut state) => {
                    match state {
                        WithdrawalState::Locking => {
                            if success {
                                *state = WithdrawalState::Success;

                                <tokenbalances::Module<T>>::destroy(who, &sym, bal)?;

                                Self::deposit_event(RawEvent::WithdrawalSuccess(who.clone(), index, sym, bal, <system::Module<T>>::block_number()));
                            } else {
                                *state = WithdrawalState::Failed;

                                <tokenbalances::Module<T>>::unlock_destroy_token(who, &sym, bal)?;

                                Self::deposit_event(RawEvent::WithdrawalFailed(who.clone(), index, sym, bal, <system::Module<T>>::block_number()));
                            }
                        }
                        _ => return Err("the withdrawal state must be Locking."),
                    }
                }
                _ => return Err("err action type in deposit_finish")
            }
            <RecordsOf<T>>::insert(who.clone(), index, *r);

            Ok(index)
        } else {
            return Err("the withdrawal record for this (accountid, index) not exist");
        }
    }
}


#[cfg(feature = "std")]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
/// The genesis block configuration type. This is a simple default-capable struct that
/// contains any fields with which this module can be configured at genesis time.
pub struct GenesisConfig<T: Trait> {
    pub deposit_fee: T::Balance,
    pub withdrawal_fee: T::Balance,
}

#[cfg(feature = "std")]
impl<T: Trait> Default for GenesisConfig<T> {
    fn default() -> Self {
        GenesisConfig {
            deposit_fee: Default::default(),
            withdrawal_fee: Default::default(),
        }
    }
}

#[cfg(feature = "std")]
impl<T: Trait> runtime_primitives::BuildStorage for GenesisConfig<T>
{
    fn build_storage(self) -> ::std::result::Result<runtime_primitives::StorageMap, String> {
        use codec::Encode;
        Ok(map![
            Self::hash(<DepositFee<T>>::key()).to_vec() => self.deposit_fee.encode(),
            Self::hash(<WithdrawalFee<T>>::key()).to_vec() => self.withdrawal_fee.encode()
        ])
    }
}
