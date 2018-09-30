// Copyright 2018 Chainpool.
//! TokenBalances: Handles token symbol balances.

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
extern crate serde;

#[cfg(feature = "std")]
#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate srml_support as runtime_support;

// Needed for tests (`with_externalities`).
#[cfg_attr(feature = "std", macro_use)]
extern crate sr_std as rstd;

#[macro_use]
extern crate parity_codec_derive;
extern crate parity_codec as codec;

extern crate substrate_primitives;
extern crate sr_io as runtime_io;
extern crate sr_primitives as primitives;
extern crate srml_system as system;
extern crate srml_balances as balances;

extern crate cxrml_support as cxrt_support;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub mod utils;

use rstd::prelude::*;
use codec::Codec;
use runtime_support::{StorageValue, StorageMap, Parameter};
use runtime_support::dispatch::Result;
use primitives::traits::{SimpleArithmetic, As, Member, CheckedAdd, CheckedSub, OnFinalise};

use cxrt_support::StorageDoubleMap;

// substrate mod
use system::ensure_signed;
use balances::address::Address;
use balances::EnsureAccountLiquid;

pub trait Trait: balances::Trait {
    /// The token balance.
    type TokenBalance: Parameter + Member + Codec + SimpleArithmetic + As<u8> + As<u16> + As<u32> + As<u64> + As<u128> + As<usize> + Copy + Default;
    /// The token precision, for example, btc, 1BTC=1000mBTC=1000000Bits, and decide a precision for btc
    type Precision: Parameter + Member + Codec + As<u8> + As<u16> + As<u32> + As<usize> + Copy + Default;
    /// the token description, better to put precision desc here
    type TokenDesc: Parameter + Member + Codec + Copy + Default;
    /// the token symbol
    type Symbol: Parameter + Member + Codec + Copy + Default;
    /// Event
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

/// Token struct.
#[derive(PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub struct Token<Symbol, TokenDesc, Precision> where
    Symbol: Parameter + Copy + Default,
    TokenDesc: Parameter + Copy + Default,
    Precision: As<u8> + As<u16> + As<u32> + As<usize> + Copy,
{
    /// Validator should ensure this many more slashes than is necessary before being unstaked.
    pub symbol: Symbol,
    /// token description
    token_desc: TokenDesc,
    /// token balance precision
    precision: Precision,
}

impl<Symbol, TokenDesc, Precision> Token<Symbol, TokenDesc, Precision> where
    Symbol: Parameter + Copy + Default,
    TokenDesc: Parameter + Copy + Default,
    Precision: As<u8> + As<u16> + As<u32> + As<usize> + Copy,
{
    pub fn new(symbol: Symbol, token_desc: TokenDesc, precision: Precision) -> Token<Symbol, TokenDesc, Precision> {
        Token { symbol, token_desc, precision }
    }

    pub fn precision(&self) -> Precision {
        self.precision
    }

    pub fn token_desc(&self) -> TokenDesc {
        self.token_desc
    }

    pub fn set_token_desc(&mut self, desc: &TokenDesc) {
        self.token_desc = desc.clone();
    }
}

pub type TokenT<T> = Token<<T as Trait>::Symbol, <T as Trait>::TokenDesc, <T as Trait>::Precision>;

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        /// register_token to module, should allow by root
        fn register_token(token: Token<T::Symbol, T::TokenDesc, T::Precision>, free: T::TokenBalance, locked: T::TokenBalance) -> Result;
        /// transfer between account
        fn transfer_token(origin, dest: Address<T::AccountId, T::AccountIndex>, sym: T::Symbol, value: T::TokenBalance) -> Result;

        fn set_transfer_token_fee(val: T::Balance) -> Result;
    }
}

