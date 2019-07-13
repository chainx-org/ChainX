// Copyright 2017-2018 Parity Technologies (UK) Ltd.
// Copyright 2018-2019 Chainpool.
// This file is part of Substrate.

// Substrate is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Substrate is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Substrate.  If not, see <http://www.gnu.org/licenses/>.

//! Session manager: is told the validators and allows them to manage their session keys for the
//! consensus module.

#![cfg_attr(not(feature = "std"), no_std)]

mod mock;
mod tests;

// Substrate
use primitives::traits::{As, Convert, One, Zero};
use rstd::{ops::Mul, prelude::*};
use support::{
    decl_event, decl_module, decl_storage, dispatch::Result, for_each_tuple, StorageMap,
    StorageValue,
};

// ChainX
use xr_primitives::Name;

/// A session has changed.
pub trait OnSessionChange<T> {
    /// Session has changed.
    fn on_session_change();
}

macro_rules! impl_session_change {
	() => (
		impl<T> OnSessionChange<T> for () {
			fn on_session_change() {}
		}
	);

	( $($t:ident)* ) => {
		impl<T: Clone, $($t: OnSessionChange<T>),*> OnSessionChange<T> for ($($t,)*) {
			fn on_session_change() {
				$($t::on_session_change();)*
			}
		}
	}
}

for_each_tuple!(impl_session_change);

pub enum SessionKeyUsability<AccountId> {
    UsedBy(AccountId),
    Unused,
}

pub trait Trait: timestamp::Trait + xaccounts::Trait {
    type ConvertAccountIdToSessionKey: Convert<Self::AccountId, Option<Self::SessionKey>>;
    type OnSessionChange: OnSessionChange<Self::Moment>;
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event<T>() = default;

        /// Set a new session length. Won't kick in until the next session change (at current length).
        fn set_length(#[compact] new: T::BlockNumber) {
            <NextSessionLength<T>>::put(new);
        }

        /// Forces a new session.
        fn force_new_session(apply_rewards: bool) -> Result {
            Self::apply_force_new_session(apply_rewards)
        }

        fn on_finalize(n: T::BlockNumber) {
            Self::check_rotate_session(n);
        }
    }
}

decl_event!(
	pub enum Event<T> where <T as system::Trait>::BlockNumber {
		/// New session has happened. Note that the argument is the session index, not the block
		/// number as the type might suggest.
		NewSession(BlockNumber),
	}
);

decl_storage! {
    trait Store for Module<T: Trait> as Session {

        /// The current set of validators.
        pub Validators get(validators) config(): Vec<(T::AccountId, u64)>;
        /// Current length of the session.
        pub SessionLength get(length) config(session_length): T::BlockNumber = T::BlockNumber::sa(1000);
        /// Current index of the session.
        pub CurrentIndex get(current_index) build(|_| T::BlockNumber::sa(0)): T::BlockNumber;
        /// Timestamp when current session started.
        pub CurrentStart get(current_start) build(|_| T::Moment::zero()): T::Moment;

        /// Total missed blocks count in the last session.
        pub SessionTotalMissedBlocksCount get(session_total_missed_blocks_count) : u32;

        /// New session is being forced is this entry exists; in which case, the boolean value is whether
        /// the new session should be considered a normal rotation (rewardable) or exceptional (slashable).
        pub ForcingNewSession get(forcing_new_session): Option<bool>;
        /// Block at which the session length last changed.
        LastLengthChange: Option<T::BlockNumber>;
        /// The next key for a given validator.
        NextKeyFor get(next_key_for) build(|config: &GenesisConfig<T>| {
            config.keys.clone()
        }): map T::AccountId => Option<T::SessionKey>;
        KeyFilterMap build(|config: &GenesisConfig<T>| {
            config.keys.clone().into_iter().map(|(a, b)| (b, a)).collect::<Vec<(T::SessionKey, T::AccountId)>>()
        }): map T::SessionKey => Option<T::AccountId>;
        /// The next session length.
        NextSessionLength: Option<T::BlockNumber>;
    }
    add_extra_genesis {
        config(keys): Vec<(T::AccountId, T::SessionKey)>;
    }
}

impl<T: Trait> xsystem::ValidatorList<T::AccountId> for Module<T> {
    fn validator_list() -> Vec<T::AccountId> {
        Self::validators().into_iter().map(|(a, _)| a).collect()
    }
}

impl<T: Trait> Module<T> {
    pub fn pubkeys_for_validator_name(name: Name) -> Option<(T::AccountId, Option<T::SessionKey>)> {
        xaccounts::Module::<T>::intention_of(&name).map(|a| {
            let r = Self::next_key_for(&a);
            (a, r)
        })
    }

