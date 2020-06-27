//! # Staking Module

#![cfg_attr(not(feature = "std"), no_std)]

mod types;

use chainx_primitives::Memo;
use frame_support::{
    decl_error, decl_event, decl_module, decl_storage,
    dispatch::DispatchResult,
    traits::Get,
    weights::{
        DispatchInfo, GetDispatchInfo, Pays, PostDispatchInfo, Weight, WeightToFeeCoefficient,
        WeightToFeePolynomial,
    },
};
use frame_system::ensure_signed;
use sp_runtime::{
    traits::{
        Convert, DispatchInfoOf, Dispatchable, PostDispatchInfoOf, SaturatedConversion, Saturating,
        SignedExtension, UniqueSaturatedFrom, UniqueSaturatedInto, Zero,
    },
    FixedI128, FixedPointNumber, FixedPointOperand,
};
use sp_std::prelude::*;
use xp_staking::UnbondedIndex;

pub trait Trait: frame_system::Trait + xpallet_assets::Trait {}

decl_storage! {
    trait Store for Module<T: Trait> as XStaking {
    }
}

decl_event!(
    pub enum Event<T>
    where
        <T as frame_system::Trait>::AccountId,
        <T as xpallet_assets::Trait>::Balance,
    {
        /// The staker has been rewarded by this amount. `AccountId` is the stash account.
        Reward(AccountId, Balance),
        /// One validator (and its nominators) has been slashed by the given amount.
        Slash(AccountId, Balance),
        ///
        /// NOTE: This event is only emitted when funds are bonded via a dispatchable. Notably,
        /// it will not be emitted for staking rewards when they are added to stake.
        Bonded(AccountId, Balance),
        /// An account has unbonded this amount.
        Unbonded(AccountId, Balance),
        /// An account has called `withdraw_unbonded` and removed unbonding chunks worth `Balance`
        /// from the unlocking queue.
        Withdrawn(AccountId, Balance),
    }
);

decl_error! {
    /// Error for the staking module.
    pub enum Error for Module<T: Trait> {
        /// Stash is already bonded.
        AlreadyBonded,
        /// Controller is already paired.
        AlreadyPaired,
        /// Duplicate index.
        DuplicateIndex,
        /// Slash record index out of bounds.
        InvalidSlashIndex,
        /// Can not bond with value less than minimum balance.
        InsufficientValue,
        /// Can not schedule more unlock chunks.
        NoMoreChunks,
        /// Can not rebond without unlocking chunks.
        NoUnlockChunk,
        /// Rewards for this era have already been claimed for this validator.
        AlreadyClaimed,
        /// The call is not allowed at the given time due to restrictions of election period.
        CallNotAllowed,
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn on_finalize() {
        }

        /// Nominates the `target` with `value` of the origin account's balance locked.
        #[weight = 10]
        fn bond(origin, target: T::AccountId, value: T::Balance, memo: Memo) {
            let sender = ensure_signed(origin)?;
        }

        /// Switchs the nomination of `value` from one validator to another.
        #[weight = 10]
        fn rebond(origin, from: T::AccountId, to: T::AccountId, value: T::Balance, memo: Memo) {
            let sender = ensure_signed(origin)?;
        }

        ///
        #[weight = 10]
        fn unbond(origin, target: T::AccountId, memo: Memo) {
            let sender = ensure_signed(origin)?;
        }

        /// Frees up the unbonded balances that are due.
        #[weight = 10]
        fn withdraw_unbonded(origin, target: T::AccountId, unbonded_index: UnbondedIndex) {
            let sender = ensure_signed(origin)?;
        }

        /// Claims the staking reward given the `target` validator.
        #[weight = 10]
        fn claim(origin, target: T::AccountId) {
            let sender = ensure_signed(origin)?;
        }

        /// Declare the desire to validate for the origin account.
        #[weight = 10]
        fn validate(origin) {
            let sender = ensure_signed(origin)?;
        }

        /// Declare no desire to validate for the origin account.
        #[weight = 10]
        fn chill(origin, target: T::AccountId, value: T::Balance, memo: Memo) {
            let sender = ensure_signed(origin)?;
        }

        /// TODO: figure out whether this should be kept.
        #[weight = 10]
        fn register(origin) {
            let sender = ensure_signed(origin)?;
        }
    }
}
