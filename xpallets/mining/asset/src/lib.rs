//! # Staking Module

#![cfg_attr(not(feature = "std"), no_std)]

mod impls;
mod types;

use chainx_primitives::{AssetId, Memo};
use frame_support::{
    decl_error, decl_event, decl_module, decl_storage,
    dispatch::DispatchResult,
    ensure,
    storage::IterableStorageMap,
    traits::Get,
    weights::{DispatchInfo, GetDispatchInfo, PostDispatchInfo, Weight},
};
use frame_system::{self as system, ensure_signed};
use sp_runtime::traits::{
    Convert, DispatchInfoOf, Dispatchable, PostDispatchInfoOf, SaturatedConversion, Saturating,
    SignedExtension, UniqueSaturatedFrom, UniqueSaturatedInto, Zero,
};
use sp_std::prelude::*;
use types::*;
use xpallet_assets::{AssetErr, AssetType};
use xpallet_support::debug;

pub trait Trait: frame_system::Trait + xpallet_assets::Trait {
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
}

decl_storage! {
    trait Store for Module<T: Trait> as XStaking {
        ///
        pub DepositReward get(fn deposit_reward): T::Balance = 100_000.into();
        ///
        pub ClaimRestrictionOf get(fn claim_restriction_of):
            map hasher(twox_64_concat) AssetId => ClaimRestriction<T::BlockNumber>;
        /// External Assets that have the mining rights.
        pub MiningPrevilegedAssets get(fn mining_previleged_assets): Vec<AssetId>;
        /// Mining weight information of the asset.
        pub AssetLedgers get(fn asset_ledgers):
            map hasher(twox_64_concat) AssetId => AssetLedger<T::BlockNumber>;
        /// The map from nominator to the vote weight ledger of all nominees.
        pub MinerLedgers get(fn miner_ledgers):
            double_map hasher(twox_64_concat) T::AccountId, hasher(twox_64_concat) AssetId
            => MinerLedger<T::BlockNumber>;
        /// Mining power map of X-type assets.
        pub XTypeAssetPowerMap get(fn x_type_asset_power_map):
            map hasher(twox_64_concat) AssetId => FixedAssetPower;
    }
}

decl_event!(
    pub enum Event<T>
    where
        <T as frame_system::Trait>::AccountId,
        <T as xpallet_assets::Trait>::Balance,
    {
        ///
        Claim(AccountId, AccountId, Balance),
    }
);

decl_error! {
    /// Error for the staking module.
    pub enum Error for Module<T: Trait> {
        /// The asset does not have the mining rights.
        UnprevilegedAsset,
        ///
        InvalidUnbondedIndex,
        ///
        UnbondRequestNotYetDue,
        ///
        AssetError,
        ///
        ZeroVoteWeight
    }
}

impl<T: Trait> From<xp_staking::ZeroVoteWeightError> for Error<T> {
    fn from(e: xp_staking::ZeroVoteWeightError) -> Self {
        Self::ZeroVoteWeight
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {

        type Error = Error<T>;

        fn deposit_event() = default;

        fn on_finalize() {
        }

        /// Claims the staking reward given the `target` validator.
        #[weight = 10]
        fn claim(origin, target: AssetId) {
            let sender = ensure_signed(origin)?;

            ensure!(
                Self::mining_previleged_assets().contains(&target),
                Error::<T>::UnprevilegedAsset
            );

            // <Self as xp_staking::Claim<T::AccountId>>::claim(&sender, &target)?;
        }

    }
}

impl<T: Trait> Module<T> {}
