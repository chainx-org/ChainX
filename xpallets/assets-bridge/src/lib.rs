// Copyright 2019-2022 ChainX Project Authors. Licensed under GPL-3.0.

//! # Assets Bridge
//!
//! ## Overview
//!
//! Bridge between pallet-assets and Erc20 tokens

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(dead_code)]

pub mod abi;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
pub use abi::*;
pub mod recover;
pub use recover::*;

use codec::Encode;
use frame_support::{
    ensure,
    pallet_prelude::*,
    traits::{Currency, ExistenceRequirement, IsType},
    transactional,
};
use sp_core::{ecdsa, H160, U256};
use sp_io::{crypto::secp256k1_ecdsa_recover, hashing::keccak_256};
use sp_runtime::traits::{StaticLookup, UniqueSaturatedInto, Zero};
use sp_std::vec::Vec;

pub use chainx_primitives::AssetId;
use pallet_evm::{AddressMapping, CallInfo, ExitReason, Runner};

pub type EcdsaSignature = ecdsa::Signature;
pub type AddressMappingOf<T> = <T as pallet_evm::Config>::AddressMapping;
pub type BalanceOf<T> = <<T as xpallet_assets::Config>::Currency as Currency<
    <T as frame_system::Config>::AccountId,
>>::Balance;

