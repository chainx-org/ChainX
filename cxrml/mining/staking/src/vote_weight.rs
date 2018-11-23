use primitives::traits::As;

use system;

use super::{Module, Trait};
use IntentionProfs;
use NominationRecord;

pub trait VoteWeight<BlockNumber> {
    fn amount(&self) -> u128;
    fn last_acum_weight(&self) -> u128;
    fn last_acum_weight_update(&self) -> u128;

    fn set_amount(&mut self, value: u128, to_add: bool);
    fn set_last_acum_weight(&mut self, s: u128);
    fn set_last_acum_weight_update(&mut self, num: BlockNumber);
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

        let amount = who.amount();
        let last_acum_weight = who.last_acum_weight();
        let last_acum_weight_update = who.last_acum_weight_update();

        let latest_acum_weight =
            last_acum_weight + amount * (current_block.as_() as u128 - last_acum_weight_update);

        who.set_last_acum_weight(latest_acum_weight);
        who.set_last_acum_weight_update(current_block);
    }

    fn generic_apply_delta<V: VoteWeight<T::BlockNumber>>(who: &mut V, value: u128, to_add: bool) {
        who.set_amount(value, to_add);
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
