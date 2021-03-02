// Copyright 2021 ChainX Project Authors. Licensed under GPL-3.0.

//! This module implements Bitcoin Bridge V2.
//!
//! Bitcoin Bridge provides decentralized functionalities to manage digital assets between
//! Bitcoin and ChainX.
//!
//! ## Terminology:
//!
//! *collateral*: PCX that reserved by bridge, which backs X-BTC.
//!
//! *vault*: Account that locks collateral in bridge, and is able to accept issue requesting by
//! other accounts.
//!
//! *issue*: Operation that transfer BTC to a vault and issue equivalent X-BTC in ChainX.
//!
//! *redeem*: Opposite operation of `issue` that burn X-BTC and receive equivalent BTC in Bitcoin.
//!
//! *exchange rate oracle*: Role that updates exchange rate between BTC and PCX.

#![cfg_attr(not(feature = "std"), no_std)]

pub mod redeem;
pub(crate) mod types;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[frame_support::pallet]
#[allow(dead_code)]
pub mod pallet {
    use sp_arithmetic::{traits::SaturatedConversion, Percent};
    use sp_std::{marker::PhantomData, str::from_utf8, vec::Vec};

    #[cfg(feature = "std")]
    use frame_support::traits::GenesisBuild;

    use frame_support::{
        dispatch::{DispatchError, DispatchResult, DispatchResultWithPostInfo},
        ensure,
        storage::types::{StorageMap, StorageValue, ValueQuery},
        traits::{Currency, Get, Hooks, IsType, ReservableCurrency},
        Blake2_128Concat, Twox64Concat,
    };
    use frame_system::{
        ensure_root, ensure_signed,
        pallet_prelude::{BlockNumberFor, OriginFor},
    };

    use chainx_primitives::AssetId;

    use crate::types::*;

    pub(crate) type BalanceOf<T> = <<T as xpallet_assets::Config>::Currency as Currency<
        <T as frame_system::Config>::AccountId,
    >>::Balance;

    pub(crate) type CurrencyOf<T> = <T as xpallet_assets::Config>::Currency;

    #[allow(type_alias_bounds)]
    pub(crate) type DefaultVault<T: Config> = Vault<T::AccountId, BlockNumberFor<T>, BalanceOf<T>>;

    pub(crate) type AddrStr = Vec<u8>;

    pub(crate) type IssueRequest<T> = crate::types::IssueRequest<
        <T as frame_system::Config>::AccountId,
        <T as frame_system::Config>::BlockNumber,
        BalanceOf<T>,
        BalanceOf<T>,
    >;

    pub(crate) type RequestId = u128;

    #[pallet::pallet]
    #[pallet::generate_store(pub(crate) trait Store)]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::config]
    pub trait Config: frame_system::Config + xpallet_assets::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
        type TargetAssetId: Get<AssetId>;
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(n: BlockNumberFor<T>) -> frame_support::weights::Weight {
            let height = Self::exchange_rate_update_time();
            let period = Self::exchange_rate_expired_period();
            if n - height > period {
                BridgeStatus::<T>::put(Status::Error(ErrorCode::EXCHANGE_RATE_EXPIRED));
            };
            0u64.into()
        }

        fn on_finalize(_: BlockNumberFor<T>) {
            // recover from error if all errors were solved.
            if let Status::Error(ErrorCode::NONE) = Self::bridge_status() {
                BridgeStatus::<T>::put(Status::Running);
            }

            // check vaults' collateral ratio
            if Self::is_bridge_running() {
                for (id, vault) in Vaults::<T>::iter() {
                    if Self::_check_vault_liquidated(&vault) {
                        let _ = Self::liquidate_vault(&id);
                    }
                }
            }
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Update exchange rate by oracle.
        #[pallet::weight(0)]
        pub(crate) fn update_exchange_rate(
            origin: OriginFor<T>,
            exchange_rate: TradingPrice,
        ) -> DispatchResultWithPostInfo {
            let sender = ensure_signed(origin)?;
            ensure!(Self::is_oracle(&sender), Error::<T>::OperationForbidden);
            Self::_update_exchange_rate(exchange_rate.clone())?;
            Self::deposit_event(Event::<T>::ExchangeRateUpdated(sender, exchange_rate));
            Ok(().into())
        }

        /// Force update the exchange rate.
        #[pallet::weight(0)]
        pub(crate) fn force_update_exchange_rate(
            origin: OriginFor<T>,
            exchange_rate: TradingPrice,
        ) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;
            Self::_update_exchange_rate(exchange_rate.clone())?;
            Self::deposit_event(Event::<T>::ExchangeRateForceUpdated(exchange_rate));
            Ok(().into())
        }

