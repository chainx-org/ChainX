use primitives::traits::As;
use runtime_support::dispatch::Result;

use balances;
use system;

use super::{Module, Trait};
use IntentionProfs;
use NominationRecord;

pub trait VoteWeight<BlockNumber: As<u64>> {
    fn amount(&self) -> u128;
    fn last_acum_weight(&self) -> u128;
    fn last_acum_weight_update(&self) -> u128;

    fn latest_acum_weight(&self, current_block: BlockNumber) -> u128 {
        Self::last_acum_weight(&self)
            + Self::amount(&self)
                * (current_block.as_() as u128 - Self::last_acum_weight_update(&self))
    }

    fn set_amount(&mut self, value: u128, to_add: bool);
    fn set_last_acum_weight(&mut self, s: u128);
    fn set_last_acum_weight_update(&mut self, num: BlockNumber);
}

pub trait Jackpot<Balance: Clone> {
    fn jackpot(&self) -> Balance;
    fn set_jackpot(&mut self, value: Balance);
}

impl<B, C> Jackpot<B> for IntentionProfs<B, C>
where
    B: Default + Clone,
    C: Default + Clone,
{
    fn jackpot(&self) -> B {
        self.jackpot.clone()
    }

    fn set_jackpot(&mut self, value: B) {
        self.jackpot = value;
    }
}

impl<B, C> VoteWeight<C> for IntentionProfs<B, C>
where
    B: Default + As<u64> + Clone,
    C: Default + As<u64> + Clone,
{
    fn amount(&self) -> u128 {
        self.total_nomination.clone().as_() as u128
    }

    fn last_acum_weight(&self) -> u128 {
        self.last_total_vote_weight as u128
    }

    fn last_acum_weight_update(&self) -> u128 {
        self.last_total_vote_weight_update.clone().as_() as u128
    }

    fn set_amount(&mut self, value: u128, to_add: bool) {
        let mut amount = Self::amount(self);
        if to_add {
            amount += value;
        } else {
            amount -= value;
        }
        self.total_nomination = B::sa(amount as u64);
    }

    fn set_last_acum_weight(&mut self, latest_vote_weight: u128) {
        self.last_total_vote_weight = latest_vote_weight as u64;
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
    fn amount(&self) -> u128 {
        self.nomination.clone().as_() as u128
    }

    fn last_acum_weight(&self) -> u128 {
        self.last_vote_weight as u128
    }

    fn last_acum_weight_update(&self) -> u128 {
        self.last_vote_weight_update.clone().as_() as u128
    }

    fn set_amount(&mut self, value: u128, to_add: bool) {
        let mut amount = Self::amount(self);
        if to_add {
            amount += value;
        } else {
            amount -= value;
        }
        self.nomination = B::sa(amount as u64);
    }

    fn set_last_acum_weight(&mut self, latest_vote_weight: u128) {
        self.last_vote_weight = latest_vote_weight as u64;
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

    fn generic_apply_delta<V: VoteWeight<T::BlockNumber>>(who: &mut V, value: u128, to_add: bool) {
        who.set_amount(value, to_add);
    }

    pub fn generic_claim<U, V>(source: &mut U, target: &mut V, who: &T::AccountId) -> Result
    where
        U: VoteWeight<T::BlockNumber>,
        V: VoteWeight<T::BlockNumber> + Jackpot<T::Balance>,
    {
        let current_block = <system::Module<T>>::block_number();

        let source_vote_weight = source.latest_acum_weight(current_block);

        if source_vote_weight == 0 {
            return Err("the vote weight of claimer is zero.");
        }

        let target_vote_weight = target.latest_acum_weight(current_block);

        let jackpot = target.jackpot();

        let dividend = T::Balance::sa(
            (source_vote_weight * jackpot.as_() as u128 / target_vote_weight) as u64,
        );

        <balances::Module<T>>::reward(who, dividend)?;

        target.set_jackpot(jackpot - dividend);

        source.set_last_acum_weight(0);
        source.set_last_acum_weight_update(current_block);

        target.set_last_acum_weight(target_vote_weight - source_vote_weight);
        target.set_last_acum_weight_update(current_block);

        Ok(())
    }

    pub fn update_vote_weight_both_way<
        U: VoteWeight<T::BlockNumber>,
        V: VoteWeight<T::BlockNumber>,
    >(
        source: &mut U,
        target: &mut V,
        value: u128,
        to_add: bool,
    ) {
        Self::generic_update_vote_weight(source);
        Self::generic_update_vote_weight(target);
        Self::generic_apply_delta(source, value, to_add);
        Self::generic_apply_delta(target, value, to_add);
    }
}
