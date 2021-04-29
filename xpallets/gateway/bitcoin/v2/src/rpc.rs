use sp_std::vec::Vec;

use crate::pallet::{BalanceOf, Config, Pallet, Vaults};

impl<T: Config> Pallet<T> {
    //rpc use
    pub fn get_first_matched_vault(xbtc_amount: BalanceOf<T>) -> Option<(T::AccountId, Vec<u8>)> {
        // Vaults::<T>::iter().filter_map()
        Vaults::<T>::iter()
            .find(|(vault_id, vault)| {
                if let Ok(token_upper_bound) = Self::_calculate_vault_token_upper_bound(vault_id) {
                    token_upper_bound
                        > xbtc_amount + Self::issued_tokens_of(vault_id) + vault.to_be_issued_tokens
                } else {
                    false
                }
            })
            .map(|(vault_id, vault)| (vault_id, vault.wallet))
    }
}