        /// Force update the exchange rate expired period.
        #[pallet::weight(0)]
        pub(crate) fn force_update_exchange_rate_expired_period(
            origin: OriginFor<T>,
            expired_period: BlockNumberFor<T>,
        ) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;
            ExchangeRateExpiredPeriod::<T>::put(expired_period);
            Self::deposit_event(Event::<T>::ExchangeRateExpiredPeriodForceUpdated(
                expired_period,
            ));
            Ok(().into())
        }

        /// Force update oracles.
        #[pallet::weight(0)]
        pub(crate) fn force_update_oracles(
            origin: OriginFor<T>,
            oracles: Vec<T::AccountId>,
        ) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;
            OracleAccounts::<T>::put(oracles.clone());
            Self::deposit_event(Event::<T>::OracleForceUpdated(oracles));
            Ok(().into())
        }

        /// Register a vault.
        #[pallet::weight(0)]
        pub(crate) fn register_vault(
            origin: OriginFor<T>,
            collateral: BalanceOf<T>,
            btc_address: AddrStr,
        ) -> DispatchResultWithPostInfo {
            let sender = ensure_signed(origin)?;
            let btc_address = from_utf8(&btc_address)
                .map_err(|_| Error::<T>::InvalidAddress)?
                .parse()
                .map_err(|_| Error::<T>::InvalidAddress)?;
            ensure!(
                collateral >= Self::minimium_vault_collateral(),
                Error::<T>::InsufficientVaultCollateralAmount
            );
            ensure!(
                !Self::vault_exists(&sender),
                Error::<T>::VaultAlreadyRegistered
            );
            ensure!(
                !Self::btc_address_exists(&btc_address),
                Error::<T>::BtcAddressOccupied
            );
            Self::lock_collateral(&sender, collateral)?;
            Self::increase_total_collateral(collateral);
            Self::insert_btc_address(&btc_address, sender.clone());
            let vault = Vault::new(sender.clone(), btc_address);
            Self::insert_vault(&sender, vault.clone());
            Self::deposit_event(Event::VaultRegistered(vault.id, collateral));
            Ok(().into())
        }

        /// Add extra collateral for registered vault.
        #[pallet::weight(0)]
        pub(crate) fn add_extra_collateral(
            origin: OriginFor<T>,
            collateral: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {
            let sender = ensure_signed(origin)?;
            ensure!(Self::vault_exists(&sender), Error::<T>::VaultNotFound);
            Self::lock_collateral(&sender, collateral)?;
            Self::increase_total_collateral(collateral);
            Self::deposit_event(Event::ExtraCollateralAdded(sender, collateral));
            Ok(().into())
        }

        /// User request issue xbtc
        ///
        /// `IssueRequest` couldn't be submitted while bridge during liquidating.
        #[pallet::weight(0)]
        pub fn request_issue(
            origin: OriginFor<T>,
            vault_id: T::AccountId,
            btc_amount: BalanceOf<T>,
            collateral: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {
            Self::ensure_bridge_running()?;

            let sender = ensure_signed(origin)?;
            let height = <frame_system::Pallet<T>>::block_number();
            let vault = Self::get_active_vault_by_id(&vault_id)?;
            let vault_collateral = Self::reserved_balance_of(&vault_id);

            // check if vault is rich enough
            let collateral_ratio_after_requesting = Self::calculate_collateral_ratio(
                vault.issued_tokens + vault.to_be_issued_tokens + btc_amount,
                vault_collateral,
            )?;
            ensure!(
                collateral_ratio_after_requesting >= Self::secure_threshold(),
                Error::<T>::InsecureVault
            );

            let required_collateral = Self::calculate_required_collateral(btc_amount)?;
            ensure!(
                collateral >= required_collateral,
                Error::<T>::InsufficientGriefingCollateral
            );

            // insert `IssueRequest` to request map
            Self::lock_collateral(&sender, collateral)?;
            let request_id = Self::get_next_request_id();
            Self::insert_issue_request(
                request_id,
                IssueRequest::<T> {
                    vault: vault.id.clone(),
                    open_time: height,
                    requester: sender,
                    btc_address: vault.wallet,
                    btc_amount,
                    griefing_collateral: collateral,
                    ..Default::default()
                },
            );
            Vaults::<T>::mutate(&vault.id, |vault| {
                if let Some(vault) = vault {
                    vault.to_be_issued_tokens += btc_amount;
                }
            });
            Self::deposit_event(Event::<T>::IssueRequestSubmitted);
            Ok(().into())
        }

        #[pallet::weight(0)]
        pub fn execute_issue(
            origin: OriginFor<T>,
            request_id: RequestId,
            _tx_id: Vec<u8>,
            _merkle_proof: Vec<u8>,
            _raw_tx: Vec<u8>,
        ) -> DispatchResultWithPostInfo {
            Self::ensure_bridge_running()?;

            let _sender = ensure_signed(origin)?;

            //TODO(wangyafei): verify tx

            let issue_request = Self::get_issue_request_by_id(request_id)
                .ok_or(Error::<T>::IssueRequestNotFound)?;

            ensure!(
                !issue_request.completed && !issue_request.cancelled,
                Error::<T>::IssueRequestDealt
            );

            let height = frame_system::Pallet::<T>::block_number();
            ensure!(
                height - issue_request.open_time < Self::issue_request_expired_time(),
                Error::<T>::IssueRequestExpired
            );

            <xpallet_assets::Module<T>>::issue(
                /*FIME(wangyafei): use associated type*/
                &1,
                &issue_request.requester,
                issue_request.btc_amount,
            )?;
            Vaults::<T>::mutate(&issue_request.vault, |vault| {
                if let Some(vault) = vault {
                    vault.to_be_issued_tokens -= issue_request.btc_amount;
                    vault.issued_tokens += issue_request.btc_amount;
                }
            });
            Self::release_collateral(&issue_request.requester, issue_request.griefing_collateral)?;

            IssueRequests::<T>::mutate(request_id, |issue_request| {
                if let Some(issue_request) = issue_request {
                    issue_request.completed = true
                }
            });
            Self::deposit_event(Event::<T>::IssueRequestExcuted);
            Ok(().into())
        }

        #[pallet::weight(0)]
        pub fn cancel_issue(
            origin: OriginFor<T>,
            request_id: RequestId,
        ) -> DispatchResultWithPostInfo {
            ensure_signed(origin)?;

            let issue_request = Self::get_issue_request_by_id(request_id)
                .ok_or(Error::<T>::IssueRequestNotFound)?;
            ensure!(
                !issue_request.completed && !issue_request.cancelled,
                Error::<T>::IssueRequestDealt
            );

            let height = <frame_system::Pallet<T>>::block_number();
            let expired_time = <IssueRequestExpiredTime<T>>::get();
            ensure!(
                height - issue_request.open_time > expired_time,
                Error::<T>::IssueRequestNotExpired
            );

            let slashed_collateral = Self::calculate_slashed_collateral(issue_request.btc_amount)?;

            Self::slash_collateral(
                &issue_request.vault,
                &issue_request.requester,
                slashed_collateral,
            )?;

            Self::release_collateral(&issue_request.requester, issue_request.griefing_collateral)?;

            Vaults::<T>::mutate(&issue_request.vault, |vault| {
                if let Some(vault) = vault {
                    vault.to_be_issued_tokens -= issue_request.btc_amount;
                }
            });

            IssueRequests::<T>::mutate(request_id, |issue_request| {
                if let Some(issue_request) = issue_request {
                    issue_request.cancelled = true
                }
            });

            Self::deposit_event(Event::<T>::IssueRequestCancelled);
            Ok(().into())
        }

        /// Update expired time for requesting issue
        #[pallet::weight(0)]
        pub fn update_expired_time(
            origin: OriginFor<T>,
            expired_time: BlockNumberFor<T>,
        ) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;
            <IssueRequestExpiredTime<T>>::put(expired_time);
            Self::deposit_event(Event::<T>::ExpiredTimeUpdated);
            Ok(().into())
        }

        /// Update griefing fee for requesting issue
        #[pallet::weight(0)]
        pub fn update_griefing_fee(
            origin: OriginFor<T>,
            griefing_fee: Percent,
        ) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;
            <IssueGriefingFee<T>>::put(griefing_fee);
            Self::deposit_event(Event::<T>::GriefingFeeUpdated);
            Ok(().into())
        }
    }

    /// Events in xbridge module
    ///
    /// Emit when extrinsics or some important operators, like releasing/locking collateral,
    /// move/transfer balance, etc, have happened.
    #[pallet::event]
    #[pallet::generate_deposit(pub(crate) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Update exchange rate by oracle
        ExchangeRateUpdated(T::AccountId, TradingPrice),
        /// Update exchange rate by root
        ExchangeRateForceUpdated(TradingPrice),
        /// Update oracles by root
        OracleForceUpdated(Vec<T::AccountId>),
        /// Collateral was slashed. [from, to, amount]
        BridgeCollateralSlashed(T::AccountId, T::AccountId, BalanceOf<T>),
        // The collateral was released to the user successfully. [who, amount]
        BridgeCollateralReleased(T::AccountId, BalanceOf<T>),
        // Update `ExchangeRateExpiredPeriod`
        ExchangeRateExpiredPeriodForceUpdated(BlockNumberFor<T>),
        /// New vault has been registered.
        VaultRegistered(<T as frame_system::Config>::AccountId, BalanceOf<T>),
        /// Extra collateral was added to a vault.
        ExtraCollateralAdded(<T as frame_system::Config>::AccountId, BalanceOf<T>),
        /// Vault released collateral.
        CollateralReleased(<T as frame_system::Config>::AccountId, BalanceOf<T>),
        // TODO(wangyafei): add details
        // An issue request was submitted and waiting user to excute.
        IssueRequestSubmitted,
        // `IssueRequest` excuted.
        IssueRequestExcuted,
        // `IssueRequest` cancelled.`
        IssueRequestCancelled,
        // Root updated `IssueRequestExpiredTime`.
        ExpiredTimeUpdated,
        // Root updated `IssueGriefingFee`.
        GriefingFeeUpdated,
    }

    /// Errors for assets module
    #[pallet::error]
    pub enum Error<T> {
        /// Permission denied.
        OperationForbidden,
        /// Requester doesn't have enough pcx for collateral.
        InsufficientFunds,
        /// Arithmetic underflow/overflow.
        ArithmeticError,
        /// Account doesn't have enough collateral to be slashed.
        InsufficientCollateral,
        /// Bridge was shutdown or in error.
        BridgeNotRunning,
        /// Try to calculate collateral ratio while has no issued_tokens
        NoIssuedTokens,
        /// The amount in request is less than lower bound.
        InsufficientVaultCollateralAmount,
        /// Collateral is less than lower bound after extrinsic.
        InsufficientVaultCollateral,
        /// Requester has been vault.
        VaultAlreadyRegistered,
        /// Btc address in request was occupied by another vault.
        BtcAddressOccupied,
        /// Vault does not exist.
        VaultNotFound,
        /// Vault was inactive
        VaultInactive,
        /// BtcAddress invalid
        InvalidAddress,
        /// Collateral in request is less than griefing collateral
        InsufficientGriefingCollateral,
        /// No such `IssueRequest`
        IssueRequestNotFound,
        /// `IssueRequest` cancelled when it's not expired
        IssueRequestNotExpired,
        /// Value to be set is invalid
        InvalidConfigValue,
        /// Tried to execute `IssueRequest` while  it's expired
        IssueRequestExpired,
        /// Vault colateral ratio was below than `SecureThreshold`
        InsecureVault,
        /// `IssueRequest` has been excuted or cancelled
        IssueRequestDealt,
    }

    /// Total collateral locked by xbridge.
    #[pallet::storage]
    #[pallet::getter(fn total_collateral)]
    pub(crate) type TotalCollateral<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

    /// Exchange rate from pcx to btc.
    #[pallet::storage]
    #[pallet::getter(fn exchange_rate)]
    pub(crate) type ExchangeRate<T: Config> = StorageValue<_, TradingPrice, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn oracle_accounts)]
    pub(crate) type OracleAccounts<T: Config> = StorageValue<_, Vec<T::AccountId>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn bridge_status)]
    pub(crate) type BridgeStatus<T: Config> = StorageValue<_, Status, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn exchange_rate_update_time)]
    pub(crate) type ExchangeRateUpdateTime<T: Config> =
        StorageValue<_, BlockNumberFor<T>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn exchange_rate_expired_period)]
    pub(crate) type ExchangeRateExpiredPeriod<T: Config> =
        StorageValue<_, BlockNumberFor<T>, ValueQuery>;

    /// Mapping account to vault struct.
    #[pallet::storage]
    pub(crate) type Vaults<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        Vault<T::AccountId, T::BlockNumber, BalanceOf<T>>,
    >;

    /// Mapping btc address to vault id.
    #[pallet::storage]
    pub(crate) type BtcAddresses<T: Config> = StorageMap<_, Twox64Concat, BtcAddress, T::AccountId>;

    /// Lower bound for registering vault or withdrawing collateral.
    #[pallet::storage]
    #[pallet::getter(fn minimium_vault_collateral)]
    pub(crate) type MinimiumVaultCollateral<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

    /// Secure threshold for vault
    /// eg, 200 means 200%.
    #[pallet::storage]
    #[pallet::getter(fn secure_threshold)]
    pub(crate) type SecureThreshold<T: Config> = StorageValue<_, u16, ValueQuery>;

    /// Secure threshold for vault
    /// eg, 150 means 150%.
    #[pallet::storage]
    #[pallet::getter(fn premium_threshold)]
    pub(crate) type PremiumThreshold<T: Config> = StorageValue<_, u16, ValueQuery>;

    /// Secure threshold for vault.
    /// eg, 100 means 100%.
    #[pallet::storage]
    #[pallet::getter(fn liquidation_threshold)]
    pub(crate) type LiquidationThreshold<T: Config> = StorageValue<_, u16, ValueQuery>;

    /// Specicial `SystemVault`
    #[pallet::storage]
    #[pallet::getter(fn liquidator)]
    pub(crate) type Liquidator<T: Config> =
        StorageValue<_, SystemVault<T::AccountId, BalanceOf<T>>, ValueQuery>;

    /// Liquidator account id
    #[pallet::storage]
    #[pallet::getter(fn liquidator_id)]
    pub(crate) type LiquidatorId<T: Config> = StorageValue<_, T::AccountId, ValueQuery>;

    /// Percentage to lock, when user requests issue
    #[pallet::storage]
    #[pallet::getter(fn issue_griefing_fee)]
    pub(crate) type IssueGriefingFee<T: Config> = StorageValue<_, Percent, ValueQuery>;

    /// Auto-increament id to identify each issue request.
    /// Also presents total amount of created requests.
    #[pallet::storage]
    pub(crate) type RequestCount<T: Config> = StorageValue<_, RequestId, ValueQuery>;

    /// Mapping from issue id to `IssueRequest`
    #[pallet::storage]
    pub(crate) type IssueRequests<T: Config> =
        StorageMap<_, Twox64Concat, RequestId, IssueRequest<T>>;

    /// Expired time for an `IssueRequest`
    #[pallet::storage]
    #[pallet::getter(fn issue_request_expired_time)]
    pub(crate) type IssueRequestExpiredTime<T: Config> =
        StorageValue<_, BlockNumberFor<T>, ValueQuery>;

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        pub(crate) exchange_rate: TradingPrice,
        pub(crate) oracle_accounts: Vec<T::AccountId>,
        pub(crate) minimium_vault_collateral: u32,
        pub(crate) secure_threshold: u16,
        pub(crate) premium_threshold: u16,
        pub(crate) liquidation_threshold: u16,
        pub(crate) liquidator_id: T::AccountId,
        pub(crate) issue_griefing_fee: Percent,
        pub(crate) expired_time: BlockNumberFor<T>,
    }

    #[cfg(feature = "std")]
    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            Self {
                exchange_rate: Default::default(),
                oracle_accounts: Default::default(),
                minimium_vault_collateral: Default::default(),
                secure_threshold: 180,
                premium_threshold: 250,
                liquidation_threshold: 300,
                liquidator_id: Default::default(),
                issue_griefing_fee: Default::default(),
                expired_time: Default::default(),
            }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
        fn build(&self) {
            let pcx: BalanceOf<T> = self.minimium_vault_collateral.into();
            <MinimiumVaultCollateral<T>>::put(pcx);

            <ExchangeRate<T>>::put(self.exchange_rate.clone());
            <OracleAccounts<T>>::put(self.oracle_accounts.clone());
            <SecureThreshold<T>>::put(self.secure_threshold);
            <PremiumThreshold<T>>::put(self.premium_threshold);
            <LiquidationThreshold<T>>::put(self.liquidation_threshold);
            <Liquidator<T>>::put(SystemVault {
                id: self.liquidator_id.clone(),
                ..Default::default()
            });
            <LiquidatorId<T>>::put(self.liquidator_id.clone());
            <IssueGriefingFee<T>>::put(self.issue_griefing_fee);
            //FIXME(wangyafei): should put expired time?
        }
    }

    impl<T: Config> Pallet<T> {
        pub fn convert_to_pcx(btc_amount: BalanceOf<T>) -> Result<BalanceOf<T>, DispatchError> {
            //TODO(wangyafei): add lower bound?
            let exchange_rate = Self::exchange_rate();
            let result = exchange_rate
                .convert_to_pcx(btc_amount.saturated_into())
                .ok_or(Error::<T>::ArithmeticError)?;
            Ok(result.saturated_into())
        }
        /// Lock collateral
        #[inline]
        pub fn lock_collateral(sender: &T::AccountId, amount: BalanceOf<T>) -> DispatchResult {
            <<T as xpallet_assets::Config>::Currency as ReservableCurrency<
                <T as frame_system::Config>::AccountId,
            >>::reserve(sender, amount)
            .map_err(|_| Error::<T>::InsufficientFunds)?;
            <TotalCollateral<T>>::mutate(|total| *total += amount);
            Ok(())
        }

        /// increase total collateral
        #[inline]
        pub fn increase_total_collateral(amount: BalanceOf<T>) {
            <TotalCollateral<T>>::mutate(|c| *c += amount);
        }

        #[inline]
        pub(crate) fn is_oracle(account: &T::AccountId) -> bool {
            let oracles: Vec<T::AccountId> = Self::oracle_accounts();
            oracles.contains(account)
        }

        pub(crate) fn _update_exchange_rate(exchange_rate: TradingPrice) -> DispatchResult {
            // TODO: sanity check?
            <ExchangeRate<T>>::put(exchange_rate);
            let height = <frame_system::Pallet<T>>::block_number();
            <ExchangeRateUpdateTime<T>>::put(height);
            Self::recover_from_exchange_rate_expired();
            Ok(())
        }

        /// Slash collateral to receiver
        pub fn slash_collateral(
            sender: &T::AccountId,
            receiver: &T::AccountId,
            amount: BalanceOf<T>,
        ) -> DispatchResult {
            let reserved_collateral = <CurrencyOf<T>>::reserved_balance(sender);
            ensure!(
                reserved_collateral >= amount,
                Error::<T>::InsufficientCollateral
            );
            let (slashed, _) = <CurrencyOf<T>>::slash_reserved(sender, amount);

            <CurrencyOf<T>>::resolve_creating(receiver, slashed);
            <CurrencyOf<T>>::reserve(receiver, amount)
                .map_err(|_| Error::<T>::InsufficientFunds)?;
            Self::deposit_event(Event::<T>::BridgeCollateralSlashed(
                sender.clone(),
                receiver.clone(),
                amount,
            ));
            Ok(().into())
        }

        /// Release collateral
        pub fn release_collateral(account: &T::AccountId, amount: BalanceOf<T>) -> DispatchResult {
            let reserved_collateral = <CurrencyOf<T>>::reserved_balance(account);
            ensure!(
                reserved_collateral >= amount,
                Error::<T>::InsufficientCollateral
            );
            <CurrencyOf<T>>::unreserve(account, amount);
            <TotalCollateral<T>>::mutate(|total| *total -= amount);
            Self::deposit_event(Event::<T>::BridgeCollateralReleased(
                account.clone(),
                amount,
            ));
            Ok(())
        }

        /// Get if the bridge running
        pub fn is_bridge_running() -> bool {
            Self::bridge_status() == Status::Running
        }

        pub fn calculate_collateral_ratio(
            issued_tokens: BalanceOf<T>,
            collateral: BalanceOf<T>,
        ) -> Result<u16, DispatchError> {
            let issued_tokens = issued_tokens.saturated_into::<u128>();
            let collateral = collateral.saturated_into::<u128>();
            ensure!(issued_tokens != 0, Error::<T>::NoIssuedTokens);

            let exchange_rate: TradingPrice = Self::exchange_rate();
            let equivalence_collateral = exchange_rate
                .convert_to_pcx(issued_tokens)
                .ok_or(Error::<T>::ArithmeticError)?;
            let raw_collateral: u128 = collateral.saturated_into();
            let collateral_ratio = raw_collateral
                .checked_mul(100)
                .ok_or(Error::<T>::ArithmeticError)?
                .checked_div(equivalence_collateral)
                .ok_or(Error::<T>::ArithmeticError)?;
            //FIXME(wangyafei): should use try_into?
            Ok(collateral_ratio as u16)
        }

        #[inline]
        pub(crate) fn ensure_bridge_running() -> DispatchResult {
            ensure!(
                Self::bridge_status() == Status::Running,
                Error::<T>::BridgeNotRunning
            );
            Ok(())
        }

        /// Clarify `ExchangeRateExpired` is solved and recover from this error.
        ///
        /// Dangerous! Ensure this error truly solved is caller's responsibility.
        pub(crate) fn recover_from_exchange_rate_expired() {
            if let Status::Error(mut error_codes) = Self::bridge_status() {
                if error_codes.contains(ErrorCode::EXCHANGE_RATE_EXPIRED) {
                    error_codes.remove(ErrorCode::EXCHANGE_RATE_EXPIRED);
                    <BridgeStatus<T>>::put(Status::Error(error_codes))
                }
            }
        }

        /// Clarify `Liquidating` is solved and recover from this error.
        ///
        /// Dangerous! Ensure this error truly solved is caller's responsibility.
        pub(crate) fn recover_from_liquidating() {
            if let Status::Error(mut error_codes) = Self::bridge_status() {
                if error_codes.contains(ErrorCode::LIQUIDATING) {
                    error_codes.remove(ErrorCode::LIQUIDATING);
                    <BridgeStatus<T>>::put(Status::Error(error_codes))
                }
            }
        }

        pub(crate) fn reserved_balance_of(who: &T::AccountId) -> BalanceOf<T> {
            CurrencyOf::<T>::reserved_balance(who)
        }

        #[inline]
        pub fn insert_vault(
            sender: &T::AccountId,
            vault: Vault<T::AccountId, T::BlockNumber, BalanceOf<T>>,
        ) {
            <Vaults<T>>::insert(sender, vault);
        }

        #[inline]
        pub fn insert_btc_address(address: &BtcAddress, vault_id: T::AccountId) {
            <BtcAddresses<T>>::insert(address, vault_id);
        }

        #[inline]
        pub fn vault_exists(id: &T::AccountId) -> bool {
            <Vaults<T>>::contains_key(id)
        }

        #[inline]
        pub fn btc_address_exists(address: &BtcAddress) -> bool {
            <BtcAddresses<T>>::contains_key(address)
        }

        pub fn get_vault_by_id(
            id: &T::AccountId,
        ) -> Result<Vault<T::AccountId, T::BlockNumber, BalanceOf<T>>, DispatchError> {
            match <Vaults<T>>::get(id) {
                Some(vault) => Ok(vault),
                None => Err(Error::<T>::VaultNotFound.into()),
            }
        }

        pub fn get_active_vault_by_id(
            id: &T::AccountId,
        ) -> Result<Vault<T::AccountId, T::BlockNumber, BalanceOf<T>>, DispatchError> {
            let vault = Self::get_vault_by_id(id)?;
            if vault.status == VaultStatus::Active {
                Ok(vault)
            } else {
                Err(Error::<T>::VaultInactive.into())
            }
        }

        /// Liquidate vault and mark it as `Liquidated`
        ///
        /// Liquidated vault cannot be updated.
        pub(crate) fn liquidate_vault(id: &T::AccountId) -> Result<(), DispatchError> {
            <Vaults<T>>::mutate(id, |vault| match vault {
                Some(ref mut vault) => {
                    if vault.status == VaultStatus::Active {
                        vault.status = VaultStatus::Liquidated;
                        Ok(())
                    } else {
                        Err(Error::<T>::VaultInactive)
                    }
                }
                None => Err(Error::<T>::VaultNotFound),
            })?;

            let vault = Self::get_vault_by_id(id)?;
            let collateral = CurrencyOf::<T>::reserved_balance(&vault.id);
            Self::slash_collateral(&vault.id, &Self::liquidator_id(), collateral)?;

            <Liquidator<T>>::mutate(|liquidator| {
                liquidator.issued_tokens += vault.issued_tokens;
                liquidator.to_be_issued_tokens += vault.to_be_issued_tokens;
                liquidator.to_be_redeemed_tokens += vault.to_be_redeemed_tokens;
            });
            Ok(())
        }

        pub(crate) fn _check_vault_liquidated(vault: &DefaultVault<T>) -> bool {
            if vault.issued_tokens == 0.into() {
                return false;
            }
            let collateral = CurrencyOf::<T>::reserved_balance(&vault.id);
            Self::calculate_collateral_ratio(vault.issued_tokens, collateral)
                .map(|collateral_ratio| collateral_ratio < Self::liquidation_threshold())
                .unwrap_or(false)
        }

        pub(crate) fn insert_issue_request(key: u128, value: IssueRequest<T>) {
            <IssueRequests<T>>::insert(&key, value)
        }

        /// generate secure key from account id
        pub(crate) fn get_next_request_id() -> RequestId {
            <RequestCount<T>>::mutate(|n| {
                *n += 1;
                *n
            })
        }

        /// Calculate minimium required collateral for a `IssueRequest`
        pub(crate) fn calculate_required_collateral(
            btc_amount: BalanceOf<T>,
        ) -> Result<BalanceOf<T>, DispatchError> {
            let pcx_amount = Self::convert_to_pcx(btc_amount)?;
            let percentage = Self::issue_griefing_fee();
            let griefing_fee = percentage.mul_ceil(pcx_amount);
            Ok(griefing_fee)
        }

        /// Get `IssueRequest` from id
        pub(crate) fn get_issue_request_by_id(request_id: RequestId) -> Option<IssueRequest<T>> {
            <IssueRequests<T>>::get(request_id)
        }

        /// Calculate slashed amount.
        ///
        /// Equals the corresponding pcx times secure threshold
        pub(crate) fn calculate_slashed_collateral(
            btc_amount: BalanceOf<T>,
        ) -> Result<BalanceOf<T>, DispatchError> {
            let pcx_amount = Self::convert_to_pcx(btc_amount)?;
            let secure_threshold = Self::secure_threshold();
            let slashed_collateral: u32 =
                (pcx_amount.saturated_into::<u128>() * secure_threshold as u128 / 100) as u32;
            Ok(slashed_collateral.into())
        }
    }
}
