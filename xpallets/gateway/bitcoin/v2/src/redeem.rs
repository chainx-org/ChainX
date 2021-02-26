#![cfg_attr(not(feature = "std"), no_std)]

pub mod types {
    use codec::{Decode, Encode};
    use light_bitcoin::keys::Address;

    pub type BtcAddress = Address;

    #[derive(Encode, Decode, Clone, PartialEq)]
    #[cfg_attr(feature = "std", derive(Debug))]
    pub enum RedeemRequestStatus {
        /// Redeem is accepted and vault will transfer btc
        Processing,
        /// Redeem is cancelled by redeemer
        Cancelled,
        /// Redeem is compeleted
        Completed,
    }

    // Default value
    impl Default for RedeemRequestStatus {
        fn default() -> Self {
            RedeemRequestStatus::Processing
        }
    }

    #[derive(Encode, Decode, Default, Clone, PartialEq)]
    #[cfg_attr(feature = "std", derive(Debug))]
    pub struct RedeemRequest<AccountId, BlockNumber, XBTC, PCX> {
        /// Vault id
        pub(crate) vault: AccountId,
        /// Block height when the redeem requested
        pub(crate) open_time: BlockNumber,
        /// Who requests redeem
        pub(crate) requester: AccountId,
        /// Vault's btc address
        pub(crate) btc_address: BtcAddress,
        /// Amount that user wants to redeem
        pub(crate) amount: XBTC,
        /// Redeem fee amount
        pub(crate) redeem_fee: PCX,
        /// Request status
        pub(crate) status: RedeemRequestStatus,
        /// If redeem is reimbursed by redeemer
        pub(crate) reimburse: bool,
    }
}

#[frame_support::pallet]
#[allow(dead_code)]
pub mod pallet {
    use sp_std::{marker::PhantomData, str::from_utf8, vec::Vec};

    use frame_support::{
        dispatch::DispatchResultWithPostInfo,
        ensure,
        storage::types::{StorageMap, StorageValue, ValueQuery},
        traits::{Hooks, IsType},
        Twox64Concat,
    };
    use frame_system::{
        ensure_root, ensure_signed,
        pallet_prelude::{BlockNumberFor, OriginFor},
    };

    use chainx_primitives::AssetId;
    use xpallet_assets::AssetType;

    // Import vault,issue,assets code.
    use super::types::RedeemRequestStatus;
    use crate::assets::{pallet as assets, pallet::BalanceOf};
    use crate::issue::pallet as issue;
    use crate::vault::pallet as vault;

    type RedeemRequest<T> = super::types::RedeemRequest<
        <T as frame_system::Config>::AccountId,
        <T as frame_system::Config>::BlockNumber,
        BalanceOf<T>,
        BalanceOf<T>,
    >;
    type RequestId = u128;
    type AddrStr = Vec<u8>;

    const ASSET_ID: AssetId = 1;

    #[pallet::pallet]
    #[pallet::generate_store(pub(crate) trait Store)]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::config]
    pub trait Config: frame_system::Config + issue::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// User request redeem
        #[pallet::weight(0)]
        pub fn request_redeem(
            origin: OriginFor<T>,
            vault_id: T::AccountId,
            redeem_amount: BalanceOf<T>,
            btc_addr: AddrStr,
        ) -> DispatchResultWithPostInfo {
            Self::ensure_chain_correct_status()?;

            // Verify redeemer asset
            let sender = ensure_signed(origin)?;
            let btc_addr = from_utf8(&btc_addr)
                .map_err(|_| Error::<T>::InvalidBtcAddress)?
                .parse()
                .map_err(|_| Error::<T>::InvalidBtcAddress)?;
            let redeemer_balance = Self::asset_balance_of(&sender);
            ensure!(
                redeem_amount <= redeemer_balance,
                Error::<T>::InsufficiantAssetsFunds
            );

            // Ensure this vault can work.
            let height = <frame_system::Pallet<T>>::block_number();
            let vault = vault::Pallet::<T>::get_active_vault_by_id(&vault_id)?;
            ensure!(
                redeem_amount <= vault.issued_tokens,
                Error::<T>::VaultTokenInsufficiant
            );

            // Only allow requests of amount above above the minimum
            let dust_value = <RedeemBtcDustValue<T>>::get();
            ensure!(
                // this is the amount the vault will send (minus fee)
                redeem_amount >= dust_value,
                Error::<T>::AmountBelowDustAmount
            );

            // Increase vault's to_be_redeemed_tokens
            <vault::Vaults<T>>::mutate(&vault.id, |vault| {
                if let Some(vault) = vault {
                    vault.to_be_redeemed_tokens += redeem_amount;
                }
            });

            // Lock redeem's xtbc
            Self::lock_xbtc(&sender, redeem_amount)?;

            // Generate redeem request identify and insert it to record
            let request_id = Self::get_next_request_id();
            <RedeemRequests<T>>::insert(
                request_id,
                RedeemRequest::<T> {
                    vault: vault_id,
                    open_time: height,
                    requester: sender,
                    btc_address: btc_addr,
                    amount: redeem_amount,
                    redeem_fee: Default::default(),
                    status: Default::default(),
                    reimburse: false,
                },
            );

            // Send msg to user
            Self::deposit_event(Event::<T>::NewRedeemRequest);
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
            Self::ensure_chain_correct_status()?;
            ensure_signed(origin)?;

            // Ensure this is the correct vault
            let request =
                <RedeemRequests<T>>::get(request_id).ok_or(Error::<T>::RedeemRequestNotFound)?;

            // Ensure this redeem not expired
            let height = <frame_system::Pallet<T>>::block_number();
            let expired_time = <RedeemRequestExpiredTime<T>>::get();

            ensure!(
                height - request.open_time < expired_time,
                Error::<T>::RedeemRequestExpired
            );

            // TODO verify tx
            // TODO: premium redeem fee

            <vault::Vaults<T>>::mutate(&request.vault, |vault| {
                if let Some(vault) = vault {
                    vault.issued_tokens -= request.amount;
                    vault.to_be_redeemed_tokens -= request.amount;
                }
            });

            // Decrase user's XBTC amount.
            Self::burn_xbtc(&request.requester, request.amount)?;

            Self::remove_redeem_request(request_id, RedeemRequestStatus::Completed);

            Self::deposit_event(Event::<T>::RedeemExecuted);
            Ok(().into())
        }

