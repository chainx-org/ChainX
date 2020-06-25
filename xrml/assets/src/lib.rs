// Copyright 2018-2019 Chainpool.
//! Assets: Handles token asset balances.

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

mod pcx;
pub mod traits;
mod trigger;
pub mod types;

use codec::{Codec, Encode};
// Substrate
use sp_core::crypto::UncheckedFrom;
use sp_runtime::traits::{
    AtLeast32Bit, CheckedAdd, CheckedSub, Hash, MaybeSerializeDeserialize, Member, Zero,
};
use sp_std::{collections::btree_map::BTreeMap, fmt::Debug, prelude::*, result};

use frame_support::{
    decl_error, decl_event, decl_module, decl_storage,
    dispatch::{DispatchError, DispatchResult},
    ensure,
    traits::{Imbalance, OnNewAccount, SignedImbalance},
    Parameter,
};
use frame_system::{self as system, ensure_root, ensure_signed};

// ChainX
use chainx_primitives::{AssetId, Desc, Memo, Token};
use xrml_support::{debug, ensure_with_errorlog, info, str};

pub use self::traits::{ChainT, OnAssetChanged, OnAssetRegisterOrRevoke, TokenJackpotAccountIdFor};
use self::trigger::AssetTriggerEventAfter;

pub use self::types::{
    is_valid_desc, is_valid_token, Asset, AssetErr, AssetRestriction, AssetRestrictions, AssetType,
    Chain, NegativeImbalance, PositiveImbalance, SignedBalance, SignedImbalanceT,
};

pub struct SimpleAccountIdDeterminator<T: Trait>(::sp_std::marker::PhantomData<T>);

impl<AccountId: Default, BlockNumber> TokenJackpotAccountIdFor<AccountId, BlockNumber> for () {
    fn accountid_for(_: &AssetId) -> AccountId {
        AccountId::default()
    }
}

impl<T: Trait> TokenJackpotAccountIdFor<T::AccountId, T::BlockNumber>
    for SimpleAccountIdDeterminator<T>
where
    T::AccountId: UncheckedFrom<T::Hash>,
    T::BlockNumber: codec::Codec,
{
    fn accountid_for(id: &AssetId) -> T::AccountId {
        let id_hash = T::Hashing::hash(&id.to_le_bytes()[..]);
        let registered_time = Module::<T>::asset_registered_block(id);
        let block_num_hash = T::Hashing::hash(registered_time.encode().as_ref());

        let mut buf = Vec::new();
        buf.extend_from_slice(id_hash.as_ref());
        buf.extend_from_slice(block_num_hash.as_ref());
        UncheckedFrom::unchecked_from(T::Hashing::hash(&buf[..]))
    }
}

pub trait Trait: system::Trait {
    type Balance: Parameter
        + Member
        + AtLeast32Bit
        + Codec
        + Default
        + Copy
        + MaybeSerializeDeserialize
        + Debug;
    /// Event
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;

    type OnAssetChanged: OnAssetChanged<Self::AccountId, Self::Balance>;

    type OnAssetRegisterOrRevoke: OnAssetRegisterOrRevoke;

    /// Generate virtual AccountId for each (psedu) token
    type DetermineTokenJackpotAccountId: TokenJackpotAccountIdFor<
        Self::AccountId,
        Self::BlockNumber,
    >;
}

decl_error! {
    /// Error for the Assets Module
    pub enum Error for Module<T: Trait> {
        /// Token length is zero or too long
        InvalidAssetLen,
        /// Token name length is zero or too long
        InvalidAssetNameLen,
        /// Desc length is zero or too long
        InvalidDescLen,
        /// Memo length is zero or too long
        InvalidMemoLen,
        /// only allow ASCII alphanumeric character or '-', '.', '|', '~'
        InvalidChar,
        /// only allow ASCII alphanumeric character
        InvalidAsscii,
        ///
        AlreadyExistedToken,
        ///
        NotExistdAsset,
        ///
        InvalidAsset,
        /// Got an overflow after adding
        Overflow,
        /// Balance too low to send value
        InsufficientBalance,
        /// Got an overflow after adding
        TotalAssetOverflow,
        /// Balance too low to send value
        TotalAssetInsufficientBalance,


        /// should not be free type here
        NotAllowFreeType,
        /// should not use chainx token here
        NotAllowPcx,
        NotAllowAction,
    }
}