decl_event!(
    pub enum Event<T> where
        <T as system::Trait>::AccountId,
        <T as Trait>::TokenBalance,
        <T as Trait>::Symbol,
        <T as Trait>::TokenDesc,
        <T as Trait>::Precision,
        <T as balances::Trait>::Balance
    {
        /// register new token (token.symbol, token.token_desc, token.precision)
        RegisterToken(Symbol, TokenDesc, Precision),
        /// cancel token
        CancelToken(Symbol),
        /// issue succeeded (who, symbol, balance)
        IssueToken(AccountId, Symbol, TokenBalance),
        /// lock destroy (who, symbol, balance)
        LockToken(AccountId, Symbol, TokenBalance),
        /// unlock destroy (who, symbol, balance)
        UnlockToken(AccountId, Symbol, TokenBalance),
        /// destroy
        DestroyToken(AccountId, Symbol, TokenBalance),
        /// Transfer succeeded (from, to, symbol, value, fees).
        TransferToken(AccountId, AccountId, Symbol, TokenBalance, Balance),
        /// set transfer token fee
        SetTransferTokenFee(Balance),
    }
);

decl_storage! {
    trait Store for Module<T: Trait> as TokenBalances {
        /// supported token list
        pub TokenListMap get(token_list_map): default map [u32 => (bool, T::Symbol)];
        /// supported token list length
        pub TokenListLen get(token_list_len): default u32;
        /// token info for every token, key is token symbol
        pub TokenInfo get(token_info): default map [T::Symbol => TokenT<T>];
        /// total free token of a symbol
        pub TotalFreeToken get(total_free_token): default map [T::Symbol => T::TokenBalance];
        /// total locked token of a symbol
        pub TotalLockedToken get(total_locked_token): default map [T::Symbol => T::TokenBalance];

        /// token list of a account
        pub TokenListOf get(token_list_of): default map [T::AccountId => Vec<T::Symbol>];

        /// transfer token fee
        pub TransferTokenFee get(transfer_token_fee): required T::Balance;
    }
}

// account token storage
pub(crate) struct FreeTokenOf<T>(::rstd::marker::PhantomData<T>);

pub(crate) struct LockedTokenOf<T>(::rstd::marker::PhantomData<T>);

impl<T: Trait> StorageDoubleMap for FreeTokenOf<T> {
    type Key1 = T::AccountId;
    type Key2 = T::Symbol;
    type Value = T::TokenBalance;
    const PREFIX: &'static [u8] = b"TokenBalances FreeTokenOf";
}

impl<T: Trait> StorageDoubleMap for LockedTokenOf<T> {
    type Key1 = T::AccountId;
    type Key2 = T::Symbol;
    type Value = T::TokenBalance;
    const PREFIX: &'static [u8] = b"TokenBalances LockedTokenOf";
}

// This trait expresses what should happen when the block is finalised.
impl<T: Trait> OnFinalise<T::BlockNumber> for Module<T> {
    fn on_finalise(_: T::BlockNumber) {
        // do nothing
    }
}

impl<T: Trait> Module<T> {
    /// Deposit one of this module's events.
    fn deposit_event(event: Event<T>) {
        <system::Module<T>>::deposit_event(<T as Trait>::Event::from(event).into());
    }
}

impl<T: Trait> Module<T> {
    // token storage
    pub fn free_token_of(who: &T::AccountId, symbol: &T::Symbol) -> T::TokenBalance {
        <FreeTokenOf<T>>::get(who.clone(), symbol.clone()).unwrap_or_default()
    }

    pub fn locked_token_of(who: &T::AccountId, symbol: &T::Symbol) -> T::TokenBalance {
        <LockedTokenOf<T>>::get(who.clone(), symbol.clone()).unwrap_or_default()
    }

    /// The combined token balance of `who` for symbol.
    pub fn total_token_of(who: &T::AccountId, symbol: &T::Symbol) -> T::TokenBalance {
        Self::free_token_of(who, symbol) + Self::locked_token_of(who, symbol)
    }

    /// tatal_token of a token symbol
    pub fn total_token(symbol: &T::Symbol) -> T::TokenBalance {
        Self::total_free_token(symbol) + Self::total_locked_token(symbol)
    }
}

