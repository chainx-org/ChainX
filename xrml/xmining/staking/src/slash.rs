use super::*;
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
}