        /// User cancle redeem
        #[pallet::weight(0)]
        pub fn cancle_redeem(
            origin: OriginFor<T>,
            request_id: RequestId,
            reimburse: bool,
        ) -> DispatchResultWithPostInfo {
            let sender = ensure_signed(origin)?;

            // Ensure sender is is redeem's owner
            let request =
                <RedeemRequests<T>>::get(request_id).ok_or(Error::<T>::RedeemRequestNotFound)?;
            ensure!(request.requester == sender, Error::<T>::UnauthorizedUser);

            // Ensure the redeem request right status
            ensure!(
                request.status == RedeemRequestStatus::Processing,
                Error::<T>::RedeemRequestProcessing
            );

            // Ensure the redeem request is outdate
            let height = <frame_system::Pallet<T>>::block_number();
            let expired_time = <RedeemRequestExpiredTime<T>>::get();
            ensure!(
                height - request.open_time > expired_time,
                Error::<T>::RedeemRequestNotExpired
            );

            let vault = vault::Pallet::<T>::get_active_vault_by_id(&request.vault)?;
            let worth_pcx = assets::Pallet::<T>::convert_to_pcx(request.amount)?;

            // Punish vault fee
            let punishment_fee: BalanceOf<T> = 0.into();

            if reimburse {
                // Decrease vault tokens
                vault::Vaults::<T>::mutate(&vault.id, |vault| {
                    if let Some(vault) = vault {
                        vault.to_be_redeemed_tokens -= request.amount;
                    }
                });

                // Vault give pcx to sender
                assets::Pallet::<T>::slash_collateral(
                    &request.vault,
                    &request.requester,
                    worth_pcx + punishment_fee,
                )?;
            } else {
                Self::release_xbtc(&request.requester, request.amount)?;
            }

            Self::remove_redeem_request(request_id, RedeemRequestStatus::Cancelled);
            Self::deposit_event(Event::<T>::RedeemCancelled);
            Ok(().into())
        }

