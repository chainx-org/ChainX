use crate::*;

impl<T: Trait> Module<T> {
    /// Initializes the validators exported from ChainX 1.0.
    #[cfg(feature = "std")]
    pub fn initialize_legacy_validators(
        validators: &[xp_genesis_builder::ValidatorInfo<T::AccountId, BalanceOf<T>>],
    ) -> DispatchResult {
        for xp_genesis_builder::ValidatorInfo {
            who,
            referral_id,
            self_bonded,
            total_nomination,
            total_weight,
        } in validators
        {
            Self::check_referral_id(referral_id)?;
            if !self_bonded.is_zero() {
                assert!(
                    Self::free_balance(who) >= *self_bonded,
                    "Validator does not have enough balance to bond."
                );
                Self::bond_reserve(who, *self_bonded);
                Nominations::<T>::mutate(who, who, |nominator| {
                    nominator.nomination = *self_bonded;
                });
            }
            Self::apply_register(who, referral_id.to_vec());
            // These validators will be chilled on the network startup.
            Self::apply_force_chilled(who);

            ValidatorLedgers::<T>::mutate(who, |validator| {
                validator.total_nomination = *total_nomination;
                validator.last_total_vote_weight = *total_weight;
            });
        }
        Ok(())
    }

    #[cfg(feature = "std")]
    pub fn force_bond(
        sender: &T::AccountId,
        target: &T::AccountId,
        value: BalanceOf<T>,
    ) -> DispatchResult {
        if !value.is_zero() {
            Self::bond_reserve(sender, value);
            Nominations::<T>::mutate(sender, target, |nominator| {
                nominator.nomination = value;
            });
        }
        Ok(())
    }

    /// Mock the `unbond` operation but lock the funds until the specific height `locked_until`.
    #[cfg(feature = "std")]
    pub fn force_unbond(
        sender: &T::AccountId,
        target: &T::AccountId,
        value: BalanceOf<T>,
        locked_until: T::BlockNumber,
    ) -> DispatchResult {
        // We can not reuse can_unbond() as the target can has no bond but has unbonds.
        // Self::can_unbond(sender, target, value)?;
        ensure!(Self::is_validator(target), Error::<T>::NotValidator);
        ensure!(
            Self::unbonded_chunks_of(sender, target).len()
                < Self::maximum_unbonded_chunk_size() as usize,
            Error::<T>::NoMoreUnbondChunks
        );
        Self::unbond_reserve(sender, value)?;
        Self::mutate_unbonded_chunks(sender, target, value, locked_until);
        Ok(())
    }

    #[cfg(feature = "std")]
    pub fn force_set_nominator_vote_weight(
        nominator: &T::AccountId,
        validator: &T::AccountId,
        new_weight: VoteWeight,
    ) {
        Nominations::<T>::mutate(nominator, validator, |nominator| {
            nominator.last_vote_weight = new_weight;
        });
    }

    #[cfg(feature = "std")]
    pub fn force_set_validator_vote_weight(who: &T::AccountId, new_weight: VoteWeight) {
        ValidatorLedgers::<T>::mutate(who, |validator| {
            validator.last_total_vote_weight = new_weight;
        });
    }
}
