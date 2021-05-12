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

mod collateral;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub mod types;

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
    use light_bitcoin::keys::MultiAddress;

    use chainx_primitives::AssetId;

    use crate::types::*;

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
        BalanceOf<T>,
    >;

    pub(crate) type RequestId = u128;

    pub(crate) type RedeemRequest<T> = crate::types::RedeemRequest<
        <T as frame_system::Config>::AccountId,
        <T as frame_system::Config>::BlockNumber,
        BalanceOf<T>,
        BalanceOf<T>,
    >;

    #[pallet::pallet]
    #[pallet::generate_store(pub(crate) trait Store)]
    pub struct Pallet<T, I = ()>(PhantomData<(T, I)>);

    #[pallet::config]
    pub trait Config<I: 'static = ()>: frame_system::Config + xpallet_assets::Config {
        type Event: From<Event<Self, I>> + IsType<<Self as frame_system::Config>::Event>;

        /// Target asset id in this chainx bridge instance.
        ///
        /// Each outer bitcoin-like chain has a corresponding instance. The field records the
        /// `AssdtId` of that chain.
        #[pallet::constant]
        type TargetAssetId: Get<AssetId>;
        /// Shadow asset for target asset.
        ///
        /// Shadow asset is a read-only asset. It only indicates how many issuance was approved by owner.
        #[pallet::constant]
        type TokenAssetId: Get<AssetId>;
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
        type IssueRequestExpiredTime: Get<BlockNumberFor<Self>>;
        /// Duration from `RedeemRequest` opened to expired.
        type RedeemRequestExpiredTime: Get<BlockNumberFor<Self>>;
        /// Duration from `ExchangeRate` last updated to expired.
        #[pallet::constant]
        type ExchangeRateExpiredPeriod: Get<BlockNumberFor<Self>>;
        /// The minimum amount of btc that is accepted for redeem requests; any lower values would
        /// risk the bitcoin client to reject the payment
        #[pallet::constant]
        type MinimumRedeemValue: Get<BalanceOf<Self>>;
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
        #[pallet::weight(0)]
        pub(crate) fn update_exchange_rate(
            origin: OriginFor<T>,
            exchange_rate: TradingPrice,
        ) -> DispatchResultWithPostInfo {
            let sender = ensure_signed(origin)?;
            ensure!(
                Self::oracle_accounts().contains(&sender),
                Error::<T, I>::NotOracle
            );
            Self::_update_exchange_rate(exchange_rate.clone())?;
            Self::deposit_event(Event::<T, I>::ExchangeRateUpdated(sender, exchange_rate));
            Ok(().into())
        }

        /// Register a vault with collateral and unique `btc_address`.
        ///
        /// The extrinsic's origin must be signed.
        /// *Relative Functions*:
        /// [`add_extra_collateral`](crate::Pallet::add_extra_collateral)
        #[pallet::weight(0)]
        pub(crate) fn register_vault(
            origin: OriginFor<T>,
            collateral: BalanceOf<T>,
            btc_address: Vec<u8>,
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
            Self::verify_address(&btc_address)?;

            ensure!(
                !BtcAddresses::<T, I>::contains_key(&btc_address),
                Error::<T, I>::BtcAddressOccupied
            );
            Self::lock_collateral(&sender, collateral)?;
            BtcAddresses::<T, I>::insert(&btc_address, sender.clone());
            Vaults::<T, I>::insert(&sender, Vault::new(btc_address));
            Self::deposit_event(Event::VaultRegistered(sender, collateral));
            Ok(().into())
        }

        /// Add extra collateral for registered vault.
        #[pallet::weight(0)]
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

        /// User request issue xbtc
        ///
        /// `IssueRequest` couldn't be submitted while bridge during liquidating.
        #[pallet::weight(0)]
        pub fn request_issue(
            origin: OriginFor<T>,
            vault_id: T::AccountId,
            btc_amount: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {
            let sender = ensure_signed(origin)?;

            Self::ensure_bridge_running()?;
            let collateral_ratio_later =
                Self::collateral_ratio_with_inc_amount(&vault_id, btc_amount)?;
            ensure!(
                collateral_ratio_later >= T::SecureThreshold::get(),
                Error::<T, I>::InsecureVault
            );

            let griefing_collateral = Self::calculate_required_collateral(btc_amount)?;
            CurrencyOf::<T>::reserve(&sender, griefing_collateral)?;

            let request_id = Self::get_next_issue_id();
            let vault = Self::try_get_active_vault(&vault_id)?;
            IssueRequests::<T, I>::insert(
                request_id,
                IssueRequest::<T> {
                    vault: vault_id.clone(),
                    open_time: <frame_system::Pallet<T>>::block_number(),
                    requester: sender,
                    btc_address: vault.wallet,
                    btc_amount,
                    griefing_collateral,
                },
            );
            Vaults::<T, I>::mutate(&vault_id, |vault| {
                if let Some(vault) = vault {
                    vault.to_be_issued_tokens += btc_amount;
                }
            });
            Self::deposit_event(Event::<T, I>::NewIssueRequest(request_id));
            Ok(().into())
        }

        /// Execute issue request in `IssueRequests`. It verifies `tx` provided and marks
        /// `IssueRequest` as completed.
        ///
        /// The execute_issue can only called by signed origin.
        #[pallet::weight(0)]
        pub fn execute_issue(
            origin: OriginFor<T>,
            request_id: RequestId,
            _tx_id: Vec<u8>,
            _merkle_proof: Vec<u8>,
            _raw_tx: Vec<u8>,
        ) -> DispatchResultWithPostInfo {
            ensure_signed(origin)?;

            Self::ensure_bridge_running()?;
            //TODO(wangyafei): verify tx

            let request = Self::try_get_issue_request(request_id)?;
            ensure!(
                Self::get_issue_request_duration(&request) < T::IssueRequestExpiredTime::get(),
                Error::<T, I>::IssueRequestExpired
            );

            CurrencyOf::<T>::unreserve(&request.requester, request.griefing_collateral);
            Vaults::<T, I>::mutate(&request.vault, |vault| {
                if let Some(vault) = vault {
                    vault.to_be_issued_tokens -= request.btc_amount;
                }
            });
            Self::mint(&request.requester, &request.vault, request.btc_amount)?;

            IssueRequests::<T, I>::remove(&request_id);

            Self::deposit_event(Event::<T, I>::IssueRequestExecuted(request_id));
            Ok(().into())
        }

        #[pallet::weight(0)]
        pub fn cancel_issue(
            origin: OriginFor<T>,
            request_id: RequestId,
        ) -> DispatchResultWithPostInfo {
            ensure_signed(origin)?;

            let request = Self::try_get_issue_request(request_id)?;

            ensure!(
                Self::get_issue_request_duration(&request) >= T::IssueRequestExpiredTime::get(),
                Error::<T, I>::IssueRequestNotExpired
            );

            let slashed_collateral = Self::calculate_slashed_collateral(request.btc_amount)?;
            Self::slash_vault(&request.vault, &request.requester, slashed_collateral)?;

            // It's allowed to reture a non-zero value while that's unlikely happened.
            CurrencyOf::<T>::unreserve(&request.requester, request.griefing_collateral);

            Vaults::<T, I>::mutate(&request.vault, |vault| {
                if let Some(vault) = vault {
                    vault.to_be_issued_tokens -= request.btc_amount;
                }
            });
            IssueRequests::<T, I>::remove(&request_id);

            Self::deposit_event(Event::<T, I>::IssueRequestCancelled(request_id));
            Ok(().into())
        }

        /// User request redeem
        #[pallet::weight(0)]
        pub fn request_redeem(
            origin: OriginFor<T>,
            vault_id: T::AccountId,
            redeem_amount: BalanceOf<T>,
            btc_address: Vec<u8>,
        ) -> DispatchResultWithPostInfo {
            let sender = ensure_signed(origin)?;
            Self::ensure_bridge_running()?;
            // Only allow requests of amount above above the minimum
            ensure!(
                // this is the amount the vault will send (minus fee)
                redeem_amount >= T::MinimumRedeemValue::get(),
                Error::<T, I>::AmountBelowDustAmount
            );
            ensure!(
                redeem_amount <= Self::target_asset_of(&sender),
                Error::<T, I>::InsufficiantAssetsFunds
            );

            // Ensure this vault can work.
            Self::try_get_active_vault(&vault_id)?;
            ensure!(
                redeem_amount <= Self::token_asset_of(&vault_id),
                Error::<T, I>::RedeemAmountTooLarge
            );

            Self::verify_address(&btc_address)?;

            // Increase vault's to_be_redeemed_tokens
            Vaults::<T, I>::mutate(&vault_id, |vault| {
                //vault exists; qed.
                if let Some(vault) = vault {
                    vault.to_be_redeemed_tokens += redeem_amount
                }
            });

            // Lock redeem's xtbc
            Self::lock_asset(&sender, redeem_amount)?;

            // Generate redeem request identify and insert it to record
            let request_id = Self::get_next_redeem_id();
            RedeemRequests::<T, I>::insert(
                request_id,
                RedeemRequest::<T> {
                    vault: vault_id,
                    open_time: <frame_system::Pallet<T>>::block_number(),
                    requester: sender,
                    btc_address,
                    btc_amount: redeem_amount,
                    redeem_fee: RedeemFee::<T, I>::get(),
                    reimburse: false,
                },
            );

            Self::deposit_event(Event::<T, I>::NewRedeemRequest(request_id));
            Ok(().into())
        }

        #[pallet::weight(0)]
        pub fn execute_redeem(
            origin: OriginFor<T>,
            request_id: RequestId,
            _tx_id: Vec<u8>,
            _merkle_proof: Vec<u8>,
            _raw_tx: Vec<u8>,
        ) -> DispatchResultWithPostInfo {
            ensure_signed(origin)?;

            Self::ensure_bridge_running()?;

            let request = <RedeemRequests<T, I>>::get(request_id)
                .ok_or(Error::<T, I>::RedeemRequestNotFound)?;

            ensure!(
                Self::get_redeem_request_duration(&request) < T::RedeemRequestExpiredTime::get(),
                Error::<T, I>::RedeemRequestExpired
            );
            Self::try_get_active_vault(&request.vault)?;

            // TODO verify tx
            let collateral = Self::collateral_of(&request.vault);
            let current_collateral_ratio =
                Self::calculate_collateral_ratio(Self::token_asset_of(&request.vault), collateral)?;

            if current_collateral_ratio < T::PremiumThreshold::get() {
                let premium_fee = Self::premium_fee();
                Self::slash_vault(&request.vault, &request.requester, premium_fee)?;
            }

            Vaults::<T, I>::mutate(&request.vault, |vault| {
                if let Some(vault) = vault {
                    vault.to_be_redeemed_tokens -= request.btc_amount;
                }
            });
            Self::burn(&request.requester, &request.vault, request.btc_amount)?;

            RedeemRequests::<T, I>::remove(&request_id);

            Self::deposit_event(Event::<T, I>::RedeemExecuted(request_id));
            Ok(().into())
        }

        /// Cancel a `RedeemRequest` when it has been expired.
        ///
        /// Call the extrinsic while request ain't expired will cause `RedeemRequestNotExpired`
        /// error.
        #[pallet::weight(0)]
        pub fn cancel_redeem(
            origin: OriginFor<T>,
            request_id: RequestId,
            reimburse: bool,
        ) -> DispatchResultWithPostInfo {
            let sender = ensure_signed(origin)?;

            let request = <RedeemRequests<T, I>>::get(request_id)
                .ok_or(Error::<T, I>::RedeemRequestNotFound)?;

            ensure!(request.requester == sender, Error::<T, I>::InvalidRequester);
            ensure!(
                Self::get_redeem_request_duration(&request) >= T::RedeemRequestExpiredTime::get(),
                Error::<T, I>::RedeemRequestNotExpired
            );

            Self::try_get_active_vault(&request.vault)?;
            let worth_pcx = Self::convert_to_pcx(request.btc_amount)?;

            if reimburse {
                // Decrease vault tokens
                Vaults::<T, I>::mutate(&request.vault, |vault| {
                    if let Some(vault) = vault {
                        vault.to_be_redeemed_tokens -= request.btc_amount;
                    }
                });

                // Punish vault fee
                let punishment_fee: BalanceOf<T> = 0u32.into();

                // Vault give pcx to sender
                Self::slash_vault(
                    &request.vault,
                    &request.requester,
                    worth_pcx + punishment_fee,
                )?;
            } else {
                Self::release_asset(&request.requester, request.btc_amount)?;
            }

            RedeemRequests::<T, I>::remove(&request_id);

            Self::deposit_event(Event::<T, I>::RedeemCancelled(request_id));
            Ok(().into())
        }

        /// Similar to [`update_exchange_rate`](crate::pallet::Pallet::update_exchange_rate),
        /// except it only allows root.
        #[pallet::weight(0)]
        pub(crate) fn force_update_exchange_rate(
            origin: OriginFor<T>,
            exchange_rate: TradingPrice,
        ) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;
            Self::_update_exchange_rate(exchange_rate.clone())?;
            Self::deposit_event(Event::<T, I>::ExchangeRateForceUpdated(exchange_rate));
            Ok(().into())
        }

        /// Force update oracles.
        ///
        /// DANGEROUS! The extrinsic will cover old oracles.
        #[pallet::weight(0)]
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
        #[pallet::weight(0)]
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
        /// `IssueRequest` or `RedeemRequest` has been executed or cancelled
        RequestDealt,
        /// Redeem request id is not exsit
        RedeemRequestNotFound,
        /// Redeem request cancelled for forced redeem when it's not expired.
        RedeemRequestNotExpired,
        /// Redeem request is expierd
        RedeemRequestExpired,
        /// Vault is under Liquidation
        VaultLiquidated,
        /// Actioner is not the request's owner
        InvalidRequester,
        /// Redeem amount is to low
        AmountBelowDustAmount,
        /// Redeem amount is not correct
        InsufficiantAssetsFunds,
        /// Redeem in Processing
        RedeemRequestProcessing,
        /// Redeem is completed
        RedeemRequestAlreadyCompleted,
        /// Redeem is cancled
        RedeemRequestAlreadyCancled,
        /// Bridge status is not correct
        BridgeStatusError,
        /// Invalid btc address
        InvalidBtcAddress,
        /// Vault issue token insufficient
        RedeemAmountTooLarge,
        /// Error propagated from xpallet_assets.
        AssetError,
    }

    /// Total collateral locked by xbridge.
    #[pallet::storage]
    #[pallet::getter(fn total_collateral)]
    pub(crate) type TotalCollateral<T: Config<I>, I: 'static = ()> =
        StorageValue<_, BalanceOf<T>, ValueQuery>;

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

    /// Mapping btc address to vault id.
    #[pallet::storage]
    pub(crate) type BtcAddresses<T: Config<I>, I: 'static = ()> =
        StorageMap<_, Twox64Concat, BtcAddress, T::AccountId>;

    /// Specicial `SystemVault`
    #[pallet::storage]
    #[pallet::getter(fn liquidator)]
    pub(crate) type Liquidator<T: Config<I>, I: 'static = ()> =
        StorageValue<_, SystemVault<T::AccountId, BalanceOf<T>>, ValueQuery>;

    /// Liquidator account id
    #[pallet::storage]
    #[pallet::getter(fn liquidator_id)]
    pub(crate) type LiquidatorId<T: Config<I>, I: 'static = ()> =
        StorageValue<_, T::AccountId, ValueQuery>;

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

    /// Redeem fee when use request redeem
    #[pallet::storage]
    #[pallet::getter(fn redeem_fee)]
    pub(crate) type RedeemFee<T: Config<I>, I: 'static = ()> =
        StorageValue<_, BalanceOf<T>, ValueQuery>;

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
        /// Fixed fee that user shall lock when requesting redeem.
        pub redeem_fee: BalanceOf<T>,
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
                redeem_fee: Default::default(),
                marker: PhantomData::<I>,
            }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config<I>, I: 'static> GenesisBuild<T, I> for GenesisConfig<T, I> {
        fn build(&self) {
            <ExchangeRate<T, I>>::put(self.exchange_rate.clone());
            <OracleAccounts<T, I>>::put(self.oracle_accounts.clone());
            <Liquidator<T, I>>::put(SystemVault {
                id: self.liquidator_id.clone(),
                ..Default::default()
            });
            <LiquidatorId<T, I>>::put(self.liquidator_id.clone());
            <IssueGriefingFee<T, I>>::put(Percent::from_parts(self.issue_griefing_fee));
            <RedeemFee<T, I>>::put(self.redeem_fee);
        }
    }

    impl<T: Config<I>, I: 'static> Pallet<T, I> {
        fn collateral_ratio_with_inc_amount(
            vault_id: &T::AccountId,
            btc_amount: BalanceOf<T>,
        ) -> Result<u16, DispatchError> {
            let vault = Self::try_get_active_vault(vault_id)?;
            // check if vault is rich enough
            let collateral_ratio_after_requesting = Self::calculate_collateral_ratio(
                Self::token_asset_of(vault_id) + vault.to_be_issued_tokens + btc_amount,
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
            <IssueRequests<T, I>>::get(request_id)
                .ok_or_else(|| Error::<T, I>::IssueRequestNotFound.into())
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

        /// Calculate slashed amount.
        ///
        /// Equals the corresponding pcx times secure threshold
        pub(crate) fn calculate_slashed_collateral(
            btc_amount: BalanceOf<T>,
        ) -> Result<BalanceOf<T>, DispatchError> {
            let pcx_amount = Self::convert_to_pcx(btc_amount)?;
            let secure_threshold = T::SecureThreshold::get();
            let slashed_collateral: u32 =
                (pcx_amount.saturated_into::<u128>() * secure_threshold as u128 / 100) as u32;
            Ok(slashed_collateral.into())
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

        fn _update_exchange_rate(exchange_rate: TradingPrice) -> DispatchResult {
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

        fn _calculate_vault_token_upper_bound(
            vault_id: &T::AccountId,
        ) -> Result<BalanceOf<T>, DispatchError> {
            let vault_collateral = Self::collateral_of(vault_id);
            let secure_collateral = 100u128
                .checked_mul(vault_collateral.saturated_into())
                .and_then(|c| c.checked_div(u128::from(T::SecureThreshold::get())))
                .ok_or(Error::<T, I>::ArithmeticError)?;
            Self::convert_to_btc(secure_collateral.saturated_into())
        }

        //rpc use
        pub fn get_first_matched_vault(
            xbtc_amount: BalanceOf<T>,
        ) -> Option<(T::AccountId, Vec<u8>)> {
            Vaults::<T, I>::iter()
                .filter(|(vault_id, vault)| {
                    if let Ok(token_upper_bound) =
                        Self::_calculate_vault_token_upper_bound(vault_id)
                    {
                        token_upper_bound
                            > xbtc_amount
                                + vault.to_be_issued_tokens
                                + Self::token_asset_of(vault_id)
                    } else {
                        false
                    }
                })
                .take(1)
                .map(|(vault_id, vault)| (vault_id, vault.wallet))
                .next()
        }
    }

    // Getter and Checker
    impl<T: Config<I>, I: 'static> Pallet<T, I> {
        /// Get if the bridge running
        pub fn is_bridge_running() -> bool {
            Self::bridge_status() == Status::Running
        }

        #[inline]
        pub(crate) fn ensure_bridge_running() -> DispatchResult {
            ensure!(
                Self::bridge_status() == Status::Running,
                Error::<T, I>::BridgeNotRunning
            );
            Ok(())
        }

        pub fn try_get_vault(
            id: &T::AccountId,
        ) -> Result<Vault<T::BlockNumber, BalanceOf<T>>, DispatchError> {
            match <Vaults<T, I>>::get(id) {
                Some(vault) => Ok(vault),
                None => Err(Error::<T, I>::VaultNotFound.into()),
            }
        }

        pub fn try_get_active_vault(
            id: &T::AccountId,
        ) -> Result<Vault<T::BlockNumber, BalanceOf<T>>, DispatchError> {
            let vault = Self::try_get_vault(id)?;
            if vault.status == VaultStatus::Active {
                Ok(vault)
            } else {
                Err(Error::<T, I>::VaultInactive.into())
            }
        }

        /// Check `id` is vault or system_vault.
        pub fn check_is_vault(id: &T::AccountId) -> bool {
            if id == &Self::liquidator_id() {
                true
            } else {
                Vaults::<T, I>::contains_key(id)
            }
        }

        #[inline]
        pub fn change_vault_status(vault_id: &T::AccountId, status: VaultStatus) -> DispatchResult {
            // Change vault type
            <Vaults<T, I>>::mutate(vault_id, |vault| match vault {
                Some(ref mut vault) => {
                    vault.status = status;
                    Ok(())
                }
                None => Err(Error::<T, I>::VaultNotFound.into()),
            })
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

        /// Clarify `Liquidating` is solved and recover from this error.
        ///
        /// Dangerous! Ensure this error truly solved is caller's responsibility.
        pub(crate) fn recover_from_liquidating() {
            if let Status::Error(mut error_codes) = Self::bridge_status() {
                if error_codes.contains(ErrorCode::LIQUIDATING) {
                    error_codes.remove(ErrorCode::LIQUIDATING);
                    <BridgeStatus<T, I>>::put(Status::Error(error_codes))
                }
            }
        }
    }
}