decl_event!(
    pub enum Event<T> where
        <T as system::Trait>::AccountId,
        <T as Trait>::Balance,
        SignedBalance = SignedBalance<T>,
    {
        Register(AssetId, bool),
        Revoke(AssetId),

        Move(AssetId, AccountId, AssetType, AccountId, AssetType, Balance),
        Issue(AssetId, AccountId, Balance),
        Destory(AssetId, AccountId, Balance),
        Set(AssetId, AccountId, AssetType, Balance),

        /// change token balance, SignedBalance mark Positive or Negative
        Change(AssetId, AccountId, AssetType, SignedBalance),
    }
);

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event() = default;

        /// register_asset to module, should allow by root
        #[weight = 0]
        pub fn register_asset(
            origin,
            #[compact] asset_id: AssetId,
            asset: Asset,
            restrictions: AssetRestrictions,
            is_online: bool,
            is_psedu_intention: bool
        ) -> DispatchResult {
            ensure_root(origin)?;
            asset.is_valid::<T>()?;
            info!("[register_asset]|id:{:}|{:?}|is_online:{:}|is_psedu_intention:{:}", asset_id, asset, is_online, is_psedu_intention);

            Self::add_asset(asset_id, asset, restrictions)?;

            T::OnAssetRegisterOrRevoke::on_register(&asset_id, is_psedu_intention)?;
            Self::deposit_event(RawEvent::Register(asset_id, is_psedu_intention));

            if !is_online {
                let _ = Self::revoke_asset(frame_system::RawOrigin::Root.into(), asset_id.into());
            }
            Ok(())
        }

        /// revoke asset, mark this asset is invalid
        #[weight = 0]
        pub fn revoke_asset(origin, #[compact] id: AssetId) -> DispatchResult {
            ensure_root(origin)?;
            ensure!(Self::asset_online(id).is_some(), Error::<T>::InvalidAsset);
            Self::remove_asset(&id)?;

            T::OnAssetRegisterOrRevoke::on_revoke(&id)?;
            Self::deposit_event(RawEvent::Revoke(id));
            Ok(())
        }

        /// set free token for an account
        #[weight = 0]
        pub fn set_balance(origin, who: T::AccountId, #[compact] id: AssetId, balances: BTreeMap<AssetType, T::Balance>) -> DispatchResult {
            ensure_root(origin)?;
            info!("[set_balance]|set balances by root|who:{:?}|id:{:}|balances_map:{:?}", who, id, balances);
            Self::set_balance_by_root(&who, &id, balances)?;
            Ok(())
        }

        /// transfer between account
        #[weight = 0]
        pub fn transfer(origin, dest: T::AccountId, #[compact] id: AssetId, #[compact] value: T::Balance, memo: Memo) -> DispatchResult {
            let transactor = ensure_signed(origin)?;
            debug!("[transfer]|from:{:?}|to:{:?}|id:{:}|value:{:?}|memo:{}", transactor, dest, id, value, memo);
            memo.check_validity().map_err(|_| Error::<T>::InvalidMemoLen)?;

            Self::can_transfer(&id)?;
            let _ = Self::move_free_balance(&id, &transactor, &dest, value).map_err::<Error::<T>, _>(Into::into)?;
            Ok(())
        }

        /// for transfer by root
        #[weight = 0]
        pub fn force_transfer(origin, transactor: T::AccountId, dest: T::AccountId, #[compact] id: AssetId, #[compact] value: T::Balance, memo: Memo) -> DispatchResult {
            ensure_root(origin)?;
            debug!("[force_transfer]|from:{:?}|to:{:?}|id:{:}|value:{:?}|memo:{}", transactor, dest, id, value, memo);
            memo.check_validity().map_err(|_| Error::<T>::InvalidMemoLen)?;

            Self::can_transfer(&id)?;
            let _ = Self::move_free_balance(&id, &transactor, &dest, value).map_err::<Error::<T>, _>(Into::into)?;
            Ok(())
        }

        #[weight = 0]
        pub fn modify_asset_info(origin, #[compact] id: AssetId, token: Option<Token>, token_name: Option<Token>, desc: Option<Desc>) -> DispatchResult {
            ensure_root(origin)?;
            let mut info = Self::asset_info(&id).ok_or(Error::<T>::InvalidAsset)?;

            token.map(|t| info.set_token(t));
            token_name.map(|name| info.set_token_name(name));
            desc.map(|desc| info.set_desc(desc));

            AssetInfo::insert(id, info);
            Ok(())
        }

        #[weight = 0]
        pub fn modify_asset_limit(origin, #[compact] id: AssetId, restriction: AssetRestriction, can_do: bool) -> DispatchResult {
            ensure_root(origin)?;
            // notice use `asset_info`, not `asset_online`
            ensure!(Self::asset_info(id).is_some(), Error::<T>::InvalidAsset);

            AssetRestrictionsOf::mutate(id, |current| {
                if can_do {
                    current.set(restriction);
                } else {
                    current.unset(restriction);
                }
            });
            Ok(())
        }
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as XAssets {
        /// Asset id list for Chain, different Chain has different id list
        pub AssetIdsOf get(fn asset_ids_of): map hasher(twox_64_concat) Chain => Vec<AssetId>;

        /// asset info for every asset, key is asset id
        pub AssetInfo get(fn asset_info): map hasher(twox_64_concat) AssetId => Option<Asset>;
        pub AssetOnline get(fn asset_online): map hasher(twox_64_concat) AssetId => Option<()>;
        pub AssetRegisteredBlock get(fn asset_registered_block): map hasher(twox_64_concat) AssetId => T::BlockNumber;
        /// asset extend limit properties, set asset "can do", example, `CanTransfer`, `CanDestroyWithdrawal`
        /// notice if not set AssetRestriction, default is true for this limit
        /// if want let limit make sense, must set false for the limit
        pub AssetRestrictionsOf get(fn asset_restrictions_of): map hasher(twox_64_concat) AssetId => AssetRestrictions;

        /// asset balance for user&asset_id, use btree_map to accept different asset type
        pub AssetBalance get(fn asset_balance):
            double_map hasher(blake2_128_concat) T::AccountId, hasher(twox_64_concat) AssetId => BTreeMap<AssetType, T::Balance>;
        /// asset balance for an asset_id, use btree_map to accept different asset type
        pub TotalAssetBalance get(fn total_asset_balance): map hasher(twox_64_concat) AssetId => BTreeMap<AssetType, T::Balance>;

        /// memo len
        pub MemoLen get(fn memo_len) config(): u32;
    }
    add_extra_genesis {
        config(assets): Vec<(AssetId, Asset, AssetRestrictions, bool, bool)>;
        config(endowed): BTreeMap<AssetId, Vec<(T::AccountId, T::Balance)>>;
        build(|config| {
            Module::<T>::initialize_assets(&config.assets, &config.endowed);
        })
    }
}

