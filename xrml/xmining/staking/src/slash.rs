// Copyright 2018-2019 Chainpool.

use super::*;
use rstd::cmp;
use rstd::result;
use xsupport::{debug, info};

impl<T: Trait> Module<T> {
    /// Actually slash the account being punished, all the slashed value will go to the council.
    fn apply_slash(slashed_account: &T::AccountId, value: T::Balance) {
        let council = xaccounts::Module::<T>::council_account();
        debug!(
            "[apply_slash]slashed_account:{:?},value:{}",
            slashed_account, value
        );
        let _ = <xassets::Module<T>>::pcx_move_free_balance(&slashed_account, &council, value);
    }

    /// Slash the total balance of misbehavior's jackpot, return the actually slashed value.
    ///
    /// This is considered be be successful always.
    fn slash_whole_jackpot(who: &T::AccountId) -> T::Balance {
        let jackpot = Self::jackpot_accountid_for_unsafe(who);
        let slashed = <xassets::Module<T>>::pcx_free_balance(&jackpot);
        Self::apply_slash(&jackpot, slashed);
        slashed
    }

    /// Try slash given the intention account and value, otherwise empty the whole jackpot.
    ///
    /// If the balance of its jackpot is unable to pay, just slash it all and return the actually slash value.
    fn try_slash_or_clear(
        who: &T::AccountId,
        should_slash: T::Balance,
    ) -> result::Result<(), T::Balance> {
        let jackpot_account = T::DetermineIntentionJackpotAccountId::accountid_for_unsafe(who);
        let jackpot_balance = <xassets::Module<T>>::pcx_free_balance(&jackpot_account);

        if should_slash <= jackpot_balance {
            Self::apply_slash(&jackpot_account, should_slash);
            Ok(())
        } else {
            Self::apply_slash(&jackpot_account, jackpot_balance);
            Err(jackpot_balance)
        }
    }

    /// Slash the double signer and return the slashed balance.
    pub fn slash_double_signer(who: &T::AccountId) -> result::Result<T::Balance, &'static str> {
        if !Self::is_intention(who) {
            return Err("Cannot slash if the reported double signer is not an intention");
        }

        // Slash the whole jackpot of double signer.
        let slashed = Self::slash_whole_jackpot(who);
        info!(
            "[slash_double_signer] {:?} is slashed: {:?}",
            who!(who),
            slashed
        );

        // Force the double signer to be inactive.
        if Self::try_force_inactive(who).is_ok() {
            info!("[slash_double_signer] force {:?} to be inactive", who!(who));
        }

        // Note the double signer so that he could be removed from the current validator set on new session.
        <EvilValidatorsPerSession<T>>::mutate(|evil_validators| {
            if !evil_validators.contains(&who) {
                evil_validators.push(who.clone())
            }
        });

        Ok(slashed)
    }

    fn reward_of_per_block(session_reward: T::Balance) -> T::Balance {
        let session_length = <xsession::SessionLength<T>>::get().saturated_into::<u64>();
        let validators_count = <xsession::Validators<T>>::get().len() as u64;
        (session_reward.into() * validators_count / session_length).into()
    }

    /// Actually slash a given active validator by a specific amount.
    /// If the jackpot of the validator can't afford the penalty and there are more than minimum validators,
    /// then he should be enforced to be inactive and removed from the validator set.
    pub(super) fn slash_active_offline_validator(
        who: &T::AccountId,
        my_reward: T::Balance,
        validators: &mut Vec<T::AccountId>,
    ) {
        // Slash 10 times per block reward for each missed block.
        let missed = u64::from(<MissedOfPerSession<T>>::take(who));
        let reward_per_block = Self::reward_of_per_block(my_reward);
        let total_slash = cmp::max(
            (reward_per_block.into() * missed * u64::from(Self::missed_blocks_severity())).into(),
            (Self::minimum_penalty().saturated_into::<u64>() * missed).into(),
        );

        let (_slashed, should_be_enforced) =
            if let Err(slashed) = Self::try_slash_or_clear(who, total_slash) {
                (slashed, true)
            } else {
                (total_slash, false)
            };

        debug!(
            "[slash_active_offline_validator] {:?} is actually slashed: {:?}, should be slashed: {:?}",
            who!(who),
            _slashed,
            total_slash
        );

        // Force those slashed yet can't afford the penalty to be inactive when the validators is not too few.
        // Then these inactive validators will not be rewarded.
        if should_be_enforced && validators.len() > Self::minimum_validator_count() as usize {
            info!(
                "[slash_active_offline_validator] validator enforced to be inactive: {:?}",
                who!(who)
            );
            Self::force_inactive_unsafe(who);

            // remove from the current validator set
            validators.retain(|x| *x != *who);
        }
    }

    /// These offline validators choose to be inactive by themselves.
    /// Since they are already inactive at present, they won't share the reward,
    /// so we only need to slash them at the minimal penalty for the missed blocks when they were active.
    pub(super) fn slash_inactive_offline_validators() {
        let slashed = <OfflineValidatorsPerSession<T>>::get();
        if slashed.is_empty() {
            return;
        }

        let mut missed_info = Vec::new();
        let mut inactive_slashed = Vec::new();

        for s in slashed {
            let missed_num = <MissedOfPerSession<T>>::get(&s);
            missed_info.push((s.clone(), missed_num));
            if !Self::is_active(&s) {
                inactive_slashed.push(s);
            }
        }

        Self::deposit_event(RawEvent::MissedBlocksOfOfflineValidatorPerSession(
            missed_info,
        ));

        for who in inactive_slashed.iter() {
            let missed: T::Balance = u64::from(<MissedOfPerSession<T>>::take(who)).into();
            let should_slash = missed * Self::minimum_penalty();
            let _ = Self::try_slash_or_clear(who, should_slash);
        }
    }
}