impl<T: Trait> Module<T> {
    // token symol
    // public call
    /// register a token into token list ans init
    pub fn register_token(token: TokenT<T>, free: T::TokenBalance, locked: T::TokenBalance) -> Result {
        Self::add_token(&token.symbol, free, locked)?;
        <TokenInfo<T>>::insert(token.symbol, token.clone());

        Self::deposit_event(RawEvent::RegisterToken(token.symbol, token.token_desc(), token.precision()));
        Ok(())
    }
    /// cancel a token from token list but not remove it
    pub fn cancel_token(symbol: &T::Symbol) -> Result {
        Self::remove_token(symbol)?;

        Self::deposit_event(RawEvent::CancelToken(*symbol));
        Ok(())
    }

    /// retuan all token list with valid flag
    pub fn all_token_list() -> Vec<(bool, T::Symbol)> {
        let len: u32 = <TokenListLen<T>>::get();
        let mut v: Vec<(bool, T::Symbol)> = Vec::new();
        for i in 0..len {
            let (flag, symbol) = <TokenListMap<T>>::get(i);
            if symbol != Default::default() {
                v.push((flag, symbol));
            }
        }
        v
    }

    /// return valid token list, only valid token
    pub fn token_list() -> Vec<T::Symbol> {
        Self::all_token_list().into_iter()
            .filter(|(flag, _)| *flag == true)
            .map(|(_, sym)| sym)
            .collect()
    }

    pub fn is_valid_token(symbol: &T::Symbol) -> Result {
        if Self::token_list().contains(symbol) {
            Ok(())
        } else {
            Err("not in the valid token list")
        }
    }

    pub fn is_valid_token_for(who: &T::AccountId, symbol: &T::Symbol) -> Result {
        if Self::token_list_of(who).contains(symbol) {
            Ok(())
        } else {
            Err("not a existed token in this account token list")
        }
    }

    fn add_token(symbol: &T::Symbol, free: T::TokenBalance, locked: T::TokenBalance) -> Result {
        let list = Self::all_token_list();
        if !list.iter().find(|(_, sym)| *sym == *symbol).is_none() {
            return Err("already has this token symbol");
        }

        let len: u32 = <TokenListLen<T>>::get();
        // mark new symbol valid
        <TokenListMap<T>>::insert(len, (true, symbol.clone()));
        <TokenListLen<T>>::put(len + 1);

        Self::init_token_balance(symbol, free, locked);

        Ok(())
    }

    fn remove_token(symbol: &T::Symbol) -> Result {
        let list = Self::token_list();

        let index = if let Some(i) = list.iter().position(|sym| *sym == *symbol) {
            i
        } else {
            return Err("this token symbol dose not register yet or is invalid");
        };

        <TokenListMap<T>>::mutate(index as u32, |value| {
            let (ref mut flag, _) = *value;
            *flag = false;
        });

        // do not remove token balance from storage
        // Self::remove_token_balance();

        Ok(())
    }

    fn init_token_balance(symbol: &T::Symbol, free: T::TokenBalance, locked: T::TokenBalance) {
        <TotalFreeToken<T>>::insert(symbol, free);
        <TotalLockedToken<T>>::insert(symbol, locked);
    }

    #[allow(dead_code)]
    fn remove_token_balance(symbol: &T::Symbol) {
        <TotalFreeToken<T>>::remove(symbol);
        <TotalLockedToken<T>>::remove(symbol);
    }
}

impl<T: Trait> Module<T> {
    fn init_token_for(who: &T::AccountId, symbol: &T::Symbol) {
        if let Err(_) = Self::is_valid_token_for(who, symbol) {
            <TokenListOf<T>>::mutate(who, |token_list| token_list.push(symbol.clone()));
        }
    }