impl<T: Trait> ChainT for Module<T> {
    const ASSET_ID: AssetId = xrml_protocol::PCX;
    fn chain() -> Chain {
        Chain::ChainX
    }
}

impl<T: Trait> Module<T> {
    fn initialize_assets(
        assets: &Vec<(AssetId, Asset, AssetRestrictions, bool, bool)>,
        endowed_accounts: &BTreeMap<AssetId, Vec<(T::AccountId, T::Balance)>>,
    ) {
        for (id, asset, restrictions, is_online, is_psedu_intention) in assets {
            Self::register_asset(
                frame_system::RawOrigin::Root.into(),
                (*id).into(),
                asset.clone(),
                restrictions.clone(),
                *is_online,
                *is_psedu_intention,
            )
            .expect("genesis for asset must success");
        }

        for (id, endowed) in endowed_accounts.iter() {
            for (accountid, value) in endowed.iter() {
                Self::issue(id, accountid, *value).unwrap();
            }
        }
    }

    pub fn should_not_free_type(type_: AssetType) -> DispatchResult {
        if type_ == AssetType::Free {
            Err(Error::<T>::NotAllowFreeType)?;
        }
        Ok(())
    }

    pub fn should_not_chainx(id: &AssetId) -> DispatchResult {
        if *id == <Self as ChainT>::ASSET_ID {
            Err(Error::<T>::NotAllowPcx)?;
        }
        Ok(())
    }
}