#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, scale_info::TypeInfo)]
pub enum ActionType {
    Direct(H160),
    FromSubToEth,
    FromEthToSub,
    BackForeign(AssetId),
}

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::traits::ReservableCurrency;
    use frame_system::pallet_prelude::*;

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config + xpallet_assets::Config + pallet_evm::Config {
        /// The overarching event type.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
        /// The assets-bridge's inner evm caller.
        #[pallet::constant]
        type EvmCaller: Get<H160>;
        /// How much should be locked up in order to claim account.
        #[pallet::constant]
        type ClaimBond: Get<BalanceOf<Self>>;
    }

    /// The Substrate Account for Evm Addresses
    ///
    /// SubAccounts: map H160 => Option<AccountId>
    #[pallet::storage]
    #[pallet::getter(fn sub_accounts)]
    pub type SubAccounts<T: Config> = StorageMap<_, Twox64Concat, H160, T::AccountId, OptionQuery>;

    /// The Evm Addresses for Substrate Accounts
    ///
    /// EvmAccounts: map AccountId => Option<H160>
    #[pallet::storage]
    #[pallet::getter(fn evm_accounts)]
    pub type EvmAccounts<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, H160, OptionQuery>;

    /// The Erc20 Contract Addresses for Asset Ids
    ///
    /// Erc20s: map AssetId => Option<H160>
    #[pallet::storage]
    #[pallet::getter(fn erc20s)]
    pub type Erc20s<T: Config> = StorageMap<_, Twox64Concat, AssetId, H160, OptionQuery>;

    /// The Asset Ids for Erc20 Contract Addresses
    ///
    /// AssetIds: map H160 => Option<AssetId>
    #[pallet::storage]
    #[pallet::getter(fn asset_ids)]
    pub type AssetIds<T: Config> = StorageMap<_, Twox64Concat, H160, AssetId, OptionQuery>;

    /// The Assets can back foreign chain
    ///
    /// AssetIds: Vec<AssetId>
    #[pallet::storage]
    #[pallet::getter(fn back_foreign_assets)]
    pub type BackForeign<T: Config> = StorageValue<_, Vec<AssetId>, ValueQuery>;

    /// The pallet admin key.
    #[pallet::storage]
    #[pallet::getter(fn admin_key)]
    pub(super) type Admin<T: Config> = StorageValue<_, T::AccountId>;

    /// The Assets in emergency
    #[pallet::storage]
    #[pallet::getter(fn emergencies)]
    pub(super) type Emergencies<T: Config> = StorageValue<_, Vec<AssetId>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn hot_account)]
    pub(super) type HotAccount<T: Config> = StorageValue<_, T::AccountId>;

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        /// The `AccountId` of the admin key.
        pub admin_key: Option<T::AccountId>,
    }

    #[cfg(feature = "std")]
    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            Self {
                admin_key: Default::default(),
            }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
        fn build(&self) {
            if let Some(key) = &self.admin_key {
                <Admin<T>>::put(key.clone());
            }
        }
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub fn deposit_event)]
    pub enum Event<T: Config> {
        /// (account_id, evm_address)
        ClaimAccount(T::AccountId, H160),
        /// (account_id)
        Dissolve(T::AccountId),
        /// (asset_id, account_id, evm_address, amount, erc20_contract)
        DepositExecuted(AssetId, T::AccountId, H160, BalanceOf<T>, H160),
        /// (asset_id, account_id, evm_address, amount, erc20_contract)
        WithdrawExecuted(AssetId, T::AccountId, H160, BalanceOf<T>, H160),
        /// (account_id, amount, action)
        Teleport(T::AccountId, BalanceOf<T>, ActionType),
        /// (account_id)
        SetAdmin(T::AccountId),
        /// (asset_id, erc20_contract)
        Register(AssetId, H160),
        /// (asset_id, erc20_contract)
        ForceUnRegister(AssetId, H160),
        /// (asset_id)
        Paused(AssetId),
        // (asset_id)
        UnPaused(AssetId),
        PausedAll,
        UnPausedAll,
        // (asset_id, remove)
        BackForeign(AssetId, bool),
    }

    /// Error for evm accounts module.
    #[pallet::error]
    pub enum Error<T> {
        /// AccountId has mapped
        AccountIdHasMapped,
        /// Eth address has mapped
        EthAddressHasMapped,
        /// Bad signature
        BadSignature,
        /// Invalid signature
        InvalidSignature,
        /// AccountId has not mapped
        AccountIdHasNotMapped,
        /// Eth address has not mapped
        EthAddressHasNotMapped,
        /// AssetId has mapped
        AssetIdHasMapped,
        /// AssetId has not mapped
        AssetIdHasNotMapped,
        /// Erc20 contract address has mapped
        ContractAddressHasMapped,
        /// Erc20 contract address has not mapped
        ContractAddressHasNotMapped,
        /// Failed Erc20 contract call
        ExecutedFailed,
        /// Require admin authority
        RequireAdmin,
        /// Ban deposit and withdraw when in emergency
        InEmergency,
        /// Ban back to foreign
        BanBackForeign,
        /// Zero balance
        ZeroBalance,
        /// Deprecated
        Deprecated,
        /// Require hot account authority
        RequireHot,
        /// Hot account not set
        HotAccountNotSet,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T>
    where
        DispatchError: From<<<T as pallet_evm::Config>::Runner as pallet_evm::Runner<T>>::Error>,
    {
        /// Claim account mapping between Substrate accounts and EVM accounts.
        /// Ensure eth_address has not been mapped.
        /// Note: for general users
        ///
        /// - `eth_address`: The address to bind to the caller's account
        /// - `eth_signature`: A signature generated by the address to prove ownership
        #[pallet::weight(100_000_000u64)]
        // #[transactional]
        pub fn claim_account(
            origin: OriginFor<T>,
            eth_address: H160,
            eth_signature: EcdsaSignature,
        ) -> DispatchResult {
            Err(Error::<T>::Deprecated.into())
            // let who = ensure_signed(origin)?;
            //
            // // ensure account_id and eth_address has not been mapped
            // ensure!(
            //     !EvmAccounts::<T>::contains_key(&who),
            //     Error::<T>::AccountIdHasMapped
            // );
            // ensure!(
            //     !SubAccounts::<T>::contains_key(eth_address),
            //     Error::<T>::EthAddressHasMapped
            // );
            //
            // // recover evm address from signature
            // let address = eth_recover(&eth_signature, &who.using_encoded(to_ascii_hex), &[][..])
            //     .ok_or(Error::<T>::BadSignature)?;
            //
            // ensure!(eth_address == address, Error::<T>::InvalidSignature);
            //
            // <T as xpallet_assets::Config>::Currency::reserve(&who, T::ClaimBond::get())?;
            //
            // SubAccounts::<T>::insert(eth_address, &who);
            // EvmAccounts::<T>::insert(&who, eth_address);
            //
            // Self::deposit_event(Event::ClaimAccount(who, eth_address));
            //
            // Ok(())
        }

        /// Dissolve substrate accounts and EVM accounts.
        /// Note: for general users
        #[pallet::weight(100_000_000u64)]
        #[transactional]
        pub fn dissolve(origin: OriginFor<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let evm_account = Self::evm_accounts(&who).ok_or(Error::<T>::EthAddressHasNotMapped)?;

            ensure!(
                SubAccounts::<T>::contains_key(&evm_account),
                Error::<T>::EthAddressHasNotMapped
            );

            <T as xpallet_assets::Config>::Currency::unreserve(&who, T::ClaimBond::get());

            SubAccounts::<T>::remove(&evm_account);
            EvmAccounts::<T>::remove(&who);

            Self::deposit_event(Event::Dissolve(who));

            Ok(())
        }

        /// Deposit substrate assets into evm erc20 contracts.
        /// Note: for general users
        ///
        /// - `asset_id`: The asset id
        /// - `amount`: Deposit amount
        #[pallet::weight(1_000_000u64)]
        #[transactional]
        pub fn deposit(
            origin: OriginFor<T>,
            asset_id: AssetId,
            amount: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {
            Err(Error::<T>::Deprecated.into())
            // let who = ensure_signed(origin)?;
            // ensure!(!Self::is_in_emergency(asset_id), Error::<T>::InEmergency);
            // ensure!(!amount.is_zero(), Error::<T>::ZeroBalance);
            //
            // // 1. check evm account
            // let evm_account = Self::evm_accounts(&who).ok_or(Error::<T>::EthAddressHasNotMapped)?;
            //
            // // 2. burn asset
            // let _ = xpallet_assets::Pallet::<T>::destroy_usable(&asset_id, &who, amount)?;
            //
            // // 3. mint erc20
            // let erc20 = Self::erc20s(asset_id).ok_or(Error::<T>::ContractAddressHasNotMapped)?;
            //
            // let inputs = mint_into_encode(evm_account, amount.unique_saturated_into());
            //
            // Self::call_evm(erc20, inputs)?;
            //
            // Self::deposit_event(Event::DepositExecuted(
            //     asset_id,
            //     who,
            //     evm_account,
            //     amount,
            //     erc20,
            // ));
            //
            // Ok(Pays::No.into())
        }

        /// Withdraw from evm erc20 contracts into substrate assets
        /// Note: for general users
        ///
        /// - `asset_id`: The asset id
        /// - `amount`: Withdraw amount
        #[pallet::weight(1_000_000u64)]
        #[transactional]
        pub fn withdraw(
            origin: OriginFor<T>,
            asset_id: AssetId,
            amount: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {
            Err(Error::<T>::Deprecated.into())
            // let who = ensure_signed(origin)?;
            // ensure!(!Self::is_in_emergency(asset_id), Error::<T>::InEmergency);
            // ensure!(!amount.is_zero(), Error::<T>::ZeroBalance);
            //
            // // 1. check evm account
            // let evm_account = Self::evm_accounts(&who).ok_or(Error::<T>::EthAddressHasNotMapped)?;
            //
            // // 2. burn erc20
            // let erc20 = Self::erc20s(asset_id).ok_or(Error::<T>::ContractAddressHasNotMapped)?;
            //
            // let inputs = burn_from_encode(evm_account, amount.unique_saturated_into());
            //
            // Self::call_evm(erc20, inputs)?;
            //
            // // 3. mint asset
            // let _ = xpallet_assets::Pallet::<T>::issue(&asset_id, &who, amount)?;
            //
            // Self::deposit_event(Event::WithdrawExecuted(
            //     asset_id,
            //     who,
            //     evm_account,
            //     amount,
            //     erc20,
            // ));
            //
            // Ok(Pays::No.into())
        }

        /// Teleport native currency between substrate account and evm address
        /// Ensure eth_address has been mapped
        /// Note: for general users
        ///
        /// - `amount`: Teleport amount
        /// - `action`:
        ///    (1) Direct(H160): direct transfer into unchecked evm address
        ///    (2) FromSubToEth: transfer from substrate account to mapped evm address
        ///    (3) FromEthToSub: transfer from mapped evm address to substrate account
        /// - companion with `relay`:
        ///    (4) BackForeign(asset_id): transfer assets back foreign chain
        #[pallet::weight(1_000_000u64)]
        // #[transactional]
        pub fn teleport(
            origin: OriginFor<T>,
            amount: BalanceOf<T>,
            action: ActionType,
        ) -> DispatchResultWithPostInfo {
            Err(Error::<T>::InEmergency.into())
            // let who = ensure_signed(origin)?;
            // ensure!(!amount.is_zero(), Error::<T>::ZeroBalance);
            //
            // let (from, to, back_foreign) = match action {
            //     ActionType::Direct(unchecked) => (
            //         who.clone(),
            //         AddressMappingOf::<T>::into_account_id(unchecked),
            //         false,
            //     ),
            //     ActionType::FromSubToEth => (
            //         who.clone(),
            //         Self::evm_accounts(&who)
            //             .map(AddressMappingOf::<T>::into_account_id)
            //             .ok_or(Error::<T>::EthAddressHasNotMapped)?,
            //         false,
            //     ),
            //     ActionType::FromEthToSub => (
            //         Self::evm_accounts(&who)
            //             .map(AddressMappingOf::<T>::into_account_id)
            //             .ok_or(Error::<T>::EthAddressHasNotMapped)?,
            //         who.clone(),
            //         false,
            //     ),
            //     ActionType::BackForeign(asset_id) => {
            //         // ensure asset_id registered in back_foreign list
            //         ensure!(
            //             Self::is_in_back_foreign(asset_id),
            //             Error::<T>::BanBackForeign
            //         );
            //         ensure!(!Self::is_in_emergency(asset_id), Error::<T>::InEmergency);
            //
            //         let amount: u128 = amount.unique_saturated_into();
            //         // burn asset first, then relay will transfer back `who`.
            //         let _ = xpallet_assets::Pallet::<T>::destroy_usable(
            //             &asset_id,
            //             &who,
            //             amount.unique_saturated_into(),
            //         )?;
            //
            //         (who.clone(), who.clone(), true)
            //     }
            // };
            //
            // if !back_foreign {
            //     <T as xpallet_assets::Config>::Currency::transfer(
            //         &from,
            //         &to,
            //         amount,
            //         ExistenceRequirement::AllowDeath,
            //     )?;
            // }
            //
            // Self::deposit_event(Event::Teleport(who, amount, action));

            // Ok(Pays::No.into())
        }

        /// Register substrate assets and erc20 contracts
        /// Note: for admin
        ///
        /// - `asset_id`: The asset id
        /// - `erc20`: The erc20 contract address
        #[pallet::weight(100_000_000u64)]
        pub fn register(
            origin: OriginFor<T>,
            asset_id: AssetId,
            erc20: H160,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            ensure!(Some(who) == Self::admin_key(), Error::<T>::RequireAdmin);

            // ensure asset_id and erc20 address has not been mapped
            ensure!(
                !Erc20s::<T>::contains_key(&asset_id),
                Error::<T>::AssetIdHasMapped
            );
            ensure!(
                !AssetIds::<T>::contains_key(&erc20),
                Error::<T>::ContractAddressHasMapped
            );

            Erc20s::<T>::insert(asset_id, erc20);
            AssetIds::<T>::insert(erc20, asset_id);

            Self::deposit_event(Event::Register(asset_id, erc20));

            Ok(Pays::No.into())
        }

        /// Pause assets bridge deposit and withdraw
        /// Note: for admin
        ///
        /// - `asset_id`: None will pause all, Some(id) will pause the specified asset
        #[pallet::weight(100_000_000u64)]
        pub fn pause(
            origin: OriginFor<T>,
            asset_id: Option<AssetId>,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            ensure!(Some(who) == Self::admin_key(), Error::<T>::RequireAdmin);

            Emergencies::<T>::try_mutate(|emergencies| {
                if let Some(id) = asset_id {
                    // ensure asset_id and erc20 address has not been mapped
                    ensure!(
                        Erc20s::<T>::contains_key(&id),
                        Error::<T>::AssetIdHasNotMapped
                    );
                    if !Self::is_in_emergency(id) {
                        emergencies.push(id);

                        Self::deposit_event(Event::Paused(id));
                    }
                } else {
                    emergencies.truncate(0);
                    for id in AssetIds::<T>::iter_values() {
                        emergencies.push(id);
                    }

                    Self::deposit_event(Event::PausedAll);
                }

                Ok(Pays::No.into())
            })
        }

        /// Unpause assets bridge deposit and withdraw
        /// Note: for admin
        ///
        /// - `asset_id`: None will unpause all, Some(id) will unpause the specified asset
        #[pallet::weight(100_000_000u64)]
        pub fn unpause(
            origin: OriginFor<T>,
            asset_id: Option<AssetId>,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            ensure!(Some(who) == Self::admin_key(), Error::<T>::RequireAdmin);

            Emergencies::<T>::try_mutate(|emergencies| {
                if let Some(id) = asset_id {
                    // ensure asset_id and erc20 address has been mapped
                    ensure!(
                        Erc20s::<T>::contains_key(&id),
                        Error::<T>::AssetIdHasNotMapped
                    );

                    if Self::is_in_emergency(id) {
                        emergencies.retain(|&emergency| emergency != id);

                        Self::deposit_event(Event::UnPaused(id));
                    }
                } else {
                    emergencies.truncate(0);

                    Self::deposit_event(Event::UnPausedAll);
                }

                Ok(Pays::No.into())
            })
        }

        /// Add assets which can back add_back_foreign chain
        /// Note: for admin
        ///
        /// - `asset_id`:
        #[pallet::weight(100_000_000u64)]
        pub fn back_foreign(
            origin: OriginFor<T>,
            asset_id: AssetId,
            remove: bool,
        ) -> DispatchResultWithPostInfo {
            Err(Error::<T>::Deprecated.into())
            // let who = ensure_signed(origin)?;
            // ensure!(Some(who) == Self::admin_key(), Error::<T>::RequireAdmin);
            //
            // BackForeign::<T>::try_mutate(|foreigns| {
            //     if remove {
            //         foreigns.retain(|id| *id != asset_id);
            //     } else if !Self::is_in_back_foreign(asset_id) {
            //         foreigns.push(asset_id);
            //     } else {
            //         return Ok(Pays::No.into());
            //     }
            //
            //     Self::deposit_event(Event::BackForeign(asset_id, remove));
            //
            //     Ok(Pays::No.into())
            // })
        }

        /// Set this pallet admin key
        /// Note: for super admin
        #[pallet::weight(100_000_000u64)]
        pub fn set_admin(
            origin: OriginFor<T>,
            new_admin: <T::Lookup as StaticLookup>::Source,
        ) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;
            let new_admin = T::Lookup::lookup(new_admin)?;

            let _ = Self::set_admin_inner(new_admin.clone());

            Self::deposit_event(Event::SetAdmin(new_admin));

            Ok(Pays::No.into())
        }

        /// Force unregister substrate assets and erc20 contracts
        /// Note: for admin
        #[pallet::weight(100_000_000u64)]
        pub fn force_unregister(
            origin: OriginFor<T>,
            asset_id: AssetId,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            ensure!(Some(who) == Self::admin_key(), Error::<T>::RequireAdmin);

            let erc20 = Self::erc20s(&asset_id).ok_or(Error::<T>::AssetIdHasNotMapped)?;

            ensure!(
                AssetIds::<T>::contains_key(&erc20),
                Error::<T>::ContractAddressHasMapped
            );

            Erc20s::<T>::remove(&asset_id);
            AssetIds::<T>::remove(&erc20);

            // clear emergency
            if Self::is_in_emergency(asset_id) {
                Emergencies::<T>::mutate(|emergencies| {
                    emergencies.retain(|&emergency| emergency != asset_id);
                })
            }

            Self::deposit_event(Event::ForceUnRegister(asset_id, erc20));

            Ok(Pays::No.into())
        }

        /// Set the hot account who can receive the pcx while calling `deposit_pcx_to_evm`
        /// Note: for admin
        #[pallet::weight(0u64)]
        pub fn set_hot_account(
            origin: OriginFor<T>,
            new_hot: <T::Lookup as StaticLookup>::Source,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            ensure!(Some(who) == Self::admin_key(), Error::<T>::RequireAdmin);

            let hot_account = T::Lookup::lookup(new_hot)?;

            HotAccount::<T>::mutate(|hot| *hot = Some(hot_account));

            Ok(Pays::No.into())
        }

        /// Deposit PCX from wasm to evm
        /// Note: for user
        #[pallet::weight(0u64)]
        pub fn deposit_pcx_to_evm(
            origin: OriginFor<T>,
            amount: BalanceOf<T>,
            eth_address: H160,
        ) -> DispatchResultWithPostInfo {
            let pcx_asset_id = 0;
            let who = ensure_signed(origin)?;
            ensure!(
                !Self::is_in_emergency(pcx_asset_id),
                Error::<T>::InEmergency
            );
            ensure!(!amount.is_zero(), Error::<T>::ZeroBalance);
            ensure!(Self::hot_account().is_some(), Error::<T>::HotAccountNotSet);

            // 1. transfer pcx to hot address
            <T as xpallet_assets::Config>::Currency::transfer(
                &who,
                &Self::hot_account().unwrap(),
                amount,
                ExistenceRequirement::AllowDeath,
            )?;

            // 2. mint pcx to eth_address in chainx-evm
            let pcx_contract =
                Self::erc20s(pcx_asset_id).ok_or(Error::<T>::ContractAddressHasNotMapped)?;

            let inputs = mint_into_encode(eth_address, amount.unique_saturated_into());

            Self::call_evm(pcx_contract, inputs)?;

            Self::deposit_event(Event::DepositExecuted(
                pcx_asset_id,
                who,
                eth_address,
                amount,
                pcx_contract,
            ));

            Ok(Pays::No.into())
        }

        /// Swap XBTC(assets moudle) to BTC(btc ledger module) by 1:1
        /// Note: for user who hold XBTC
        #[pallet::weight(0u64)]
        pub fn swap_xbtc_to_btc(
            origin: OriginFor<T>,
            amount: u128,
            eth_address: H160,
        ) -> DispatchResultWithPostInfo {
            let xbtc_asset_id = 1;

            let who = ensure_signed(origin)?;
            ensure!(
                !Self::is_in_emergency(xbtc_asset_id),
                Error::<T>::InEmergency
            );
            ensure!(amount > 0, Error::<T>::ZeroBalance);

            // 1. burn useable xbtc from account
            let _ = xpallet_assets::Pallet::<T>::destroy_usable(
                &xbtc_asset_id,
                &who,
                amount.unique_saturated_into(),
            )?;

            // 2. mint btc to mapping account of the evm address
            let mapping_account = AddressMappingOf::<T>::into_account_id(eth_address);
            <T as pallet_evm::Config>::Currency::deposit_creating(
                &mapping_account,
                amount.unique_saturated_into(),
            );

            Ok(Pays::No.into())
        }
    }
}

impl<T: Config> Pallet<T> {
    pub fn set_admin_inner(new_admin: T::AccountId) -> Weight {
        Admin::<T>::mutate(|admin| *admin = Some(new_admin));
        T::DbWeight::get().write
    }

    pub fn apply_direct_deposit(
        evm_account: H160,
        asset_id: AssetId,
        amount: u128,
    ) -> DispatchResult {
        ensure!(!Self::is_in_emergency(asset_id), Error::<T>::InEmergency);
        ensure!(amount > 0, Error::<T>::ZeroBalance);

        let mapping_account = AddressMappingOf::<T>::into_account_id(evm_account);
        <T as pallet_evm::Config>::Currency::deposit_creating(
            &mapping_account,
            amount.unique_saturated_into(),
        );

        Ok(())
    }

    fn call_evm(erc20: H160, inputs: Vec<u8>) -> DispatchResult {
        match T::Runner::call(
            T::EvmCaller::get(),
            erc20,
            inputs,
            U256::default(),
            3_000_000,
            None,
            None,
            None,
            Vec::new(),
            false,
            T::config(),
        ) {
            Ok(CallInfo {
                exit_reason: ExitReason::Succeed(_),
                ..
            }) => Ok(()),
            _ => Err(Error::<T>::ExecutedFailed.into()),
        }
    }

    fn is_in_emergency(asset_id: AssetId) -> bool {
        Self::emergencies()
            .iter()
            .any(|&emergency| emergency == asset_id)
    }

    fn is_in_back_foreign(asset_id: AssetId) -> bool {
        Self::back_foreign_assets().iter().any(|&id| id == asset_id)
    }
}
