//! Some configurable implementations as associated type for the ChainX runtime.

use crate::{Authorship, Balances, NegativeImbalance, Runtime};
use chainx_primitives::Balance;
use frame_support::traits::{Currency, Imbalance, OnUnbalanced};
use sp_runtime::traits::Convert;

pub struct Author;
impl OnUnbalanced<NegativeImbalance> for Author {
    fn on_nonzero_unbalanced(amount: NegativeImbalance) {
        Balances::resolve_creating(&Authorship::author(), amount);
    }
}

pub struct DealWithFees;
impl OnUnbalanced<NegativeImbalance> for DealWithFees {
    fn on_nonzero_unbalanced(fees: NegativeImbalance) {
        // for fees, 90% to the reward pot of author, 10% to author
        let (to_reward_pot, to_author) = fees.ration(90, 10);

        let to_author_numeric_amount = to_author.peek();
        let to_reward_pot_numeric_amount = to_reward_pot.peek();

        let author = <pallet_authorship::Module<Runtime>>::author();
        let reward_pot = <xpallet_mining_staking::Module<Runtime>>::reward_pot_for(&author);

        <pallet_balances::Module<Runtime>>::resolve_creating(&author, to_author);
        <pallet_balances::Module<Runtime>>::resolve_creating(&reward_pot, to_reward_pot);
        <frame_system::Module<Runtime>>::deposit_event(
            xpallet_system::RawEvent::TransactionFeePaid(
                author,
                to_author_numeric_amount,
                reward_pot,
                to_reward_pot_numeric_amount,
            ),
        );
    }
}

/// Struct that handles the conversion of Balance -> `u64`. This is used for staking's election
/// calculation.
pub struct CurrencyToVoteHandler;

impl CurrencyToVoteHandler {
    fn factor() -> Balance {
        (Balances::total_issuance() / u64::max_value() as Balance).max(1)
    }
}

impl Convert<Balance, u64> for CurrencyToVoteHandler {
    fn convert(x: Balance) -> u64 {
        (x / Self::factor()) as u64
    }
}

impl Convert<u128, Balance> for CurrencyToVoteHandler {
    fn convert(x: u128) -> Balance {
        x * Self::factor()
    }
}