// asset related
impl<T: Trait> Module<T> {
    /// add an asset into the storage, notice the asset must be valid
    fn add_asset(id: AssetId, asset: Asset, restrictions: AssetRestrictions) -> DispatchResult {
        let chain = asset.chain();
        if Self::asset_info(&id).is_some() {
            Err(Error::<T>::AlreadyExistedToken)?;
        }

        AssetInfo::insert(&id, asset);
        AssetRestrictionsOf::insert(&id, restrictions);
        AssetOnline::insert(&id, ());
        AssetRegisteredBlock::<T>::insert(&id, system::Module::<T>::block_number());

        AssetIdsOf::mutate(chain, |v| {
            if !v.contains(&id) {
                v.push(id.clone());
            }
        });
        Ok(())
    }

    fn remove_asset(id: &AssetId) -> DispatchResult {
        AssetOnline::remove(id);
        Ok(())
    }

    pub fn asset_ids() -> Vec<AssetId> {
        let mut v = Vec::new();
        for i in Chain::iterator() {
            v.extend(Self::asset_ids_of(i));
        }
        v
    }

    pub fn all_assets() -> Vec<(Asset, bool)> {
        let list = Self::asset_ids();
        list.into_iter()
            .filter_map(|id| {
                Self::asset_info(id).map(|asset| (asset, Self::asset_online(id).is_some()))
            })
            .collect()
    }

    pub fn valid_asset_ids() -> Vec<AssetId> {
        Self::asset_ids()
            .into_iter()
            .filter(|id| Self::asset_online(id).is_some())
            .collect()
    }

    pub fn valid_assets_of(who: &T::AccountId) -> Vec<(AssetId, BTreeMap<AssetType, T::Balance>)> {
        use frame_support::IterableStorageDoubleMap;
        AssetBalance::<T>::iter_prefix(who)
            .filter_map(|(id, map)| Self::asset_online(id).map(|_| (id, map)))
            .collect()
    }

    pub fn get_asset(id: &AssetId) -> result::Result<Asset, DispatchError> {
        if let Some(asset) = Self::asset_info(id) {
            if Self::asset_online(id).is_some() {
                Ok(asset)
            } else {
                Err(Error::<T>::InvalidAsset)?
            }
        } else {
            Err(Error::<T>::NotExistdAsset)?
        }
    }

    pub fn can_do(id: &AssetId, limit: AssetRestriction) -> bool {
        Self::asset_restrictions_of(id).contains(limit)
    }
    // can do wrapper
    #[inline]
    pub fn can_move(id: &AssetId) -> DispatchResult {
        ensure_with_errorlog!(
            Self::can_do(id, AssetRestriction::Move),
            Error::<T>::NotAllowAction,
            "this asset do not allow move|id:{:}|action:{:?}",
            id,
            AssetRestriction::Move,
        );
        Ok(())
    }

    #[inline]
    pub fn can_transfer(id: &AssetId) -> DispatchResult {
        ensure_with_errorlog!(
            Self::can_do(id, AssetRestriction::Transfer),
            Error::<T>::NotAllowAction,
            "this asset do not allow transfer|id:{:}|action:{:?}",
            id,
            AssetRestriction::Transfer,
        );
        Ok(())
    }

    #[inline]
    pub fn can_destroy_withdrawal(id: &AssetId) -> DispatchResult {
        ensure_with_errorlog!(
            Self::can_do(id, AssetRestriction::DestroyWithdrawal),
            Error::<T>::NotAllowAction,
            "this asset do not allow destroy withdrawal|id:{:}|action:{:?}",
            id,
            AssetRestriction::DestroyWithdrawal,
        );
        Ok(())
    }

    #[inline]
    pub fn can_destroy_free(id: &AssetId) -> DispatchResult {
        ensure_with_errorlog!(
            Self::can_do(id, AssetRestriction::DestroyFree),
            Error::<T>::NotAllowAction,
            "this asset do not allow destroy free|id:{:}|action:{:?}",
            id,
            AssetRestriction::DestroyFree,
        );
        Ok(())
    }
}

