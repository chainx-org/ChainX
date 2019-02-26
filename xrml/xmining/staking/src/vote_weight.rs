// Copyright 2018 Chainpool.
//! Vote weight calculation.

use super::{ClaimType, Module, Trait};
use rstd::result;
use runtime_primitives::traits::As;
use runtime_support::StorageMap;
use system;
use xassets::{self, Token};
use IntentionProfs;
use NominationRecord;

pub trait VoteWeight<BlockNumber: As<u64>> {
    fn amount(&self) -> u64;
    fn last_acum_weight(&self) -> u64;
    fn last_acum_weight_update(&self) -> u64;

    fn latest_acum_weight(&self, current_block: BlockNumber) -> u64 {
        Self::last_acum_weight(&self)
            + Self::amount(&self) * (current_block.as_() - Self::last_acum_weight_update(&self))
    }

    fn set_amount(&mut self, value: u64, to_add: bool);
    fn set_last_acum_weight(&mut self, s: u64);
    fn set_last_acum_weight_update(&mut self, num: BlockNumber);
}

impl<B, C> VoteWeight<C> for IntentionProfs<B, C>
where
    B: Default + As<u64> + Clone,
    C: Default + As<u64> + Clone,
{
    fn amount(&self) -> u64 {
        self.total_nomination.clone().as_()
    }

    fn last_acum_weight(&self) -> u64 {
        self.last_total_vote_weight as u64
    }

    fn last_acum_weight_update(&self) -> u64 {
        self.last_total_vote_weight_update.clone().as_()
    }

    fn set_amount(&mut self, value: u64, to_add: bool) {
        let mut amount = Self::amount(self);
        if to_add {
            amount += value;
        } else {
            amount -= value;
        }
        self.total_nomination = B::sa(amount);
    }

    fn set_last_acum_weight(&mut self, latest_vote_weight: u64) {
        self.last_total_vote_weight = latest_vote_weight;
    }

    fn set_last_acum_weight_update(&mut self, current_block: C) {
        self.last_total_vote_weight_update = current_block;
    }
}

impl<B, C> VoteWeight<C> for NominationRecord<B, C>
where
    B: Default + As<u64> + Clone,
    C: Default + As<u64> + Clone,
{
    fn amount(&self) -> u64 {
        self.nomination.clone().as_()
    }

    fn last_acum_weight(&self) -> u64 {
        self.last_vote_weight
    }

    fn last_acum_weight_update(&self) -> u64 {
        self.last_vote_weight_update.clone().as_()
    }

    fn set_amount(&mut self, value: u64, to_add: bool) {
        let mut amount = Self::amount(self);
        if to_add {
            amount += value;
        } else {
            amount -= value;
        }
        self.nomination = B::sa(amount);
    }

    fn set_last_acum_weight(&mut self, latest_vote_weight: u64) {
        self.last_vote_weight = latest_vote_weight;
    }

    fn set_last_acum_weight_update(&mut self, current_block: C) {
        self.last_vote_weight_update = current_block;
    }
}

impl<T: Trait> Module<T> {
    fn generic_update_vote_weight<V: VoteWeight<T::BlockNumber>>(who: &mut V) {
        let current_block = <system::Module<T>>::block_number();

        let latest_acum_weight = who.latest_acum_weight(current_block);

        who.set_last_acum_weight(latest_acum_weight);
        who.set_last_acum_weight_update(current_block);
    }

    fn generic_apply_delta<V: VoteWeight<T::BlockNumber>>(who: &mut V, value: u64, to_add: bool) {
        who.set_amount(value, to_add);
    }

    fn channel_or_council_of(who: &T::AccountId, token: &Token) -> T::AccountId {
        let council_address = Self::council_address();

        if let Some(asset_info) = <xassets::AssetInfo<T>>::get(token) {
            let asset = asset_info.0;
            let chain = asset.chain();

            if let Some(bind) = <xaccounts::CrossChainBindOf<T>>::get(&(chain, who.clone())) {
                if let Some(first_bind) = bind.get(0) {
                    if let Some(addr_map) =
                        <xaccounts::CrossChainAddressMapOf<T>>::get(&(chain, first_bind.clone()))
                    {
                        return addr_map.1;
                    }
                }
            }
        }

        return council_address;
    }

    pub fn generic_claim<U, V>(
        source: &mut U,
        who: &T::AccountId,
        target: &mut V,
        target_jackpot_addr: &T::AccountId,
        claim_type: ClaimType,
    ) -> result::Result<(u64, u64, T::Balance), &'static str>
    where
        U: VoteWeight<T::BlockNumber>,
        V: VoteWeight<T::BlockNumber>, // + Jackpot<T::Balance>,
    {
        let current_block = <system::Module<T>>::block_number();

        let source_vote_weight = source.latest_acum_weight(current_block);

        if source_vote_weight == 0 {
            return Err("the vote weight of claimer is zero.");
        }

        let target_vote_weight = target.latest_acum_weight(current_block);

        let total_jackpot: u64 = xassets::Module::<T>::pcx_free_balance(target_jackpot_addr).as_();

        // source_vote_weight * total_jackpot could overflow.
        let dividend = match (source_vote_weight as u128).checked_mul(total_jackpot as u128) {
            Some(x) => T::Balance::sa((x / target_vote_weight as u128) as u64),
            None => panic!("source_vote_weight * total_jackpot overflow"),
        };

        match claim_type {
            ClaimType::Intention => {
                xassets::Module::<T>::pcx_move_free_balance(target_jackpot_addr, who, dividend)
                    .map_err(|e| e.info())?;
            }
            ClaimType::PseduIntention(token) => {
                let channel_or_council = Self::channel_or_council_of(who, &token);
                // FIXME be configurable
                let to_channel_or_council = T::Balance::sa(dividend.as_() * 1 / 10);

                xassets::Module::<T>::pcx_move_free_balance(
                    target_jackpot_addr,
                    &channel_or_council,
                    to_channel_or_council,
                )
                .map_err(|e| e.info())?;

                xassets::Module::<T>::pcx_move_free_balance(
                    target_jackpot_addr,
                    who,
                    dividend - to_channel_or_council,
                )
                .map_err(|e| e.info())?;
            }
        }

        source.set_last_acum_weight(0);
        source.set_last_acum_weight_update(current_block);

        target.set_last_acum_weight(target_vote_weight - source_vote_weight);
        target.set_last_acum_weight_update(current_block);

        Ok((source_vote_weight, target_vote_weight, dividend))
    }

    pub fn update_vote_weight_both_way<
        U: VoteWeight<T::BlockNumber>,
        V: VoteWeight<T::BlockNumber>,
    >(
        source: &mut U,
        target: &mut V,
        value: u64,
        to_add: bool,
    ) {
        // Update to the latest vote weight
        Self::generic_update_vote_weight(source);
        Self::generic_update_vote_weight(target);
        // Update the nomination balance
        Self::generic_apply_delta(source, value, to_add);
        Self::generic_apply_delta(target, value, to_add);
    }
}
