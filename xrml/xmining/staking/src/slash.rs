use super::*;
use rstd::cmp;
use rstd::result;
use xsupport::info;

impl<T: Trait> Module<T> {
    /// Slash the double signer and return the slashed balance.
    ///
    /// TODO extract the similar slashing logic in shifter.rs.
    pub fn slash_double_signer(who: &T::AccountId) -> result::Result<T::Balance, &'static str> {
        if !Self::is_intention(who) {
            return Err("Cannot slash if the reported double signer is not an intention");
        }

        // Slash the whole jackpot of double signer.
        let council = xaccounts::Module::<T>::council_account();
        let jackpot = Self::jackpot_accountid_for_unsafe(who);

        let slashed = <xassets::Module<T>>::pcx_free_balance(&jackpot);
        let _ = <xassets::Module<T>>::pcx_move_free_balance(&jackpot, &council, slashed);
        info!(
            "[slash_double_signer] {:?} is slashed: {:?}",
            who!(who),
            slashed
        );

        // Force the double signer to be inactive.
        <xaccounts::IntentionPropertiesOf<T>>::mutate(who, |props| {
            props.is_active = false;
            props.last_inactive_since = <system::Module<T>>::block_number();
            info!("[slash_double_signer] force {:?} to be inactive", who!(who));
        });

        // Note the double signer so that he could be removed from the current validator set on new session.
        <EvilValidatorsPerSession<T>>::mutate(|evil_validators| {
            if !evil_validators.contains(&who) {
                evil_validators.push(who.clone())
            }
        });

        Ok(slashed)
    }

    fn reward_of_per_block(session_reward: T::Balance) -> T::Balance {
        let session_length = <xsession::SessionLength<T>>::get().as_();
        let validators_count = <xsession::Validators<T>>::get().len() as u64;
        T::Balance::sa(session_reward.as_() * validators_count / session_length)
    }

    /// Actually slash a given active validator by a specific amount.
    /// If the jackpot of the validator can't afford the penalty and there are more than minimum validators,
    /// then he should be enforced to be inactive and removed from the validator set.
    pub(super) fn slash_active_offline_validator(
        who: &T::AccountId,
        my_reward: T::Balance,
        validators: &mut Vec<T::AccountId>,
    ) {
        let council = xaccounts::Module::<T>::council_account();

        // Slash 10 times per block reward for each missed block.
        let missed = u64::from(<MissedOfPerSession<T>>::take(who));
        let reward_per_block = Self::reward_of_per_block(my_reward);
        let total_slash = cmp::max(
            T::Balance::sa(
                reward_per_block.as_() * missed * u64::from(Self::missed_blocks_severity()),
            ),
            T::Balance::sa(Self::minimum_penalty().as_() * missed),
        );

        let jackpot_addr = T::DetermineIntentionJackpotAccountId::accountid_for_unsafe(who);
        let jackpot_balance = <xassets::Module<T>>::pcx_free_balance(&jackpot_addr);

        let (slashed, should_be_enforced) = if total_slash <= jackpot_balance {
            (total_slash, false)
        } else {
            (jackpot_balance, true)
        };

        let _ = <xassets::Module<T>>::pcx_move_free_balance(&jackpot_addr, &council, slashed);

        debug!(
            "[slash_active_offline_validator] {:?} is actually slashed: {:?}, should be slashed: {:?}",
            who!(who),
            slashed,
            total_slash
        );

        // Force those slashed yet can't afford the penalty to be inactive when the validators is not too few.
        // Then these inactive validators will not be rewarded.
        if should_be_enforced && validators.len() > Self::minimum_validator_count() as usize {
            <xaccounts::IntentionPropertiesOf<T>>::mutate(who, |props| {
                props.is_active = false;
                props.last_inactive_since = <system::Module<T>>::block_number();
                info!(
                    "[slash_active_offline_validator] validator enforced to be inactive: {:?}",
                    who!(who)
                );
            });

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
            let missed = T::Balance::sa(u64::from(<MissedOfPerSession<T>>::take(who)));
            let should_slash = missed * Self::minimum_penalty();
            let council = xaccounts::Module::<T>::council_account();

            let jackpot_addr = T::DetermineIntentionJackpotAccountId::accountid_for_unsafe(who);
            let jackpot_balance = <xassets::Module<T>>::pcx_free_balance(&jackpot_addr);

            let slash = cmp::min(should_slash, jackpot_balance);

            let _ = <xassets::Module<T>>::pcx_move_free_balance(&jackpot_addr, &council, slash);
        }
    }
}