/// token issue destroy reserve/unreserve, it's core function
impl<T: Trait> Module<T> {
    pub fn all_type_total_asset_balance(id: &AssetId) -> T::Balance {
        let map = Self::total_asset_balance(id);
        map.values().fold(Zero::zero(), |acc, &x| acc + x)
    }

    pub fn all_type_asset_balance(who: &T::AccountId, id: &AssetId) -> T::Balance {
        let map = Self::asset_balance(who, id);
        map.values().fold(Zero::zero(), |acc, &x| acc + x)
    }

    pub fn asset_balance_of(who: &T::AccountId, id: &AssetId, type_: AssetType) -> T::Balance {
        Self::asset_type_balance(who, id, type_)
    }

    pub fn free_balance_of(who: &T::AccountId, id: &AssetId) -> T::Balance {
        Self::asset_type_balance(&who, &id, AssetType::Free)
    }

    fn asset_type_balance(who: &T::AccountId, id: &AssetId, type_: AssetType) -> T::Balance {
        let balance_map = Self::asset_balance(who, id);
        match balance_map.get(&type_) {
            Some(b) => *b,
            None => Zero::zero(),
        }
    }

    pub fn issue(id: &AssetId, who: &T::AccountId, value: T::Balance) -> DispatchResult {
        {
            ensure!(Self::asset_online(id).is_some(), Error::<T>::InvalidAsset);

            // may set storage inner
            Self::try_new_account(&who, id);

            let type_ = AssetType::Free;
            let _imbalance = Self::inner_issue(id, who, type_, value)?;
        }
        Ok(())
    }

    pub fn destroy(id: &AssetId, who: &T::AccountId, value: T::Balance) -> DispatchResult {
        {
            ensure!(Self::asset_online(id).is_some(), Error::<T>::InvalidAsset);
            Self::can_destroy_withdrawal(id)?;

            let type_ = AssetType::ReservedWithdrawal;

            let _imbalance = Self::inner_destroy(id, who, type_, value)?;
        }
        Ok(())
    }

    pub fn destroy_free(id: &AssetId, who: &T::AccountId, value: T::Balance) -> DispatchResult {
        {
            ensure!(Self::asset_online(id).is_some(), Error::<T>::InvalidAsset);
            Self::can_destroy_free(id)?;

            let type_ = AssetType::Free;

            let _imbalance = Self::inner_destroy(id, who, type_, value)?;
        }
        Ok(())
    }

    fn new_account(who: &T::AccountId) {
        T::OnNewAccount::on_new_account(&who);
        // set empty balance for pcx
        assert!(
            !AssetBalance::<T>::contains_key(&who, Self::ASSET_ID),
            "when new account, the pcx must not exist for this account!"
        );
        info!("[new_account]|create new account|who:{:?}", who);
        AssetBalance::<T>::insert(
            &who,
            Self::ASSET_ID,
            BTreeMap::<AssetType, T::Balance>::new(),
        );
        // Self::deposit_event(RawEvent::NewAccount(who.clone()));
    }

    fn try_new_account(who: &T::AccountId, id: &AssetId) {
        // lookup chainx balance
        let existed = if *id == Self::ASSET_ID {
            AssetBalance::<T>::contains_key(who, id)
        } else {
            AssetBalance::<T>::contains_key(who, Self::ASSET_ID)
        };

        if !existed {
            // init account
            Self::new_account(who);
        }
    }

    fn make_type_balance_be(
        who: &T::AccountId,
        id: &AssetId,
        type_: AssetType,
        new_balance: T::Balance,
    ) -> SignedImbalanceT<T> {
        let mut original: T::Balance = Zero::zero();
        AssetBalance::<T>::mutate(who, id, |balance_map| {
            if new_balance == Zero::zero() {
                // remove Zero balance to save space
                if let Some(old) = balance_map.remove(&type_) {
                    original = old;
                }
            } else {
                let balance = balance_map.entry(type_).or_default();
                original = *balance;
                // modify to new balance
                *balance = new_balance;
            }
        });
        let imbalance = if original <= new_balance {
            SignedImbalance::Positive(PositiveImbalance::<T>::new(
                new_balance - original,
                *id,
                type_,
            ))
        } else {
            SignedImbalance::Negative(NegativeImbalance::<T>::new(
                original - new_balance,
                *id,
                type_,
            ))
        };
        imbalance
    }

