// Copyright 2018 Chainpool.

//! this module is for tokenstaking, virtual mining for holding tokens

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
#[cfg(feature = "std")]
extern crate substrate_primitives;

// for substrate runtime
// map!, vec! marco.
extern crate sr_std as rstd;
// Needed for tests (`with_externalities`).
extern crate sr_io as runtime_io;
extern crate sr_primitives as runtime_primitives;
// for substrate runtime module lib
// Needed for type-safe access to storage DB.
#[macro_use]
extern crate srml_support as runtime_support;
extern crate srml_balances as balances;
extern crate srml_system as system;
extern crate srml_timestamp as timestamp;

#[cfg(test)]
extern crate cxrml_associations as associations;
extern crate cxrml_exchange_pendingorders as pendingorders;
extern crate cxrml_funds_financialrecords as financialrecords;
extern crate cxrml_mining_staking as staking;
extern crate cxrml_support as cxsupport;
#[cfg(test)]
extern crate cxrml_system as cxsystem;
extern crate cxrml_tokenbalances as tokenbalances;

extern crate cxrml_bridge_btc as btc;

#[cfg(test)]
mod tests;

//use codec::{Codec, Decode, Encode};
//use rstd::marker::PhantomData;
use rstd::prelude::*;
//use rstd::result::Result as StdResult;
use runtime_primitives::traits::{As, CheckedAdd, CheckedSub, OnFinalise, Zero};
use runtime_support::dispatch::Result;
use runtime_support::{StorageMap, StorageValue};

use system::ensure_signed;

use financialrecords::{OnDepositToken, OnWithdrawToken};
use pendingorders::OrderPair;
use staking::{Jackpot, OnNewSessionForTokenStaking, OnReward, Validator, VoteWeight};
use tokenbalances::{OnMoveToken, Symbol, TokenT};

pub trait Trait:
    system::Trait
    + timestamp::Trait
    + balances::Trait
    + tokenbalances::Trait
    + staking::Trait
    + pendingorders::Trait
    + btc::Trait
{
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

/// Profile of virtual intention
#[derive(PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub struct VirtualIntentionProfs<Balance: Default, BlockNumber: Default> {
    pub jackpot: Balance,
    pub last_total_weight: u128,
    pub last_total_weight_update: BlockNumber,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub struct HodlingRecord<BlockNumber>
where
    BlockNumber: Default + As<u64> + Copy + Clone,
{
    pub last_weight: u128,
    pub last_weight_update: BlockNumber,
}

pub struct IntentionProfsWrapper<'a, T: Trait> {
    pub sym: Symbol,
    pub profs: &'a mut VirtualIntentionProfs<T::Balance, T::BlockNumber>,
}

impl<'a, T: Trait> VoteWeight<T::BlockNumber> for IntentionProfsWrapper<'a, T> {
    fn amount(&self) -> u128 {
        tokenbalances::Module::<T>::total_token(&self.sym).as_()
    }

    fn last_acum_weight(&self) -> u128 {
        self.profs.last_total_weight
    }

    fn last_acum_weight_update(&self) -> u128 {
        self.profs.last_total_weight_update.as_() as u128
    }

    fn set_amount(&mut self, _: u128, _: bool) {}

    fn set_last_acum_weight(&mut self, latest_deposit_weight: u128) {
        self.profs.last_total_weight = latest_deposit_weight;
    }

    fn set_last_acum_weight_update(&mut self, current_block: T::BlockNumber) {
        self.profs.last_total_weight_update = current_block;
    }
}

impl<'a, T: Trait> Jackpot<T::Balance> for IntentionProfsWrapper<'a, T> {
    fn jackpot(&self) -> T::Balance {
        self.profs.jackpot
    }

    fn set_jackpot(&mut self, value: T::Balance) {
        self.profs.jackpot = value;
    }
}

