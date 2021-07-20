// Copyright 2021 ChainX Project Authors. Licensed under GPL-3.0.

//! This module implements Bitcoin Bridge V2.
//!
//! Bitcoin Bridge provides decentralized functionalities to manage digital assets between
//! Bitcoin and ChainX.
//!
//! - [`Pallet`]
//! - [`Config`]
//! - [`Call`]
//!
//! ## Overview
//! TODO(wangyafei)
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

#[cfg(any(feature = "runtime-benchmarks", test))]
mod benchmarking;
mod collateral;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub mod types;
pub mod weights;

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
        traits::{
            BalanceStatus, Currency, ExistenceRequirement, Get, Hooks, IsType, ReservableCurrency,
        },
        Blake2_128Concat, Twox64Concat,
    };
    use frame_system::{
        ensure_root, ensure_signed,
        pallet_prelude::{BlockNumberFor, OriginFor},
    };

    use chainx_primitives::AssetId;
    use light_bitcoin::keys::MultiAddress;

    use crate::types::*;
    use crate::weights::WeightInfo;

    pub(crate) type BalanceOf<T> = <<T as xpallet_assets::Config>::Currency as Currency<
        <T as frame_system::Config>::AccountId,
    >>::Balance;

    pub(crate) type CurrencyOf<T> = <T as xpallet_assets::Config>::Currency;

    #[allow(type_alias_bounds)]
    pub(crate) type DefaultVault<T: Config> = Vault<BlockNumberFor<T>, BalanceOf<T>>;

    pub(crate) type IssueRequest<T> = crate::types::IssueRequest<
        <T as frame_system::Config>::AccountId,
        <T as frame_system::Config>::BlockNumber,
        BalanceOf<T>,
    >;

    pub(crate) type RequestId = u128;

    pub(crate) type RedeemRequest<T> = crate::types::RedeemRequest<
        <T as frame_system::Config>::AccountId,
        <T as frame_system::Config>::BlockNumber,
        BalanceOf<T>,
    >;

    #[pallet::pallet]
    #[pallet::generate_store(pub(crate) trait Store)]
    pub struct Pallet<T, I = ()>(_);

    #[pallet::config]
    pub trait Config<I: 'static = ()>: frame_system::Config + xpallet_assets::Config {
        type Event: From<Event<Self, I>> + IsType<<Self as frame_system::Config>::Event>;

        /// Target asset id in this chainx bridge instance.
        ///
        /// Each outer bitcoin-like chain has a corresponding instance. The field records the
        /// `AssdtId` of that chain.
        #[pallet::constant]
        type TargetAssetId: Get<AssetId>;
        /// Lower bound of vault's collateral.
        #[pallet::constant]
        type DustCollateral: Get<BalanceOf<Self>>;
        /// Vault considered as secure when his collateral ratio is upper than this.
        #[pallet::constant]
        type SecureThreshold: Get<u16>;
        /// Vault needs to pay additional fee to redeemer when his collateral ratio is below than
        /// this.
        #[pallet::constant]
        type PremiumThreshold: Get<u16>;
        /// Vault will be liquidated if his collateral ratio lower than this.
        ///
        /// See also [liquidating](#Liquidating)
        #[pallet::constant]
        type LiquidationThreshold: Get<u16>;
        /// Duration from `IssueRequest` opened to expired.
        #[pallet::constant]
        type IssueRequestExpiredPeriod: Get<BlockNumberFor<Self>>;
        /// Duration from `RedeemRequest` opened to expired.
        #[pallet::constant]
        type RedeemRequestExpiredPeriod: Get<BlockNumberFor<Self>>;
        /// Duration from `ExchangeRate` last updated to expired.
        #[pallet::constant]
        type ExchangeRateExpiredPeriod: Get<BlockNumberFor<Self>>;
        /// The minimum amount of btc that is accepted for redeem requests; any lower values would
        /// risk the bitcoin client to reject the payment
        #[pallet::constant]
        type MinimumRedeemValue: Get<BalanceOf<Self>>;
        type WeightInfo: WeightInfo;
    }

    #[pallet::hooks]
    impl<T: Config<I>, I: 'static> Hooks<BlockNumberFor<T>> for Pallet<T, I> {
        fn on_initialize(n: BlockNumberFor<T>) -> frame_support::weights::Weight {
            let height = Self::exchange_rate_update_time();
            let period = T::ExchangeRateExpiredPeriod::get();
            if n - height > period {
                BridgeStatus::<T, I>::put(Status::Error(ErrorCode::EXCHANGE_RATE_EXPIRED));
            };

            0u64
        }

        fn on_finalize(_: BlockNumberFor<T>) {
            // recover from error if all errors were solved.
            if let Status::Error(ErrorCode::NONE) = Self::bridge_status() {
                BridgeStatus::<T, I>::put(Status::Running);
            }
        }
    }

    #[pallet::call]
    impl<T: Config<I>, I: 'static> Pallet<T, I> {
        /// Update exchange rate by oracle.
        ///
        /// The extrinsic only allows oracle accounts.
        ///
        /// *Relative Functions*:
        /// [`force_update_exchange_rate`](crate::Pallet::force_update_exchange_rate)
        #[pallet::weight(<T as Config<I>>::WeightInfo::update_exchange_rate())]
        pub(crate) fn update_exchange_rate(
            origin: OriginFor<T>,
            exchange_rate: TradingPrice,
        ) -> DispatchResultWithPostInfo {
            let sender = ensure_signed(origin)?;
            ensure!(
                Self::oracle_accounts().contains(&sender),
                Error::<T, I>::NotOracle
            );
            Self::inner_update_exchange_rate(exchange_rate.clone())?;
            Self::deposit_event(Event::<T, I>::ExchangeRateUpdated(sender, exchange_rate));
            Ok(().into())
        }

        /// Register a vault with collateral and unique `btc_address`.
        ///
        /// The extrinsic's origin must be signed.
        /// *Relative Functions*:
        /// [`add_extra_collateral`](crate::Pallet::add_extra_collateral)
        #[pallet::weight(<T as Config<I>>::WeightInfo::register_vault())]
        pub(crate) fn register_vault(
            origin: OriginFor<T>,
            collateral: BalanceOf<T>,
            addr_str: Vec<u8>,
        ) -> DispatchResultWithPostInfo {
            let sender = ensure_signed(origin)?;
            ensure!(
                collateral >= T::DustCollateral::get(),
                Error::<T, I>::CollateralAmountTooSmall
            );
            ensure!(
                !Vaults::<T, I>::contains_key(&sender),
                Error::<T, I>::VaultAlreadyRegistered
            );
            Self::verify_address(&addr_str)?;

            ensure!(
                !OuterAddresses::<T, I>::contains_key(&addr_str),
                Error::<T, I>::BtcAddressOccupied
            );
            Self::inner_register_vault(&sender, addr_str, collateral)?;
            Self::deposit_event(Event::VaultRegistered(sender, collateral));
            Ok(().into())
        }

        /// Add extra collateral for registered vault.
        #[pallet::weight(<T as Config<I>>::WeightInfo::add_extra_collateral())]
        pub(crate) fn add_extra_collateral(
            origin: OriginFor<T>,
            collateral: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {
            let sender = ensure_signed(origin)?;
            ensure!(
                Vaults::<T, I>::contains_key(&sender),
                Error::<T, I>::VaultNotFound
            );
            Self::lock_collateral(&sender, collateral)?;
            Self::deposit_event(Event::ExtraCollateralAdded(sender, collateral));
            Ok(().into())
        }

        /// User request issue cross-chain asset.
        ///
        /// Sender should lock part of pcx, aka `griefing_fee`, which would be slashed to vault in
        /// case of malicious behavior and would be released while the request was executed.
        /// Sender also should pay service charge whether the request was executed or cancelled.
        /// All these are proportional to `amount`.
        /// `IssueRequest` couldn't be submitted while bridge during liquidating.
        #[pallet::weight(<T as Config<I>>::WeightInfo::request_issue())]
        pub fn request_issue(
            origin: OriginFor<T>,
            vault_id: T::AccountId,
            amount: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {
            let requester = ensure_signed(origin)?;

            Self::ensure_bridge_running()?;
            let collateral_ratio_later = Self::collateral_ratio_with_inc_amount(&vault_id, amount)?;
            ensure!(
                collateral_ratio_later >= T::SecureThreshold::get(),
                Error::<T, I>::InsecureVault
            );

            let griefing_collateral = Self::calculate_required_collateral(amount)?;
            let service_charge = Self::calculate_service_charge(amount)?;

            ensure!(
                griefing_collateral + service_charge < CurrencyOf::<T>::free_balance(&requester),
                Error::<T, I>::FreeBalanceTooLow
            );

            // locking griefing_fee
            CurrencyOf::<T>::reserve(&requester, griefing_collateral)?;
            // pay service charge to vault
            CurrencyOf::<T>::transfer(
                &requester,
                &vault_id,
                service_charge,
                ExistenceRequirement::KeepAlive,
            )?;

            let request_id =
                Self::insert_new_issue_request(requester, &vault_id, amount, griefing_collateral)?;
            // increase vault's `to_be_issued_tokens` to limit collateral ratio
            Self::increase_vault_to_be_issued_token(&vault_id, amount);
            Self::deposit_event(Event::<T, I>::NewIssueRequest(request_id));
            Ok(().into())
        }

        /// Execute issue request in `IssueRequests` which would be removed if `tx` valid.
        ///
        /// It verifies `tx` provided. The execute_issue can only called by signed origin.
        #[pallet::weight(<T as Config<I>>::WeightInfo::execute_issue())]
        pub fn execute_issue(
            origin: OriginFor<T>,
            request_id: RequestId,
            _block_hash: Vec<u8>,
            _merkle_proof: Vec<u8>,
            _raw_tx: Vec<u8>,
        ) -> DispatchResultWithPostInfo {
            ensure_signed(origin)?;
            Self::ensure_bridge_running()?;
            //TODO(wangyafei): verify tx

            let request = Self::try_get_issue_request(request_id)?;
            ensure!(
                Self::get_issue_request_duration(&request) < T::IssueRequestExpiredPeriod::get(),
                Error::<T, I>::IssueRequestExpired
            );
            // unlock user's `griefing_collateral` during `request_issue`
            CurrencyOf::<T>::unreserve(&request.requester, request.griefing_collateral);

            Self::mint(&request.requester, &request.vault, request.amount)?;
            IssueRequests::<T, I>::remove(&request_id);

            Self::deposit_event(Event::<T, I>::IssueRequestExecuted(request_id));
            Ok(().into())
        }

        /// Cancel an out-dated request and slash the griefing fee to vault.
        #[pallet::weight(<T as Config<I>>::WeightInfo::cancel_issue())]
        pub fn cancel_issue(
            origin: OriginFor<T>,
            request_id: RequestId,
        ) -> DispatchResultWithPostInfo {
            ensure_signed(origin)?;

            let request = Self::try_get_issue_request(request_id)?;

            ensure!(
                Self::get_issue_request_duration(&request) >= T::IssueRequestExpiredPeriod::get(),
                Error::<T, I>::IssueRequestNotExpired
            );

            // Punish griefing requester
            CurrencyOf::<T>::repatriate_reserved(
                &request.requester,
                &request.vault,
                request.griefing_collateral,
                BalanceStatus::Free,
            )?;

            Self::decrease_vault_to_be_issued_token(&request.vault, request.amount);
            IssueRequests::<T, I>::remove(&request_id);
            Self::deposit_event(Event::<T, I>::IssueRequestCancelled(request_id));
            Ok(().into())
        }

        /// Request to burn target asset in ChainX, e.g. XBTC, and get equivalent coins in outer chain, e.g. Bitcoin.
        #[pallet::weight(<T as Config<I>>::WeightInfo::request_redeem())]
        pub fn request_redeem(
            origin: OriginFor<T>,
            vault_id: T::AccountId,
            amount: BalanceOf<T>,
            outer_address: Vec<u8>,
        ) -> DispatchResultWithPostInfo {
            let sender = ensure_signed(origin)?;
            Self::ensure_bridge_running()?;
            // Only allow requests of amount above above the minimum
            ensure!(
                // this is the amount the vault will send (minus fee)
                amount >= T::MinimumRedeemValue::get(),
                Error::<T, I>::AmountBelowDustAmount
            );
            ensure!(
                amount <= Self::target_asset_of(&sender),
                Error::<T, I>::InsufficiantAssetsFunds
            );

            // Ensure this vault can work.
            let vault = Self::try_get_vault(&vault_id)?;
            ensure!(
                amount <= vault.issue_tokens,
                Error::<T, I>::RedeemAmountTooLarge
            );

            let service_charge = Self::calculate_service_charge(amount)?;
            ensure!(
                service_charge < CurrencyOf::<T>::free_balance(&sender),
                Error::<T, I>::FreeBalanceTooLow
            );

            Self::verify_address(&outer_address)?;
            // Lock redeemer's xtbc
            Self::lock_asset(&sender, amount)?;
            // Increase vault's to_be_redeemed_tokens
            Self::increase_vault_to_be_redeem_token(&vault_id, amount);

            // pay service charge to vault
            CurrencyOf::<T>::transfer(
                &sender,
                &vault_id,
                service_charge,
                ExistenceRequirement::KeepAlive,
            )?;

            let request_id =
                Self::insert_new_redeem_request(sender, &vault_id, amount, outer_address)?;
            Self::deposit_event(Event::<T, I>::NewRedeemRequest(request_id));
            Ok(().into())
        }

        #[pallet::weight(<T as Config<I>>::WeightInfo::execute_redeem())]
        pub fn execute_redeem(
            origin: OriginFor<T>,
            request_id: RequestId,
            _block_hash: Vec<u8>,
            _merkle_proof: Vec<u8>,
            _raw_tx: Vec<u8>,
        ) -> DispatchResultWithPostInfo {
            ensure_signed(origin)?;
            Self::ensure_bridge_running()?;
            let request = Self::try_get_redeem_request(request_id)?;
            ensure!(
                Self::get_redeem_request_duration(&request) < T::RedeemRequestExpiredPeriod::get(),
                Error::<T, I>::RedeemRequestExpired
            );
            Self::ensure_vault_exists(&request.vault)?;

            // TODO verify tx
            let current_collateral_ratio = Self::vault_collateral_ratio(&request.vault)?;
            if current_collateral_ratio < T::PremiumThreshold::get() {
                let premium_fee = Self::premium_fee();
                Self::slash_vault(&request.vault, &request.requester, premium_fee)?;
            }

            Self::burn(&request.requester, &request.vault, request.amount)?;

            RedeemRequests::<T, I>::remove(&request_id);

            Self::deposit_event(Event::<T, I>::RedeemExecuted(request_id));
            Ok(().into())
        }

        /// Cancel a `RedeemRequest` when it has been expired.
        ///
        /// Call the extrinsic while request ain't expired will cause `RedeemRequestNotExpired`
        /// error.
        #[pallet::weight(<T as Config<I>>::WeightInfo::cancel_redeem())]
        pub fn cancel_redeem(
            origin: OriginFor<T>,
            request_id: RequestId,
            reimburse: bool,
        ) -> DispatchResultWithPostInfo {
            ensure_signed(origin)?;
            let request = Self::try_get_redeem_request(request_id)?;
            ensure!(
                Self::get_redeem_request_duration(&request) >= T::RedeemRequestExpiredPeriod::get(),
                Error::<T, I>::RedeemRequestNotExpired
            );

            Self::ensure_vault_exists(&request.vault)?;

            if reimburse {
                // Decrease vault tokens
                let worth_pcx = Self::convert_to_pcx(request.amount)?;
                Self::slash_vault(&request.vault, &request.requester, worth_pcx)?;
            } else {
                Self::release_asset(&request.requester, request.amount)?;
            }
            Self::decrease_vault_to_be_redeem_token(&request.vault, request.amount);
            RedeemRequests::<T, I>::remove(&request_id);
            Self::deposit_event(Event::<T, I>::RedeemCancelled(request_id));
            Ok(().into())
        }

        /// Similar to [`update_exchange_rate`](crate::pallet::Pallet::update_exchange_rate),
        /// except it only allows root.
        #[pallet::weight(<T as Config<I>>::WeightInfo::force_update_exchange_rate())]
        pub(crate) fn force_update_exchange_rate(
            origin: OriginFor<T>,
            exchange_rate: TradingPrice,
        ) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;
            Self::inner_update_exchange_rate(exchange_rate.clone())?;
            Self::deposit_event(Event::<T, I>::ExchangeRateForceUpdated(exchange_rate));
            Ok(().into())
        }

        /// Force update oracles.
        ///
        /// DANGEROUS! The extrinsic will cover old oracles.
        #[pallet::weight(<T as Config<I>>::WeightInfo::force_update_oracles())]
        pub(crate) fn force_update_oracles(
            origin: OriginFor<T>,
            oracles: Vec<T::AccountId>,
        ) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;
            OracleAccounts::<T, I>::put(oracles.clone());
            Self::deposit_event(Event::<T, I>::OracleForceUpdated(oracles));
            Ok(().into())
        }

        /// Update griefing fee for requesting issue
        #[pallet::weight(<T as Config<I>>::WeightInfo::update_issue_griefing_fee())]
        pub fn update_issue_griefing_fee(
            origin: OriginFor<T>,
            griefing_fee: Percent,
        ) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;
            <IssueGriefingFee<T, I>>::put(griefing_fee);
            Self::deposit_event(Event::<T, I>::GriefingFeeUpdated(griefing_fee));
            Ok(().into())
        }
    }

    /// Events in xbridge module
    ///
    /// Emit when extrinsics or some important operators, like releasing/locking collateral,
    /// move/transfer balance, etc, have happened.
    #[pallet::event]
    #[pallet::generate_deposit(pub(crate) fn deposit_event)]
    #[pallet::metadata(T::AccountId = "AccountId", BalanceOf<T> = "Balance", BlockNumberFor<T> = "BlockNumber", Vec<T::AccountId>="Vec<AccountId>")]
    pub enum Event<T: Config<I>, I: 'static = ()> {
        /// Update exchange rate by oracle
        ExchangeRateUpdated(T::AccountId, TradingPrice),
        /// Update exchange rate by root
        ExchangeRateForceUpdated(TradingPrice),
        /// Update oracles by root
        OracleForceUpdated(Vec<T::AccountId>),
        /// Collateral was slashed. [from, to, amount]
        CollateralSlashed(T::AccountId, T::AccountId, BalanceOf<T>),
        /// The collateral was released to the user successfully. [who, amount]
        BridgeCollateralReleased(T::AccountId, BalanceOf<T>),
        /// Update `ExchangeRateExpiredPeriod`
        ExchangeRateExpiredPeriodForceUpdated(BlockNumberFor<T>),
        /// New vault has been registered.
        VaultRegistered(T::AccountId, BalanceOf<T>),
        /// Extra collateral was added to a vault.
        ExtraCollateralAdded(T::AccountId, BalanceOf<T>),
        /// Vault released collateral.
        CollateralReleased(T::AccountId, BalanceOf<T>),
        /// An issue request was submitted and waiting user to excute.
        NewIssueRequest(RequestId),
        /// `IssueRequest` excuted.
        IssueRequestExecuted(RequestId),
        /// `IssueRequest` cancelled.`
        IssueRequestCancelled(RequestId),
        /// Redeem request is accepted
        NewRedeemRequest(RequestId),
        /// Execute redeem is accepted
        RedeemExecuted(RequestId),
        /// Cancel redeem is accepted
        RedeemCancelled(RequestId),
        /// Root updated `IssueGriefingFee`.
        GriefingFeeUpdated(Percent),
    }

    /// Errors for assets module
    #[pallet::error]
    pub enum Error<T, I = ()> {
        /// Permission denied.
        NotOracle,
        /// Arithmetic underflow/overflow.
        ArithmeticError,
        /// Account doesn't have enough collateral to be slashed.
        InsufficientCollateral,
        /// Bridge was shutdown or in error.
        BridgeNotRunning,
        /// Try to calculate collateral ratio while has no issued_tokens
        NoIssuedTokens,
        /// The amount in request is less than lower bound.
        CollateralAmountTooSmall,
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
        /// No such `IssueRequest`
        IssueRequestNotFound,
        /// `IssueRequest` cancelled when it's not expired
        IssueRequestNotExpired,
        /// Tried to execute `IssueRequest` while  it's expired
        IssueRequestExpired,
        /// Vault colateral ratio was below than `SecureThreshold`
        InsecureVault,
        /// Redeem request id is not exsit
        RedeemRequestNotFound,
        /// Redeem request cancelled for forced redeem when it's not expired.
        RedeemRequestNotExpired,
        /// Redeem request is expierd
        RedeemRequestExpired,
        /// Vault is under Liquidation
        VaultLiquidated,
        /// Redeem amount is to low
        AmountBelowDustAmount,
        /// Redeem amount is not correct
        InsufficiantAssetsFunds,
        /// Account balance were not enough to be transfered or reserved.
        FreeBalanceTooLow,
        /// Vault issue token insufficient
        RedeemAmountTooLarge,
        /// Error propagated from xpallet_assets.
        AssetError,
    }

    /// Collateral for each vault.
    #[pallet::storage]
    #[pallet::getter(fn collaterals)]
    pub(crate) type Collaterals<T: Config<I>, I: 'static = ()> =
        StorageMap<_, Blake2_128Concat, T::AccountId, BalanceOf<T>, ValueQuery>;

    /// Exchange rate from pcx to btc.
    #[pallet::storage]
    #[pallet::getter(fn exchange_rate)]
    pub(crate) type ExchangeRate<T: Config<I>, I: 'static = ()> =
        StorageValue<_, TradingPrice, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn oracle_accounts)]
    pub(crate) type OracleAccounts<T: Config<I>, I: 'static = ()> =
        StorageValue<_, Vec<T::AccountId>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn bridge_status)]
    pub(crate) type BridgeStatus<T: Config<I>, I: 'static = ()> =
        StorageValue<_, Status, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn exchange_rate_update_time)]
    pub(crate) type ExchangeRateUpdateTime<T: Config<I>, I: 'static = ()> =
        StorageValue<_, BlockNumberFor<T>, ValueQuery>;

    /// Mapping account to vault struct.
    #[pallet::storage]
    pub(crate) type Vaults<T: Config<I>, I: 'static = ()> =
        StorageMap<_, Blake2_128Concat, T::AccountId, Vault<T::BlockNumber, BalanceOf<T>>>;

    /// Mapping out chain address to vault id.
    #[pallet::storage]
    pub(crate) type OuterAddresses<T: Config<I>, I: 'static = ()> =
        StorageMap<_, Twox64Concat, AddrStr, T::AccountId>;

    /// Percentage to lock, when user requests issue
    #[pallet::storage]
    #[pallet::getter(fn issue_griefing_fee)]
    pub(crate) type IssueGriefingFee<T: Config<I>, I: 'static = ()> =
        StorageValue<_, Percent, ValueQuery>;

    /// Auto-increament id to identify each issue request.
    /// Also presents total amount of created requests.
    #[pallet::storage]
    pub(crate) type IssueRequestCount<T: Config<I>, I: 'static = ()> =
        StorageValue<_, RequestId, ValueQuery>;

    /// Mapping from issue id to `IssueRequest`
    #[pallet::storage]
    pub(crate) type IssueRequests<T: Config<I>, I: 'static = ()> =
        StorageMap<_, Twox64Concat, RequestId, IssueRequest<T>>;

    /// Slashed when excuting redeem if vault's collateral is below than `PremiumThreshold`
    #[pallet::storage]
    #[pallet::getter(fn premium_fee)]
    pub(crate) type PremiumFee<T: Config<I>, I: 'static = ()> =
        StorageValue<_, BalanceOf<T>, ValueQuery>; /*TODO(wangyafei): use fixed currently*/

    /// Auto-increament id to identify each redeem request.
    /// Also presents total amount of created requests.
    #[pallet::storage]
    pub(crate) type RedeemRequestCount<T: Config<I>, I: 'static = ()> =
        StorageValue<_, RequestId, ValueQuery>;

    /// Mapping from redeem id to `RedeemRequest`
    #[pallet::storage]
    pub(crate) type RedeemRequests<T: Config<I>, I: 'static = ()> =
        StorageMap<_, Twox64Concat, RequestId, RedeemRequest<T>>;

    /// Radio in percentage that service charge to the issue/redeem amount.
    #[pallet::storage]
    #[pallet::getter(fn service_charge_ratio)]
    pub(crate) type ServiceChargeRatio<T: Config<I>, I: 'static = ()> =
        StorageValue<_, Percent, ValueQuery>;

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config<I>, I: 'static = ()> {
        /// Trading pair of pcx/btc.
        pub exchange_rate: TradingPrice,
        /// Accounts that allow to update exchange rate.
        pub oracle_accounts: Vec<T::AccountId>,
        /// SystemVault's account id.
        pub liquidator_id: T::AccountId,
        /// Fee that needs to be locked while user requests issuing xbtc, and will be released when
        /// the `IssueRequest` completed. It's proportional to `btc_amount` in `IssueRequest`.
        pub issue_griefing_fee: u8,
        /// Fee which is as the service charge while issue/redeem.
        pub service_charge_ratio: u8,
        pub marker: PhantomData<I>,
    }

    #[cfg(feature = "std")]
    impl<T: Config<I>, I: 'static> Default for GenesisConfig<T, I> {
        fn default() -> Self {
            Self {
                exchange_rate: Default::default(),
                oracle_accounts: Default::default(),
                liquidator_id: Default::default(),
                issue_griefing_fee: Default::default(),
                service_charge_ratio: 5u8,
                marker: PhantomData::<I>,
            }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config<I>, I: 'static> GenesisBuild<T, I> for GenesisConfig<T, I> {
        fn build(&self) {
            ExchangeRate::<T, I>::put(self.exchange_rate.clone());
            OracleAccounts::<T, I>::put(self.oracle_accounts.clone());
            IssueGriefingFee::<T, I>::put(Percent::from_parts(self.issue_griefing_fee));
            ServiceChargeRatio::<T, I>::put(Percent::from_parts(self.service_charge_ratio));
        }
    }

    impl<T: Config<I>, I: 'static> Pallet<T, I> {
        fn collateral_ratio_with_inc_amount(
            vault_id: &T::AccountId,
            btc_amount: BalanceOf<T>,
        ) -> Result<u16, DispatchError> {
            let vault = Self::try_get_vault(vault_id)?;
            // check if vault is rich enough
            let collateral_ratio_after_requesting = Self::calculate_collateral_ratio(
                vault.issue_tokens + vault.to_be_issued_tokens + btc_amount,
                Self::collateral_of(vault_id),
            )?;

            Ok(collateral_ratio_after_requesting)
        }

        #[inline]
        fn get_issue_request_duration(request: &IssueRequest<T>) -> BlockNumberFor<T> {
            let current_block = frame_system::Pallet::<T>::block_number();
            current_block - request.open_time
        }

        #[inline]
        fn get_redeem_request_duration(request: &RedeemRequest<T>) -> BlockNumberFor<T> {
            let current_block = frame_system::Pallet::<T>::block_number();
            current_block - request.open_time
        }
    }

    impl<T: Config<I>, I: 'static> Pallet<T, I> {
        pub fn convert_to_pcx(btc_amount: BalanceOf<T>) -> Result<BalanceOf<T>, DispatchError> {
            let exchange_rate = Self::exchange_rate();
            let result = exchange_rate
                .convert_to_pcx(btc_amount.saturated_into())
                .ok_or(Error::<T, I>::ArithmeticError)?;
            Ok(result.saturated_into())
        }

        pub fn convert_to_btc(pcx_amount: BalanceOf<T>) -> Result<BalanceOf<T>, DispatchError> {
            let exchange_rate = Self::exchange_rate();
            let result = exchange_rate
                .convert_to_btc(pcx_amount.saturated_into())
                .ok_or(Error::<T, I>::ArithmeticError)?;
            Ok(result.saturated_into())
        }

        fn verify_address(address: &[u8]) -> Result<MultiAddress, Error<T, I>> {
            from_utf8(address)
                .map_err(|_| Error::<T, I>::InvalidAddress)?
                .parse()
                .map_err(|_| Error::<T, I>::InvalidAddress)
        }
    }

    impl<T: Config<I>, I: 'static> Pallet<T, I> {
        pub fn vault_collateral_ratio(vault_id: &T::AccountId) -> Result<u16, DispatchError> {
            let collateral = Self::collateral_of(&vault_id);
            let token = Self::try_get_vault(&vault_id)
                .map_or_else(|_| 0u32.into(), |vault| vault.issue_tokens);
            Self::calculate_collateral_ratio(token, collateral)
        }
        pub fn calculate_collateral_ratio(
            issued_tokens: BalanceOf<T>,
            collateral: BalanceOf<T>,
        ) -> Result<u16, DispatchError> {
            ensure!(
                issued_tokens != 0u32.saturated_into(),
                Error::<T, I>::NoIssuedTokens
            );

            let exchange_rate: TradingPrice = Self::exchange_rate();
            let equivalence_collateral = exchange_rate
                .convert_to_pcx(issued_tokens.saturated_into())
                .ok_or(Error::<T, I>::ArithmeticError)?;
            let raw_collateral: u128 = collateral.saturated_into();
            let collateral_ratio = raw_collateral
                .saturating_mul(100)
                .checked_div(equivalence_collateral)
                .ok_or(Error::<T, I>::ArithmeticError)?;
            Ok(collateral_ratio as u16)
        }

        /// Get `IssueRequest` from id
        pub(crate) fn try_get_issue_request(
            request_id: RequestId,
        ) -> Result<IssueRequest<T>, DispatchError> {
            IssueRequests::<T, I>::get(request_id)
                .ok_or_else(|| Error::<T, I>::IssueRequestNotFound.into())
        }

        /// Get `IssueRequest` from id
        pub(crate) fn try_get_redeem_request(
            request_id: RequestId,
        ) -> Result<RedeemRequest<T>, DispatchError> {
            RedeemRequests::<T, I>::get(request_id)
                .ok_or_else(|| Error::<T, I>::RedeemRequestNotFound.into())
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

        /// Calculate service charge would be paid to vault.
        ///
        /// `amount` is the amount of target asset, e.g. bitcoin or dogecoin,
        /// and the result is in native asset, aka pcx.
        pub(crate) fn calculate_service_charge(
            amount: BalanceOf<T>,
        ) -> Result<BalanceOf<T>, DispatchError> {
            let pcx_amount = Self::convert_to_pcx(amount)?;
            let percentage = Self::service_charge_ratio();
            let service_charge = percentage.mul_ceil(pcx_amount);
            Ok(service_charge)
        }

        /// generate secure key from account id
        pub(crate) fn get_next_issue_id() -> RequestId {
            <IssueRequestCount<T, I>>::mutate(|n| {
                *n += 1;
                *n
            })
        }

        /// Generate secure key from account id
        pub(crate) fn get_next_redeem_id() -> RequestId {
            <RedeemRequestCount<T, I>>::mutate(|n| {
                *n += 1;
                *n
            })
        }

        fn inner_update_exchange_rate(exchange_rate: TradingPrice) -> DispatchResult {
            ensure!(
                exchange_rate.price > 0 && exchange_rate.decimal > 0,
                Error::<T, I>::ArithmeticError
            );
            <ExchangeRate<T, I>>::put(exchange_rate);
            let height = <frame_system::Pallet<T>>::block_number();
            <ExchangeRateUpdateTime<T, I>>::put(height);
            Self::recover_from_exchange_rate_expired();
            Ok(())
        }
    }

    // Getter and Checker
    impl<T: Config<I>, I: 'static> Pallet<T, I> {
        #[inline]
        pub(crate) fn ensure_bridge_running() -> DispatchResult {
            ensure!(
                Self::bridge_status() == Status::Running,
                Error::<T, I>::BridgeNotRunning
            );
            Ok(())
        }

        #[inline]
        pub(crate) fn ensure_vault_exists(id: &T::AccountId) -> DispatchResult {
            Self::try_get_vault(id)?;
            Ok(())
        }

        pub fn try_get_vault(id: &T::AccountId) -> Result<DefaultVault<T>, DispatchError> {
            Vaults::<T, I>::get(id).ok_or_else(|| Error::<T, I>::VaultNotFound.into())
        }
    }

    impl<T: Config<I>, I: 'static> Pallet<T, I> {
        /// Clarify `ExchangeRateExpired` is solved and recover from this error.
        ///
        /// Dangerous! Ensure this error truly solved is caller's responsibility.
        pub(crate) fn recover_from_exchange_rate_expired() {
            if let Status::Error(mut error_codes) = Self::bridge_status() {
                if error_codes.contains(ErrorCode::EXCHANGE_RATE_EXPIRED) {
                    error_codes.remove(ErrorCode::EXCHANGE_RATE_EXPIRED);
                    <BridgeStatus<T, I>>::put(Status::Error(error_codes))
                }
            }
        }
    }

    // Vault related stuff.
    impl<T: Config<I>, I: 'static> Pallet<T, I> {
        pub(crate) fn inner_register_vault(
            who: &T::AccountId,
            address: AddrStr,
            collateral: BalanceOf<T>,
        ) -> DispatchResult {
            Self::lock_collateral(&who, collateral)?;
            OuterAddresses::<T, I>::insert(&address, who.clone());
            Vaults::<T, I>::insert(&who, Vault::new(address));
            Ok(())
        }

        #[inline]
        pub(crate) fn process_vault_issue(vault_id: &T::AccountId, amount: BalanceOf<T>) {
            Vaults::<T, I>::mutate(vault_id, |vault| {
                if let Some(vault) = vault {
                    vault.to_be_issued_tokens -= amount;
                    vault.issue_tokens += amount;
                }
            })
        }

        #[inline]
        pub(crate) fn process_vault_redeem(vault_id: &T::AccountId, amount: BalanceOf<T>) {
            Vaults::<T, I>::mutate(vault_id, |vault| {
                if let Some(vault) = vault {
                    vault.to_be_redeemed_tokens -= amount;
                    vault.issue_tokens -= amount;
                }
            })
        }

        #[inline]
        pub(crate) fn increase_vault_to_be_issued_token(
            vault_id: &T::AccountId,
            amount: BalanceOf<T>,
        ) {
            Vaults::<T, I>::mutate(vault_id, |vault| {
                if let Some(vault) = vault {
                    vault.to_be_issued_tokens += amount;
                }
            });
        }

        #[inline]
        pub(crate) fn decrease_vault_to_be_issued_token(
            vault_id: &T::AccountId,
            amount: BalanceOf<T>,
        ) {
            Vaults::<T, I>::mutate(vault_id, |vault| {
                if let Some(vault) = vault {
                    vault.to_be_issued_tokens -= amount;
                }
            });
        }

        #[inline]
        pub(crate) fn increase_vault_to_be_redeem_token(
            vault_id: &T::AccountId,
            amount: BalanceOf<T>,
        ) {
            Vaults::<T, I>::mutate(&vault_id, |vault| {
                //vault exists; qed.
                if let Some(vault) = vault {
                    vault.to_be_redeemed_tokens += amount
                }
            });
        }

        #[inline]
        pub(crate) fn decrease_vault_to_be_redeem_token(
            vault_id: &T::AccountId,
            amount: BalanceOf<T>,
        ) {
            Vaults::<T, I>::mutate(&vault_id, |vault| {
                //vault exists; qed.
                if let Some(vault) = vault {
                    vault.to_be_redeemed_tokens -= amount
                }
            });
        }
    }
    impl<T: Config<I>, I: 'static> Pallet<T, I> {
        pub(crate) fn insert_new_issue_request(
            requester: T::AccountId,
            vault_id: &T::AccountId,
            amount: BalanceOf<T>,
            griefing_collateral: BalanceOf<T>,
        ) -> Result<RequestId, DispatchError> {
            let request_id = Self::get_next_issue_id();
            let vault = Self::try_get_vault(vault_id)?;
            IssueRequests::<T, I>::insert(
                request_id,
                IssueRequest::<T> {
                    vault: vault_id.clone(),
                    open_time: <frame_system::Pallet<T>>::block_number(),
                    requester,
                    outer_address: vault.wallet,
                    amount,
                    griefing_collateral,
                },
            );
            Ok(request_id)
        }

        pub(crate) fn insert_new_redeem_request(
            requester: T::AccountId,
            vault_id: &T::AccountId,
            amount: BalanceOf<T>,
            outer_address: AddrStr,
        ) -> Result<RequestId, DispatchError> {
            // Generate redeem request identify and insert it to record
            let request_id = Self::get_next_redeem_id();
            RedeemRequests::<T, I>::insert(
                request_id,
                RedeemRequest::<T> {
                    vault: vault_id.clone(),
                    open_time: <frame_system::Pallet<T>>::block_number(),
                    requester,
                    outer_address,
                    amount,
                    reimburse: false,
                },
            );
            Ok(request_id)
        }
    }
}