    fn inner_issue(
        id: &AssetId,
        who: &T::AccountId,
        type_: AssetType,
        value: T::Balance,
    ) -> result::Result<PositiveImbalance<T>, DispatchError> {
        let current = Self::asset_type_balance(&who, id, type_);

        debug!(
            "[issue]|issue to account|id:{:}|who:{:?}|type:{:?}|current:{:?}|value:{:?}",
            id, who, type_, current, value
        );
        // check
        let new = match current.checked_add(&value) {
            Some(b) => b,
            None => Err(Error::<T>::Overflow)?,
        };

        AssetTriggerEventAfter::<T>::on_issue_before(id, who);

        // set to storage
        let imbalance = Self::make_type_balance_be(who, id, type_, new);
        let positive = if let SignedImbalance::Positive(p) = imbalance {
            p
        } else {
            // Impossible, but be defensive.
            PositiveImbalance::<T>::new(Zero::zero(), *id, type_)
        };

        AssetTriggerEventAfter::<T>::on_issue(id, who, value)?;
        Ok(positive)
    }

    fn inner_destroy(
        id: &AssetId,
        who: &T::AccountId,
        type_: AssetType,
        value: T::Balance,
    ) -> result::Result<NegativeImbalance<T>, DispatchError> {
        let current = Self::asset_type_balance(&who, id, type_);

        debug!("[destroy_directly]|destroy asset for account|id:{:}|who:{:?}|type:{:?}|current:{:?}|destroy:{:?}",
               id, who, type_, current, value);
        // check
        let new = match current.checked_sub(&value) {
            Some(b) => b,
            None => Err(Error::<T>::InsufficientBalance)?,
        };

        AssetTriggerEventAfter::<T>::on_destroy_before(id, who);

        let imbalance = Self::make_type_balance_be(who, id, type_, new);
        let negative = if let SignedImbalance::Negative(n) = imbalance {
            n
        } else {
            // Impossible, but be defensive.
            NegativeImbalance::<T>::new(Zero::zero(), *id, type_)
        };

        AssetTriggerEventAfter::<T>::on_destroy(id, who, value)?;
        Ok(negative)
    }

    pub fn move_balance(
        id: &AssetId,
        from: &T::AccountId,
        from_type: AssetType,
        to: &T::AccountId,
        to_type: AssetType,
        value: T::Balance,
        do_trigger: bool,
    ) -> result::Result<(SignedImbalanceT<T>, SignedImbalanceT<T>), AssetErr> {
        // check
        ensure!(Self::asset_online(id).is_some(), AssetErr::InvalidAsset);
        Self::can_move(id).map_err(|_| AssetErr::NotAllow)?;

        if value == Zero::zero() {
            // value is zero, do not read storage, no event
            return Ok((
                SignedImbalance::Positive(PositiveImbalance::<T>::zero()),
                SignedImbalance::Positive(PositiveImbalance::<T>::zero()),
            ));
        }

        let from_balance = Self::asset_type_balance(from, id, from_type);
        let to_balance = Self::asset_type_balance(to, id, to_type);

        debug!("[move_balance]|id:{:}|from:{:?}|f_type:{:?}|f_balance:{:?}|to:{:?}|t_type:{:?}|t_balance:{:?}|value:{:?}",
               id, from, from_type, from_balance, to, to_type, to_balance, value);

        // judge balance is enough and test overflow
        let new_from_balance = match from_balance.checked_sub(&value) {
            Some(b) => b,
            None => return Err(AssetErr::NotEnough),
        };
        let new_to_balance = match to_balance.checked_add(&value) {
            Some(b) => b,
            None => return Err(AssetErr::OverFlow),
        };

        // finish basic check, start self check
        if from == to && from_type == to_type {
            // same account, same type, return directly
            // same account also do trigger
            if do_trigger {
                AssetTriggerEventAfter::<T>::on_move_before(
                    id, from, from_type, to, to_type, value,
                );
                AssetTriggerEventAfter::<T>::on_move(id, from, from_type, to, to_type, value)?;
            }
            return Ok((
                SignedImbalance::Positive(PositiveImbalance::<T>::zero()),
                SignedImbalance::Positive(PositiveImbalance::<T>::zero()),
            ));
        }

        // !!! all check pass, start set storage
        // for account to set storage
        if to_type == AssetType::Free {
            Self::try_new_account(to, id);
        }

        if do_trigger {
            AssetTriggerEventAfter::<T>::on_move_before(id, from, from_type, to, to_type, value);
        }

        let from_imbalance = Self::make_type_balance_be(from, id, from_type, new_from_balance);
        let to_imbalance = Self::make_type_balance_be(to, id, to_type, new_to_balance);

        if do_trigger {
            AssetTriggerEventAfter::<T>::on_move(id, from, from_type, to, to_type, value)?;
        }
        Ok((from_imbalance, to_imbalance))
    }