pub struct HodlingRecordWrapper<'a, T: Trait> {
    pub sym: Symbol,
    pub account: T::AccountId,
    pub record: &'a mut HodlingRecord<T::BlockNumber>,
}

impl<'a, T: Trait> VoteWeight<T::BlockNumber> for HodlingRecordWrapper<'a, T> {
    fn amount(&self) -> u128 {
        tokenbalances::Module::<T>::total_token_of(&self.account, &self.sym).as_()
    }

    fn last_acum_weight(&self) -> u128 {
        self.record.last_weight
    }

    fn last_acum_weight_update(&self) -> u128 {
        self.record.last_weight_update.clone().as_() as u128
    }

    fn set_amount(&mut self, _: u128, _: bool) {}

    fn set_last_acum_weight(&mut self, latest_vote_weight: u128) {
        self.record.last_weight = latest_vote_weight;
    }

    fn set_last_acum_weight_update(&mut self, current_block: T::BlockNumber) {
        self.record.last_weight_update = current_block;
    }
}

decl_event!(
    pub enum Event<T> where
        <T as system::Trait>::AccountId
    {
        TokenRewardClaim(AccountId, Symbol),
    }
);

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn claim(origin, sym: Symbol) -> Result;
    }
}

impl<T: Trait> OnFinalise<T::BlockNumber> for Module<T> {
    fn on_finalise(_: T::BlockNumber) {
        // do nothing
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as TokenStkaing {
        pub VirtualProfsFor get(virtual_profs_for): map Symbol => VirtualIntentionProfs<T::Balance, T::BlockNumber>;
        pub HodlingRecordFor get(hodling_record_for): map (T::AccountId, Symbol) => HodlingRecord<T::BlockNumber>;

        pub DiscountRatioFor get(discount_ratio_for): map Symbol => (u32, u32) = (1, 2);

        pub Fee get(fee) config(): T::Balance;
    }
}

impl<T: Trait> Module<T> {
    // event
    /// Deposit one of this module's events.
    fn deposit_event(event: Event<T>) {
        <system::Module<T>>::deposit_event(<T as Trait>::Event::from(event).into());
    }
}

impl<T: Trait> Module<T> {
    fn claim(origin: T::Origin, sym: Symbol) -> Result {
        let who = ensure_signed(origin)?;
        cxsupport::Module::<T>::handle_fee_before(&who, Self::fee(), true, || Ok(()))?;

        let mut profs = Module::<T>::virtual_profs_for(&sym);
        let key = (who.clone(), sym.clone());
        let mut record = Module::<T>::hodling_record_for(&key);

        {
            let mut iprofs = IntentionProfsWrapper::<T> {
                sym: sym.clone(),
                profs: &mut profs,
            };
            let mut hodling = HodlingRecordWrapper::<T> {
                sym: sym.clone(),
                account: who.clone(),
                record: &mut record,
            };
            staking::Module::<T>::generic_claim(&mut hodling, &mut iprofs, &who)?;
        }

        VirtualProfsFor::<T>::insert(&sym, profs);
        HodlingRecordFor::<T>::insert(key, record);

        Self::deposit_event(RawEvent::TokenRewardClaim(who, sym));
        Ok(())
    }
}

// trigger
impl<T: Trait> OnMoveToken<T::AccountId, T::TokenBalance> for Module<T> {
    fn on_move_token(from: &T::AccountId, to: &T::AccountId, sym: &Symbol, value: T::TokenBalance) {
        if is_valid_exchange_token::<T>(sym) == false {
            return;
        }

        if VirtualProfsFor::<T>::exists(sym) == false {
            return;
        }

        let mut profs = Self::virtual_profs_for(sym);
        let key_from = (from.clone(), sym.clone());
        let key_to = (to.clone(), sym.clone());
        if HodlingRecordFor::<T>::exists(&key_from) == false {
            return;
        }

        let mut from_record = Self::hodling_record_for(&key_from);
        let mut to_record = Self::hodling_record_for(&key_to);

        {
            let mut iprofs = IntentionProfsWrapper::<T> {
                sym: sym.clone(),
                profs: &mut profs,
            };
            let mut from_hodling = HodlingRecordWrapper::<T> {
                sym: sym.clone(),
                account: from.clone(),
                record: &mut from_record,
            };
            let mut to_hodling = HodlingRecordWrapper::<T> {
                sym: sym.clone(),
                account: to.clone(),
                record: &mut to_record,
            };

            // sub from
            staking::Module::<T>::update_vote_weight_both_way(
                &mut iprofs,
                &mut from_hodling,
                value.as_(),
                false,
            );
            // add to
            staking::Module::<T>::update_vote_weight_both_way(
                &mut iprofs,
                &mut to_hodling,
                value.as_(),
                true,
            );
        }
        VirtualProfsFor::<T>::insert(sym, profs);
        HodlingRecordFor::<T>::insert(key_from, from_record);
        HodlingRecordFor::<T>::insert(key_to, to_record);
    }
}

impl<T: Trait> OnNewSessionForTokenStaking<T::AccountId, T::Balance> for Module<T> {
    fn token_staking_info() -> Vec<(Validator<T::AccountId>, T::Balance)> {
        runtime_io::print("new session token stake  --sym--pcx--average_price");
        let mut syms: Vec<Symbol> = tokenbalances::Module::<T>::valid_token_list();
        if let Some(index) = syms.iter().position(|x| x.as_slice() == T::CHAINX_SYMBOL) {
            syms.remove(index);
        }

        syms.into_iter()
            .filter(|s| is_valid_exchange_token::<T>(s))
            .map(|sym| {
                let o = OrderPair {
                    first: T::CHAINX_SYMBOL.to_vec(),
                    second: sym.clone(),
                };
                // get price
                let pcx_amount =
                    if let Some(price) = pendingorders::Module::<T>::last_average_price(&o) {
                        // get token amount
                        let token_amount: T::TokenBalance =
                            tokenbalances::Module::<T>::total_token(&sym);
                        if price != Zero::zero() {
                            let r: T::TokenBalance = token_amount / As::sa(price.as_());
                            let pcx: T::Balance = As::sa(r.as_() as u64);
                            token_pcx_discount::<T>(&sym, pcx)
                        } else {
                            Zero::zero()
                        }
                    } else {
                        Zero::zero()
                    };
                // log
                runtime_io::print(sym.as_slice());
                runtime_io::print(pcx_amount.as_() as u64);
                match pendingorders::Module::<T>::last_average_price(o) {
                    Some(n) => runtime_io::print(n.as_() as u64),
                    None => runtime_io::print("None"),
                }

                (Validator::Token(sym), pcx_amount)
            })
            .collect()
    }
}

fn token_pcx_discount<T: Trait>(sym: &Symbol, pcx: T::Balance) -> T::Balance {
    let rate = Module::<T>::discount_ratio_for(sym);
    // calc discount
    pcx * As::sa(rate.0 as u64) / As::sa(rate.1 as u64)
}

impl<T: Trait> OnReward<T::AccountId, T::Balance> for Module<T> {
    fn on_reward(v: &Validator<T::AccountId>, b: T::Balance) {
        // trigger
        match v {
            Validator::Token(sym) => {
                runtime_io::print("reward for token, ---sym---newjackpot");
                runtime_io::print(sym.as_slice());

                VirtualProfsFor::<T>::mutate(sym, |profs| {
                    profs.jackpot += b;
                    runtime_io::print(profs.jackpot.as_());
                });
            }
            _ => { /*do nothing*/ }
        }
    }
}

impl<T: Trait> OnDepositToken<T::AccountId, T::TokenBalance> for Module<T> {
    fn on_deposit_token(who: &T::AccountId, sym: &Symbol, value: T::TokenBalance) {
        if is_valid_exchange_token::<T>(sym) == false {
            return;
        }
        change_vote::<T>(who, sym, value, true);

        deposit_reward::<T>(who, sym, value);
    }
}

fn deposit_reward<T: Trait>(who: &T::AccountId, sym: &Symbol, value: T::TokenBalance) {
    runtime_io::print("deposit reward  --sym--block_count--reward--new_jackpot--new_balance");
    runtime_io::print(sym.as_slice());

    let sec_per_block: T::Moment = timestamp::Module::<T>::block_period();
    let block_count: u32 = match sym.as_slice() {
        // btc
        btc::Module::<T>::SYMBOL => {
            let irr_block: u32 = btc::Module::<T>::irr_block();
            let all_second = irr_block * 10 * 60 * 60;
            all_second / sec_per_block.as_() as u32
        }
        _ => return,
    };
    runtime_io::print(block_count as u64);

    let mut profs = Module::<T>::virtual_profs_for(sym);
    if profs.last_total_weight == 0 {
        return;
    }
    let block_count: u128 = block_count as u128;
    // calc reward, block_count*value = weight, jackpot * weight/total_weight = reward, weight may larger than total_wight
    let reward =
        (profs.jackpot.as_() as u128) * block_count * value.as_() / profs.last_total_weight;

    runtime_io::print(reward as u64);

    let reward: T::Balance = As::sa(reward as u64);
    let balance: T::Balance = balances::Module::<T>::free_balance(who);
    // if jackpot not enough, draw all jackpot
    let new_jackpot: T::Balance = match profs.jackpot.checked_sub(&reward) {
        Some(n) => n,
        None => Zero::zero(),
    };
    let diff = profs.jackpot - new_jackpot;

    runtime_io::print(new_jackpot.as_());

    match balance.checked_add(&diff) {
        Some(new_balance) => {
            profs.jackpot = new_jackpot;
            VirtualProfsFor::<T>::insert(sym, profs);
            runtime_io::print(new_balance.as_());
            balances::FreeBalance::<T>::insert(who, new_balance);
        }
        None => {
            // no change for storage
        }
    };
}

impl<T: Trait> OnWithdrawToken<T::AccountId, T::TokenBalance> for Module<T> {
    fn on_withdraw_token(who: &T::AccountId, sym: &Symbol, value: T::TokenBalance) {
        if is_valid_exchange_token::<T>(sym) == false {
            return;
        }
        if VirtualProfsFor::<T>::exists(sym) == false {
            return;
        }
        if HodlingRecordFor::<T>::exists((who.clone(), sym.clone())) == false {
            return;
        }
        change_vote::<T>(who, sym, value, false)
    }
}

fn change_vote<T: Trait>(who: &T::AccountId, sym: &Symbol, value: T::TokenBalance, is_add: bool) {
    let mut profs = Module::<T>::virtual_profs_for(sym);
    let key = (who.clone(), sym.clone());
    let mut record = Module::<T>::hodling_record_for(&key);

    {
        let mut iprofs = IntentionProfsWrapper::<T> {
            sym: sym.clone(),
            profs: &mut profs,
        };
        let mut hodling = HodlingRecordWrapper::<T> {
            sym: sym.clone(),
            account: who.clone(),
            record: &mut record,
        };
        // sub from
        staking::Module::<T>::update_vote_weight_both_way(
            &mut iprofs,
            &mut hodling,
            value.as_(),
            is_add,
        );
    }
    VirtualProfsFor::<T>::insert(sym, profs);
    HodlingRecordFor::<T>::insert(key, record);
}

fn is_valid_exchange_token<T: Trait>(sym: &Symbol) -> bool {
    let o = OrderPair {
        first: T::CHAINX_SYMBOL.to_vec(),
        second: sym.clone(),
    };
    pendingorders::Module::<T>::pair_detail_of(o).is_some()
}