    /// issue from real coin to chainx token, notice it become free token directly
    pub fn issue(who: &T::AccountId, symbol: &T::Symbol, balance: T::TokenBalance) -> Result {
        Self::is_valid_token(symbol)?;

        <T as balances::Trait>::EnsureAccountLiquid::ensure_account_liquid(who)?;

        // increase for all, overflow would exist at this point
        Self::increase_total_free_token_by(symbol, balance)?;

        // init for account
        Self::init_token_for(who, symbol);
        // increase for this account
        Self::increase_account_free_token_by(who, symbol, balance)?;

        Self::deposit_event(RawEvent::IssueToken(who.clone(), symbol.clone(), balance));
        Ok(())
    }

    /// destroy token must be lock first, and become locked state
    pub fn lock_destroy_token(who: &T::AccountId, symbol: &T::Symbol, balance: T::TokenBalance) -> Result {
        Self::is_valid_token(symbol)?;
        Self::is_valid_token_for(who, symbol)?;

        <T as balances::Trait>::EnsureAccountLiquid::ensure_account_liquid(who)?;

        // for all token
        if Self::total_free_token(symbol) < balance {
            return Err("not enough free token to lock");
        }
        // for account
        if Self::free_token_of(who, symbol) < balance {
            return Err("not enough free token to lock for this account");
        }
        // modify store
        // for all token
        // would exist if overflow
        Self::decrease_total_free_token_by(symbol, balance)?;
        Self::increase_total_locked_token_by(symbol, balance)?;
        // for account
        Self::decrease_account_free_token_by(who, symbol, balance)?;
        Self::increase_account_locked_token_by(who, symbol, balance)?;

        Self::deposit_event(RawEvent::LockToken(who.clone(), symbol.clone(), balance));
        Ok(())
    }

    /// unlock locked token if destroy failed
    pub fn unlock_destroy_token(who: &T::AccountId, symbol: &T::Symbol, balance: T::TokenBalance) -> Result {
        Self::is_valid_token(symbol)?;
        Self::is_valid_token_for(who, symbol)?;

        <T as balances::Trait>::EnsureAccountLiquid::ensure_account_liquid(who)?;

        // for all token
        if Self::total_locked_token(symbol) < balance {
            return Err("not enough locked token to unlock");
        }
        // for account
        if Self::locked_token_of(who, symbol) < balance {
            return Err("not enough locked token to lock for this account");
        }
        // modify store
        // for all token
        // would exist if overflow
        Self::decrease_total_locked_token_by(symbol, balance)?;
        Self::increase_total_free_token_by(symbol, balance)?;
        // for account
        Self::decrease_account_locked_token_by(who, symbol, balance)?;
        Self::increase_account_free_token_by(who, symbol, balance)?;

        Self::deposit_event(RawEvent::UnlockToken(who.clone(), symbol.clone(), balance));
        Ok(())
    }

    /// real destroy token, only decrease in account locked token
    pub fn destroy(who: &T::AccountId, symbol: &T::Symbol, balance: T::TokenBalance) -> Result {
        Self::is_valid_token(symbol)?;
        Self::is_valid_token_for(who, symbol)?;

        <T as balances::Trait>::EnsureAccountLiquid::ensure_account_liquid(who)?;

        // for all token
        if Self::total_locked_token(symbol) < balance {
            return Err("not enough locked token to destroy");
        }
        // for account
        if Self::locked_token_of(who, symbol) < balance {
            return Err("not enough locked token to destroy for this account");
        }
        // destroy token
        // for all token
        // would exist if overflow
        Self::decrease_total_locked_token_by(symbol, balance)?;
        // for account
        Self::decrease_account_locked_token_by(who, symbol, balance)?;

        Self::deposit_event(RawEvent::DestroyToken(who.clone(), symbol.clone(), balance));
        Ok(())
    }