        /// User liquidation redeem. when user do this means he can get pcx only.
        // FIXME(wangyafei): need to reimplement
        #[pallet::weight(0)]
        pub fn liquidation_redeem(
            origin: OriginFor<T>,
            redeem_amount: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {
            let sender = ensure_signed(origin)?;

            // Ensure redeem amount less than have
            let redeemer_balance = Self::asset_balance_of(&sender);
            ensure!(
                redeem_amount <= redeemer_balance,
                Error::<T>::InsufficiantAssetsFunds
            );

            // User burn xbtc
            Self::burn_xbtc(&sender, redeem_amount)?;

            // Catulate user's XBTC worth how much pcx, then give he the pcx
            let worth_pcx = assets::Pallet::<T>::convert_to_pcx(redeem_amount)?;

            // System vault give him pcx
            let system_vault = <vault::Liquidator<T>>::get();
            assets::Pallet::<T>::slash_collateral(&system_vault.id, &sender, worth_pcx)?;

            // Send msg to user
            Self::deposit_event(Event::<T>::RedeemLiquidated);
            Ok(().into())
        }

        /// Update expired time for requesting redeem
        #[pallet::weight(0)]
        pub fn update_expired_time(
            origin: OriginFor<T>,
            expired_time: BlockNumberFor<T>,
        ) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;
            <RedeemRequestExpiredTime<T>>::put(expired_time);
            Ok(().into())
        }
    }

    /// Events for redeem module
    #[pallet::event]
    #[pallet::generate_deposit(pub(crate) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Current chain status is not right
        ChainStatusError,
        /// Redeem request is accepted
        NewRedeemRequest,
        /// Cancel redeem is accepted
        RedeemCancelled,
        /// Liquidation redeem is accepted
        RedeemLiquidated,
        /// Execute redeem is accepted
        RedeemExecuted,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::error]
    pub enum Error<T> {
        /// Redeem request id is not exsit
        RedeemRequestNotFound,
        /// Redeem request cancelled for forced redeem when it's not expired.
        RedeemRequestNotExpired,
        /// Redeem request is expierd
        RedeemRequestExpired,
        /// Vault is under Liquidation
        VaultLiquidated,
        /// Actioner is not the request's owner
        UnauthorizedUser,
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
        VaultTokenInsufficiant,
    }

    /// Redeem fee when use request redeem
    #[pallet::storage]
    #[pallet::getter(fn redeem_fee)]
    pub(crate) type RedeemFee<T: Config> = StorageValue<_, u8, ValueQuery>;

    /// Auto-increament id to identify each redeem request.
    /// Also presents total amount of created requests.
    #[pallet::storage]
    pub(crate) type RequestCount<T: Config> = StorageValue<_, RequestId, ValueQuery>;

    /// The minimum amount of btc that is accepted for redeem requests; any lower values would
    /// Risk the bitcoin client to reject the payment
    #[pallet::storage]
    pub(crate) type RedeemBtcDustValue<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

    /// Mapping from redeem id to `RedeemRequest`
    #[pallet::storage]
    pub(crate) type RedeemRequests<T: Config> =
        StorageMap<_, Twox64Concat, RequestId, RedeemRequest<T>>;

    /// Expired time for an `RedeemRequest`
    #[pallet::storage]
    pub(crate) type RedeemRequestExpiredTime<T: Config> =
        StorageValue<_, BlockNumberFor<T>, ValueQuery>;

    impl<T: Config> Pallet<T> {
        /// Ensure the chain is in correct status
        fn ensure_chain_correct_status() -> DispatchResultWithPostInfo {
            let bridge_status = <assets::BridgeStatus<T>>::get();
            ensure!(
                bridge_status == crate::assets::types::Status::Running,
                Error::<T>::BridgeStatusError
            );
            Ok(().into())
        }

        /// Generate secure key from account id
        pub(crate) fn get_next_request_id() -> RequestId {
            <RequestCount<T>>::mutate(|n| {
                *n += 1;
                *n
            })
        }

        /// Get `RedeemssueRequest` from id
        pub(crate) fn get_redeem_request_by_id(request_id: RequestId) -> Option<RedeemRequest<T>> {
            <RedeemRequests<T>>::get(request_id)
        }

        /// Mark the request as removed
        fn remove_redeem_request(request_id: RequestId, status: RedeemRequestStatus) {
            <RedeemRequests<T>>::mutate(request_id, |request| {
                if let Some(request) = request {
                    request.status = status;
                }
            });
        }

        /// Lock XBTC
        fn lock_xbtc(user: &T::AccountId, count: BalanceOf<T>) -> DispatchResultWithPostInfo {
            xpallet_assets::Module::<T>::move_balance(
                &ASSET_ID,
                &user,
                AssetType::Usable,
                &user,
                AssetType::Locked,
                count,
            )
            .map_err::<xpallet_assets::Error<T>, _>(Into::into)?;
            Ok(().into())
        }

        /// Release XBTC
        fn release_xbtc(user: &T::AccountId, amount: BalanceOf<T>) -> DispatchResultWithPostInfo {
            xpallet_assets::Module::<T>::move_balance(
                &ASSET_ID,
                &user,
                AssetType::Locked,
                &user,
                AssetType::Usable,
                amount,
            )
            .map_err::<xpallet_assets::Error<T>, _>(Into::into)?;
            Ok(().into())
        }

        /// Burn XBTC
        fn burn_xbtc(user: &T::AccountId, count: BalanceOf<T>) -> DispatchResultWithPostInfo {
            xpallet_assets::Module::<T>::destroy_reserved_withdrawal(&ASSET_ID, &user, count)?;
            Ok(().into())
        }

        /// User have XBTC count
        fn asset_balance_of(user: &T::AccountId) -> BalanceOf<T> {
            xpallet_assets::Module::<T>::asset_balance_of(&user, &ASSET_ID, AssetType::Usable)
        }
    }
}
