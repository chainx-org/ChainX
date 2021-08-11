use frame_support::{decl_module, decl_storage};
use sp_std::prelude::*;

#[cfg(feature = "std")]
use xp_genesis_builder::AllParams;
#[cfg(feature = "std")]
use xpallet_assets::BalanceOf as AssetBalanceOf;
#[cfg(feature = "std")]
use xpallet_mining_staking::BalanceOf as StakingBalanceOf;

#[cfg(feature = "std")]
mod genesis;

pub trait Trait:
    pallet_balances::Trait + xpallet_mining_asset::Trait + xpallet_mining_staking::Trait
{
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {}
}

decl_storage! {
    trait Store for Module<T: Trait> as XGenesisBuilder {}
    add_extra_genesis {
        config(params): AllParams<T::AccountId, T::Balance, AssetBalanceOf<T>, StakingBalanceOf<T>>;
        config(root_endowed): T::Balance;
        config(initial_authorities_endowed): T::Balance;
        build(|config| {
            genesis::initialize(config);
        })
    }
}
