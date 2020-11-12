// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

//! Some configurable implementations as associated type for the ChainX runtime.

use codec::{Decode, Encode};

use sp_runtime::{
    traits::{Convert, DispatchInfoOf, SignedExtension},
    transaction_validity::{
        InvalidTransaction, TransactionValidity, TransactionValidityError, ValidTransaction,
    },
    FixedPointNumber, Perquintill, RuntimeDebug,
};

use frame_support::{
    parameter_types,
    traits::{Currency, ExistenceRequirement, Imbalance, OnUnbalanced, WithdrawReason},
};

use pallet_transaction_payment::{Multiplier, TargetedFeeAdjustment};

use xpallet_gateway_common::Call as XGatewayCommonCall;
use xpallet_mining_staking::Call as XStakingCall;

use chainx_primitives::{AccountId, Balance};

use crate::{Authorship, Balances, Call, Runtime};

type NegativeImbalance = <Balances as Currency<AccountId>>::NegativeImbalance;

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
            xpallet_transaction_fee::Event::<Runtime>::FeePaid(
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

parameter_types! {
    pub const TargetBlockFullness: Perquintill = Perquintill::from_percent(25);
    pub AdjustmentVariable: Multiplier = Multiplier::saturating_from_rational(1, 100_000);
    pub MinimumMultiplier: Multiplier = Multiplier::saturating_from_rational(1, 1_000_000_000u128);
}

pub type SlowAdjustingFeeUpdate<R> =
    TargetedFeeAdjustment<R, TargetBlockFullness, AdjustmentVariable, MinimumMultiplier>;

/// A struct for charging additional fee for some special calls.
#[derive(Encode, Decode, Clone, Eq, PartialEq, RuntimeDebug)]
pub struct ChargeExtraFee;

impl ChargeExtraFee {
    /// Returns the optional extra fee for the given `call`.
    pub fn has_extra_fee(call: &Call) -> Option<Balance> {
        // 1 PCX
        const BASE_EXTRA_FEE: Balance = 100_000_000;

        let extra_cofficient: Option<u32> = match call {
            Call::XGatewayCommon(xgateway_common) => match xgateway_common {
                XGatewayCommonCall::setup_trustee(..) => Some(1),
                _ => None,
            },
            Call::XStaking(xstaking) => match xstaking {
                XStakingCall::register(..) => Some(10),
                XStakingCall::validate(..) => Some(1),
                XStakingCall::rebond(..) => Some(1),
                _ => None,
            },
            _ => None,
        };

        extra_cofficient.map(|cofficient| Balance::from(cofficient) * BASE_EXTRA_FEE)
    }

    /// Actually withdraws the extra `fee` from account `who`.
    pub fn withdraw_fee(who: &AccountId, fee: Balance) -> TransactionValidity {
        if let Err(_) = Balances::withdraw(
            who,
            fee,
            WithdrawReason::TransactionPayment.into(),
            ExistenceRequirement::KeepAlive,
        ) {
            return Err(InvalidTransaction::Payment.into());
        }

        DealWithFees::on_nonzero_unbalanced(NegativeImbalance::new(fee));

        Ok(ValidTransaction::default())
    }
}

impl SignedExtension for ChargeExtraFee {
    const IDENTIFIER: &'static str = "ChargeExtraFee";
    type AccountId = AccountId;
    type Call = Call;
    type AdditionalSigned = ();
    type Pre = ();

    fn additional_signed(&self) -> sp_std::result::Result<(), TransactionValidityError> {
        Ok(())
    }

    fn validate(
        &self,
        who: &Self::AccountId,
        call: &Self::Call,
        _info: &DispatchInfoOf<Self::Call>,
        _len: usize,
    ) -> TransactionValidity {
        if let Some(fee) = ChargeExtraFee::has_extra_fee(call) {
            ChargeExtraFee::withdraw_fee(who, fee)?;
        }

        Ok(ValidTransaction::default())
    }
}