    pub fn move_free_balance(
        id: &AssetId,
        from: &T::AccountId,
        to: &T::AccountId,
        value: T::Balance,
    ) -> result::Result<(SignedImbalanceT<T>, SignedImbalanceT<T>), AssetErr> {
        Self::move_balance(id, from, AssetType::Free, to, AssetType::Free, value, true)
    }

    pub fn set_balance_by_root(
        who: &T::AccountId,
        id: &AssetId,
        balances: BTreeMap<AssetType, T::Balance>,
    ) -> DispatchResult {
        for (type_, val) in balances.into_iter() {
            let old_val = Self::asset_type_balance(who, id, type_);
            if old_val == val {
                continue;
            }

            let _imbalance = Self::make_type_balance_be(who, id, type_, val);

            AssetTriggerEventAfter::<T>::on_set_balance(id, who, type_, val)?;
        }
        Ok(())
    }
}

// wrapper for balances module
impl<T: Trait> Module<T> {
    pub fn pcx_free_balance(who: &T::AccountId) -> T::Balance {
        Self::asset_balance_of(who, &Self::ASSET_ID, AssetType::Free)
    }

    pub fn pcx_type_balance(who: &T::AccountId, type_: AssetType) -> T::Balance {
        Self::asset_balance_of(who, &Self::ASSET_ID, type_)
    }

    pub fn pcx_all_type_balance(who: &T::AccountId) -> T::Balance {
        Self::all_type_asset_balance(who, &Self::ASSET_ID)
    }

    pub fn pcx_total_balance() -> T::Balance {
        Self::all_type_total_asset_balance(&Self::ASSET_ID)
    }

    pub fn pcx_issue(who: &T::AccountId, value: T::Balance) -> DispatchResult {
        Self::issue(&Self::ASSET_ID, who, value)
    }

    pub fn pcx_move_balance(
        from: &T::AccountId,
        from_type: AssetType,
        to: &T::AccountId,
        to_type: AssetType,
        value: T::Balance,
    ) -> result::Result<(), AssetErr> {
        let _ = Self::move_balance(
            &<Self as ChainT>::ASSET_ID,
            from,
            from_type,
            to,
            to_type,
            value,
            true,
        )?;
        Ok(())
    }

    pub fn pcx_move_free_balance(
        from: &T::AccountId,
        to: &T::AccountId,
        value: T::Balance,
    ) -> result::Result<(), AssetErr> {
        Self::pcx_move_balance(from, AssetType::Free, to, AssetType::Free, value)
    }

    pub fn pcx_make_free_balance_be(who: &T::AccountId, value: T::Balance) -> SignedImbalanceT<T> {
        Self::try_new_account(who, &Self::ASSET_ID);
        let imbalance = Self::make_type_balance_be(who, &Self::ASSET_ID, AssetType::Free, value);
        let b = match imbalance {
            SignedImbalance::Positive(ref p) => SignedBalance::Positive(p.peek()),
            SignedImbalance::Negative(ref n) => SignedBalance::Negative(n.peek()),
        };
        Self::deposit_event(RawEvent::Change(
            Self::ASSET_ID,
            who.clone(),
            AssetType::Free,
            b,
        ));
        imbalance
    }
}