    // token calc
    /// Increase TotalFreeToken by Value.
    fn increase_total_free_token_by(symbol: &T::Symbol, value: T::TokenBalance) -> Result {
        if let Some(v) = Self::total_free_token(symbol).checked_add(&value) {
            <TotalFreeToken<T>>::mutate(symbol, |b: &mut T::TokenBalance| {
                *b = v;
            });
            Ok(())
        } else {
            Err("Overflow in increase_total_free_token_by")
        }
    }
    /// Decrease TotalFreeToken by Value.
    fn decrease_total_free_token_by(symbol: &T::Symbol, value: T::TokenBalance) -> Result {
        if let Some(v) = Self::total_free_token(symbol).checked_sub(&value) {
            <TotalFreeToken<T>>::mutate(symbol, |b: &mut T::TokenBalance| {
                *b = v;
            });
            Ok(())
        } else {
            Err("Overflow in decrease_total_free_token_by")
        }
    }

    /// Increase TotalLockedToken by Value.
    fn increase_total_locked_token_by(symbol: &T::Symbol, value: T::TokenBalance) -> Result {
        if let Some(v) = Self::total_locked_token(symbol).checked_add(&value) {
            <TotalLockedToken<T>>::mutate(symbol, |b: &mut T::TokenBalance| {
                *b = v;
            });
            Ok(())
        } else {
            Err("Overflow in increase_total_locked_token_by")
        }
    }
    /// Decrease TotalLockedToken by Value.
    fn decrease_total_locked_token_by(symbol: &T::Symbol, value: T::TokenBalance) -> Result {
        if let Some(v) = Self::total_locked_token(symbol).checked_sub(&value) {
            <TotalLockedToken<T>>::mutate(symbol, |b: &mut T::TokenBalance| {
                *b = v;
            });
            Ok(())
        } else {
            Err("Overflow in decrease_total_locked_token_by")
        }
    }

    /// Increase FreeToken balance to account for a symbol by Value.
    fn increase_account_free_token_by(who: &T::AccountId, symbol: &T::Symbol, value: T::TokenBalance) -> Result {
        let b: T::TokenBalance = Self::free_token_of(who, symbol);
        if let Some(v) = b.checked_add(&value) {
            <FreeTokenOf<T>>::insert(who.clone(), symbol.clone(), v);
            Ok(())
        } else {
            Err("Overflow in increase_account_free_token_by for account")
        }
    }
    /// Decrease FreeToken balance to account for a symbol by Value.
    fn decrease_account_free_token_by(who: &T::AccountId, symbol: &T::Symbol, value: T::TokenBalance) -> Result {
        let b: T::TokenBalance = Self::free_token_of(who, symbol);
        if let Some(v) = b.checked_sub(&value) {
            <FreeTokenOf<T>>::insert(who.clone(), symbol.clone(), v);
            Ok(())
        } else {
            Err("Overflow in decrease_account_free_token_by for account")
        }
    }
    /// Increase LockedToken balance to account for a symbol by Value.
    fn increase_account_locked_token_by(who: &T::AccountId, symbol: &T::Symbol, value: T::TokenBalance) -> Result {
        let b: T::TokenBalance = Self::locked_token_of(who, symbol);
        if let Some(v) = b.checked_add(&value) {
            <LockedTokenOf<T>>::insert(who.clone(), symbol.clone(), v);
            Ok(())
        } else {
            Err("Overflow in increase_account_locked_token_by for account")
        }
    }
    /// Decrease LockedToken balance to account for a symbol by Value.
    fn decrease_account_locked_token_by(who: &T::AccountId, symbol: &T::Symbol, value: T::TokenBalance) -> Result {
        let b: T::TokenBalance = Self::locked_token_of(who, symbol);
        if let Some(v) = b.checked_sub(&value) {
            <LockedTokenOf<T>>::insert(who.clone(), symbol.clone(), v);
            Ok(())
        } else {
            Err("Overflow in decrease_account_locked_token_by for account")
        }
    }
}