    pub fn check_session_key_usability(key: &T::SessionKey) -> SessionKeyUsability<T::AccountId> {
        if let Some(cur_owner) = <KeyFilterMap<T>>::get(key) {
            SessionKeyUsability::UsedBy(cur_owner)
        } else {
            SessionKeyUsability::Unused
        }
    }

    pub fn account_id_for(key: &T::SessionKey) -> Option<T::AccountId> {
        <KeyFilterMap<T>>::get(key)
    }

    pub fn set_key(who: &T::AccountId, key: &T::SessionKey) {
        if let Some(old_key) = <NextKeyFor<T>>::get(who) {
            <KeyFilterMap<T>>::remove(old_key);
        }
        <NextKeyFor<T>>::insert(who, key);
        <KeyFilterMap<T>>::insert(key, who);
    }
}

impl<T: Trait> Module<T> {
    /// The number of validators currently.
    pub fn validator_count() -> u32 {
        <Validators<T>>::get().len() as u32 // TODO: can probably optimised
    }

    /// The last length change, if there was one, zero if not.
    pub fn last_length_change() -> T::BlockNumber {
        <LastLengthChange<T>>::get().unwrap_or_else(T::BlockNumber::zero)
    }

    // INTERNAL API (available to other runtime modules)
    /// Forces a new session, no origin.
    pub fn apply_force_new_session(apply_rewards: bool) -> Result {
        <ForcingNewSession<T>>::put(apply_rewards);
        Ok(())
    }

    /// Set the current set of validators.
    ///
    /// Called by `staking::new_era()` only. `next_session` should be called after this in order to
    /// update the session keys to the next validator set.
    pub fn set_validators(new: &[(T::AccountId, u64)]) {
        <Validators<T>>::put(&new.to_vec());
        <consensus::Module<T>>::set_authorities(
            &new.iter()
                .cloned()
                .map(|(account_id, _)| {
                    <NextKeyFor<T>>::get(account_id.clone())
                        .or_else(|| T::ConvertAccountIdToSessionKey::convert(account_id))
                        .unwrap_or_default()
                })
                .collect::<Vec<_>>(),
        );
    }

    /// Hook to be called after transaction processing.
    pub fn check_rotate_session(block_number: T::BlockNumber) {
        // do this last, after the staking system has had chance to switch out the authorities for the
        // new set.
        // check block number and call next_session if necessary.
        let is_final_block =
            ((block_number - Self::last_length_change()) % Self::length()).is_zero();
        let (should_end_session, apply_rewards) = <ForcingNewSession<T>>::take()
            .map_or((is_final_block, is_final_block), |apply_rewards| {
                (true, apply_rewards)
            });
        if should_end_session {
            Self::rotate_session(is_final_block, apply_rewards);
        }
    }

    /// Move onto next session: register the new authority set.
    pub fn rotate_session(is_final_block: bool, _apply_rewards: bool) {
        let now = <timestamp::Module<T>>::get();
        let session_index = <CurrentIndex<T>>::get() + One::one();

        Self::deposit_event(RawEvent::NewSession(session_index));

        // Increment current session index.
        <CurrentIndex<T>>::put(session_index);
        <CurrentStart<T>>::put(now);

        // Enact session length change.
        let len_changed = if let Some(next_len) = <NextSessionLength<T>>::take() {
            <SessionLength<T>>::put(next_len);
            true
        } else {
            false
        };
        if len_changed || !is_final_block {
            let block_number = <system::Module<T>>::block_number();
            <LastLengthChange<T>>::put(block_number);
        }

        T::OnSessionChange::on_session_change();
    }

    /// Get the time that should have elapsed over a session if everything was working perfectly.
    pub fn ideal_session_duration() -> T::Moment {
        let block_period: T::Moment = <timestamp::Module<T>>::minimum_period();
        let session_length: T::BlockNumber = Self::length();
        Mul::<T::BlockNumber>::mul(block_period, session_length)
    }

    /// Number of blocks remaining in this session, not counting this one. If the session is
    /// due to rotate at the end of this block, then it will return 0. If the just began, then
    /// it will return `Self::length() - 1`.
    pub fn blocks_remaining() -> T::BlockNumber {
        let length = Self::length();
        let length_minus_1 = length - One::one();
        let block_number = <system::Module<T>>::block_number();
        length_minus_1 - (block_number - Self::last_length_change() + length_minus_1) % length
    }
}