impl<T: Trait> Module<T> {
    // public call
    /// transfer token between accountid, notice the fee is chainx
    pub fn transfer_token(origin: T::Origin, dest: balances::Address<T>, sym: T::Symbol, value: T::TokenBalance) -> Result {
        let transactor = ensure_signed(origin)?;
        let dest = <balances::Module<T>>::lookup(dest)?;

        Self::is_valid_token(&sym)?;
        Self::is_valid_token_for(&transactor, &sym)?;
        // Self::is_valid_token_for(&dest, &sym)?;
        Self::init_token_for(&dest, &sym);

        let fee = Self::transfer_token_fee();
        let sender = &transactor;
        let receiver = &dest;
        Self::handle_fee(sender, fee, true, || {
            if Self::free_token_of(&sender, &sym) < value {
                return Err("transactor's free token balance too low, can't transfer this token");
            }
            if sender != receiver {
                Self::decrease_account_free_token_by(sender, &sym, value)?;
                Self::increase_account_free_token_by(receiver, &sym, value)?;
                Self::deposit_event(RawEvent::TransferToken(sender.clone(), receiver.clone(), sym, value, fee));
            }
            Ok(())
        })?;

        Ok(())
    }

    pub fn set_transfer_token_fee(val: T::Balance) -> Result {
        <TransferTokenFee<T>>::put(val);
        Self::deposit_event(RawEvent::SetTransferTokenFee(val));
        Ok(())
    }
}

impl<T: Trait> Module<T> {
    /// handle the fee with the func
    pub fn handle_fee<F>(who: &T::AccountId, fee: T::Balance, check_after_open: bool, mut func: F) -> Result
        where F: FnMut() -> Result
    {
        let from_balance = <balances::Module<T>>::free_balance(who);
        let new_from_balance = match from_balance.checked_sub(&fee) {
            Some(b) => b,
            None => return Err("chainx balance too low to exec this option"),
        };
        <T as balances::Trait>::EnsureAccountLiquid::ensure_account_liquid(who)?;
        if check_after_open && new_from_balance < <balances::Module<T>>::existential_deposit() {
            return Err("chainx balance is not enough after this tx, not allow to be killed at here");
        }

        let ret = func()?;

        // deduct free
        <balances::Module<T>>::set_free_balance(who, new_from_balance);
        Ok(ret)
    }
}

#[cfg(feature = "std")]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
/// The genesis block configuration type. This is a simple default-capable struct that
/// contains any fields with which this module can be configured at genesis time.
pub struct GenesisConfig<T: Trait> {
    /// A value with which to initialise the Dummy storage item.
    pub token_list: Vec<(TokenT<T>, T::TokenBalance, T::TokenBalance)>,

    pub transfer_token_fee: T::Balance,
}

#[cfg(feature = "std")]
impl<T: Trait> Default for GenesisConfig<T> {
    fn default() -> Self {
        GenesisConfig {
            token_list: Default::default(),
            transfer_token_fee: Default::default(),
        }
    }
}


#[cfg(feature = "std")]
impl<T: Trait> primitives::BuildStorage for GenesisConfig<T>
{
    fn build_storage(self) -> ::std::result::Result<primitives::StorageMap, String> {
        use codec::Encode;

        let mut r: primitives::StorageMap = map![];
        // token list
        // 0 token list length
        r.insert(Self::hash(<TokenListLen<T>>::key()).to_vec(), self.token_list.len().encode());
        for (index, (token, free_token, locked_token)) in self.token_list.into_iter().enumerate() {
            // 1 token balance
            r.insert(Self::hash(&<TotalFreeToken<T>>::key_for(token.symbol)).to_vec(), free_token.encode());
            r.insert(Self::hash(&<TotalLockedToken<T>>::key_for(token.symbol)).to_vec(), locked_token.encode());
            // 2 token info
            r.insert(Self::hash(&<TokenInfo<T>>::key_for(token.symbol)).to_vec(), token.encode());
            // 3 token list map
            r.insert(Self::hash(&<TokenListMap<T>>::key_for(index as u32)).to_vec(), (true, token.symbol).encode());
        }
        // transfer token fee
        r.insert(Self::hash(<TransferTokenFee<T>>::key()).to_vec(), self.transfer_token_fee.encode());

        Ok(r)
    }
}
