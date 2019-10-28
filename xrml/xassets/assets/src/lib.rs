// Copyright 2018-2019 Chainpool.
//! Assets: Handles token asset balances.

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

mod pcx;
pub mod traits;
mod trigger;
pub mod types;

mod mock;
mod tests;

use parity_codec::{Codec, Encode};
// Substrate
use primitives::traits::{
    CheckedAdd, CheckedSub, Hash, MaybeDisplay, MaybeSerializeDebug, Member, SimpleArithmetic,
    StaticLookup, Zero,
};
use rstd::collections::btree_map::BTreeMap;
use rstd::{
    convert::{TryFrom, TryInto},
    prelude::*,
    result,
};
use substrate_primitives::crypto::UncheckedFrom;

use support::traits::{Imbalance, SignedImbalance};
use support::{decl_event, decl_module, decl_storage, dispatch::Result, Parameter, StorageMap};
use system::{ensure_signed, IsDeadAccount, OnNewAccount};

// ChainX
use xsupport::{debug, ensure_with_errorlog, error, info};
#[cfg(feature = "std")]
use xsupport::{token, u8array_to_string};

pub use self::traits::{ChainT, OnAssetChanged, OnAssetRegisterOrRevoke, TokenJackpotAccountIdFor};
use self::trigger::AssetTriggerEventAfter;

pub use self::types::{
    is_valid_desc, is_valid_memo, is_valid_token, Asset, AssetErr, AssetLimit, AssetType, Chain,
    Desc, DescString, Memo, NegativeImbalance, PositiveImbalance, Precision, SignedBalance,
    SignedImbalanceT, Token, TokenString,
};

pub struct SimpleAccountIdDeterminator<T: Trait>(::rstd::marker::PhantomData<T>);

impl<AccountId: Default, BlockNumber> TokenJackpotAccountIdFor<AccountId, BlockNumber> for () {
    fn accountid_for_unsafe(_: &Token) -> AccountId {
        AccountId::default()
    }
    fn accountid_for_safe(_: &Token) -> Option<AccountId> {
        Some(AccountId::default())
    }
}

impl<T: Trait> TokenJackpotAccountIdFor<T::AccountId, T::BlockNumber>
    for SimpleAccountIdDeterminator<T>
where
    T::AccountId: UncheckedFrom<T::Hash>,
    T::BlockNumber: parity_codec::Codec,
{
    fn accountid_for_unsafe(token: &Token) -> T::AccountId {
        Self::accountid_for_safe(token).expect("the asset must be existed before")
    }
    fn accountid_for_safe(token: &Token) -> Option<T::AccountId> {
        Module::<T>::asset_info(token).map(|(_, _, init_number)| {
            let token_hash = T::Hashing::hash(token);
            let block_num_hash = T::Hashing::hash(init_number.encode().as_ref());

            let mut buf = Vec::new();
            buf.extend_from_slice(token_hash.as_ref());
            buf.extend_from_slice(block_num_hash.as_ref());
            UncheckedFrom::unchecked_from(T::Hashing::hash(&buf[..]))
        })
    }
}

pub trait Trait: system::Trait {
    type Balance: Parameter
        + Member
        + SimpleArithmetic
        + From<u64>
        + Into<u64>
        + TryInto<u64>
        + TryFrom<u64>
        + Codec
        + Default
        + Copy
        + MaybeDisplay
        + MaybeSerializeDebug;
    /// Handler for when a new account is created.
    type OnNewAccount: OnNewAccount<Self::AccountId>;
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

decl_event!(
    pub enum Event<T> where
        <T as system::Trait>::AccountId,
        <T as Trait>::Balance,
        SignedBalance = SignedBalance<T>,
    {
        Move(Token, AccountId, AssetType, AccountId, AssetType, Balance),
        Issue(Token, AccountId, Balance),
        Destory(Token, AccountId, Balance),
        Set(Token, AccountId, AssetType, Balance),
        Register(Token, bool),
        Revoke(Token),
        NewAccount(AccountId),
        /// change token balance, SignedBalance mark Positive or Negative
        Change(Token, AccountId, AssetType, SignedBalance),
    }
);

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event<T>() = default;

        /// register_asset to module, should allow by root
        pub fn register_asset(asset: Asset, is_online: bool, is_psedu_intention: bool) -> Result {
            asset.is_valid()?;
            info!("[register_asset]|{:?}|is_online:{:}|is_psedu_intention:{:}", asset, is_online, is_psedu_intention);

            let token = asset.token();

            Self::add_asset(asset)?;

            T::OnAssetRegisterOrRevoke::on_register(&token, is_psedu_intention)?;
            Self::deposit_event(RawEvent::Register(token.clone(), is_psedu_intention));

            if !is_online {
                let _ = Self::revoke_asset(token);
            }
            Ok(())
        }

        /// revoke asset, mark this asset is invalid
        pub fn revoke_asset(token: Token) -> Result {
            is_valid_token(&token)?;
            Self::remove_asset(&token)?;

            T::OnAssetRegisterOrRevoke::on_revoke(&token)?;
            Self::deposit_event(RawEvent::Revoke(token));
            Ok(())
        }

        /// set free token for an account
        pub fn set_balance(who: <T::Lookup as StaticLookup>::Source, token: Token, balances: BTreeMap<AssetType, T::Balance>) -> Result {
            let who = <T as system::Trait>::Lookup::lookup(who)?;
            info!("[set_balance]|set balances by root|who:{:?}|token:{:}|balances_map:{:?}", who, token!(token), balances);
            Self::set_balance_by_root(&who, &token, balances)?;
            Ok(())
        }

        /// transfer between account
        pub fn transfer(origin, dest: <T::Lookup as StaticLookup>::Source, token: Token, value: T::Balance, memo: Memo) -> Result {
            let transactor = ensure_signed(origin)?;
            let dest = <T as system::Trait>::Lookup::lookup(dest)?;
            debug!("[transfer]|from:{:?}|to:{:?}|token:{:}|value:{:}|memo:{:}", transactor, dest, token!(token), value, u8array_to_string(&memo));
            is_valid_memo::<T>(&memo)?;
            if transactor == dest {
                return Ok(())
            }

            Self::can_transfer(&token)?;
            let _ = Self::move_free_balance(&token, &transactor, &dest, value).map_err(|e| e.info())?;
            Ok(())
        }

        pub fn modify_asset_info(token: Token, token_name: Option<Token>, desc: Option<Desc>) {
            if let Some(ref mut info) = Self::asset_info(&token) {
                token_name.map(|name| info.0.set_token_name(name));
                desc.map(|desc| info.0.set_desc(desc));

                AssetInfo::<T>::insert(token, info);
            } else {
                error!("[modify_asset_info]|asset not exist|token:{:}", token!(token));
            }
        }

        pub fn set_asset_limit_props(token: Token, props: BTreeMap<AssetLimit, bool>) {
            if Self::asset_info(&token).is_some() {
                AssetLimitProps::<T>::insert(&token, props)
            } else {
                error!("[set_asset_limit_props]|asset not exist|token:{:}", token!(token));
            }
        }

        pub fn modify_asset_limit(token: Token, limit: AssetLimit, can_do: bool) {
            if Self::asset_info(&token).is_some() {
                AssetLimitProps::<T>::mutate(token, |limit_map| {
                    if can_do {
                        limit_map.remove(&limit);
                    } else {
                        limit_map.insert(limit, false);
                    }
                })
            } else {
                error!("[set_asset_limit_props]|asset not exist|token:{:}", token!(token));
            }
        }
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as XAssets {
        /// Asset token index list for Chain, different Chain has different token list
        pub AssetList get(asset_list): map Chain => Vec<Token>;

        /// asset info for every token, key is token token
        pub AssetInfo get(asset_info): map Token => Option<(Asset, bool, T::BlockNumber)>;
        /// asset extend limit properties, set asset "can do", example, `CanTransfer`, `CanDestroyWithdrawal`
        /// notice if not set AssetLimit, default is true for this limit
        /// if want let limit make sense, must set false for the limit
        pub AssetLimitProps get(asset_limit_props): map Token => BTreeMap<AssetLimit, bool>;

        /// asset balance for user&token, use btree_map to accept different asset type
        pub AssetBalance get(asset_balance): map (T::AccountId, Token) => BTreeMap<AssetType, T::Balance>;
        /// asset balance for a token, use btree_map to accept different asset type
        pub TotalAssetBalance get(total_asset_balance): map Token => BTreeMap<AssetType, T::Balance>;

        /// memo len
        pub MemoLen get(memo_len) config(): u32;
    }

}

impl<T: Trait> ChainT for Module<T> {
    const TOKEN: &'static [u8] = b"PCX";
    fn chain() -> Chain {
        Chain::ChainX
    }
}

impl<T: Trait> Module<T> {
    #[cfg(feature = "std")]
    pub fn bootstrap_register_asset(
        asset: Asset,
        is_online: bool,
        is_psedu_intention: bool,
    ) -> Result {
        Self::register_asset(asset, is_online, is_psedu_intention)
    }

    pub fn should_not_free_type(type_: AssetType) -> Result {
        if type_ == AssetType::Free {
            return Err("should not be free type here");
        }
        Ok(())
    }

    pub fn should_not_chainx(token: &Token) -> Result {
        if token.as_slice() == <Self as ChainT>::TOKEN {
            return Err("should not use chainx token here");
        }
        Ok(())
    }
}

// asset related
impl<T: Trait> Module<T> {
    /// add an asset into the storage, notice the asset must be valid
    fn add_asset(asset: Asset) -> Result {
        let token = asset.token();
        let chain = asset.chain();
        if AssetInfo::<T>::exists(&token) {
            return Err("already has this token");
        }

        AssetInfo::<T>::insert(&token, (asset, true, system::Module::<T>::block_number()));

        AssetList::<T>::mutate(chain, |v| {
            if !v.contains(&token) {
                v.push(token.clone());
            }
        });
        Ok(())
    }

    fn remove_asset(token: &Token) -> Result {
        if let Some(mut info) = Self::asset_info(token) {
            // let chain = info.0.chain();
            info.1 = false;
            AssetInfo::<T>::insert(token.clone(), info);
            // remove this token index from AssetList
            // AssetList::<T>::mutate(chain, |v| {
            //     v.retain(|i| i != token);
            // });

            Ok(())
        } else {
            Err("this token dose not register yet or is invalid")
        }
    }

    pub fn is_valid_asset(token: &Token) -> Result {
        is_valid_token(token)?;

        if let Some(info) = Self::asset_info(token) {
            if info.1 == true {
                return Ok(());
            }
            return Err("not a valid token");
        }
        Err("not a registered token")
    }

    pub fn assets() -> Vec<Token> {
        let mut v = Vec::new();
        for i in Chain::iterator() {
            v.extend(Self::asset_list(i));
        }
        v
    }

    pub fn all_assets() -> Vec<(Asset, bool)> {
        let list = Self::assets();
        let mut v = Vec::new();
        for token in list {
            if let Some((asset, valid, _)) = Self::asset_info(token) {
                v.push((asset, valid))
            }
        }
        v
    }

    /// notice don't call this func in runtime
    pub fn valid_assets() -> Vec<Token> {
        Self::assets()
            .into_iter()
            .filter(|t| {
                if let Some(t) = Self::asset_info(t) {
                    t.1
                } else {
                    false
                }
            })
            .collect()
    }

    pub fn valid_assets_of(who: &T::AccountId) -> Vec<(Token, BTreeMap<AssetType, T::Balance>)> {
        let tokens = Self::valid_assets();
        let mut list = Vec::new();
        for token in tokens.into_iter() {
            let key = (who.clone(), token.clone());
            if AssetBalance::<T>::exists(&key) {
                let map = Self::asset_balance(&key);
                list.push((token, map));
            }
        }
        list
    }

    pub fn get_asset(token: &Token) -> result::Result<Asset, &'static str> {
        if let Some((asset, valid, _)) = Self::asset_info(token) {
            if valid == false {
                return Err("this asset is invalid, maybe has been revoked.");
            }
            Ok(asset)
        } else {
            return Err("this token asset not exist!");
        }
    }

    pub fn can_do(token: &Token, limit: AssetLimit) -> bool {
        Self::asset_limit_props(token)
            .get(&limit)
            .map(|b| *b)
            .unwrap_or(true)
    }
    // can do wrapper
    #[inline]
    pub fn can_move(token: &Token) -> Result {
        ensure_with_errorlog!(
            Self::can_do(token, AssetLimit::CanMove),
            "this asset do not allow move",
            "this asset do not allow move|token:{:}",
            token!(token)
        );
        Ok(())
    }

    #[inline]
    pub fn can_transfer(token: &Token) -> Result {
        ensure_with_errorlog!(
            Self::can_do(token, AssetLimit::CanTransfer),
            "this asset do not allow transfer",
            "this asset do not allow transfer|token:{:}",
            token!(token)
        );
        Ok(())
    }

    #[inline]
    pub fn can_destroy_withdrawal(token: &Token) -> Result {
        ensure_with_errorlog!(
            Self::can_do(token, AssetLimit::CanDestroyWithdrawal),
            "this asset do not allow destroy withdrawal",
            "this asset do not allow destroy withdrawal|token:{:}",
            token!(token)
        );
        Ok(())
    }

    #[inline]
    pub fn can_destroy_free(token: &Token) -> Result {
        ensure_with_errorlog!(
            Self::can_do(token, AssetLimit::CanDestroyFree),
            "this asset do not allow destroy free token",
            "this asset do not allow destroy free token|token:{:}",
            token!(token)
        );
        Ok(())
    }
}

/// token issue destroy reserve/unreserve, it's core function
impl<T: Trait> Module<T> {
    pub fn all_type_total_asset_balance(token: &Token) -> T::Balance {
        let map = Self::total_asset_balance(token);
        map.values().fold(Zero::zero(), |acc, &x| acc + x)
    }

    pub fn all_type_asset_balance(who: &T::AccountId, token: &Token) -> T::Balance {
        let key = (who.clone(), token.clone());
        let map = Self::asset_balance(key);
        map.values().fold(Zero::zero(), |acc, &x| acc + x)
    }

    pub fn asset_balance_of(who: &T::AccountId, token: &Token, type_: AssetType) -> T::Balance {
        Self::asset_type_balance(&(who.clone(), token.clone()), type_)
    }

    pub fn free_balance_of(who: &T::AccountId, token: &Token) -> T::Balance {
        Self::asset_type_balance(&(who.clone(), token.clone()), AssetType::Free)
    }

    fn asset_type_balance(who_token: &(T::AccountId, Token), type_: AssetType) -> T::Balance {
        let balance_map = Self::asset_balance(who_token);
        match balance_map.get(&type_) {
            Some(b) => *b,
            None => Zero::zero(),
        }
    }

    pub fn issue(token: &Token, who: &T::AccountId, value: T::Balance) -> Result {
        {
            Self::is_valid_asset(token)?;

            // may set storage inner
            Self::try_new_account(&(who.clone(), token.clone()));

            let type_ = AssetType::Free;
            debug!("[issue]normal issue token for this account");
            let _imbalance = Self::inner_issue(token, who, type_, value)?;
        }
        Ok(())
    }

    pub fn destroy(token: &Token, who: &T::AccountId, value: T::Balance) -> Result {
        {
            Self::should_not_chainx(token)?;
            Self::is_valid_asset(token)?;

            Self::can_destroy_withdrawal(token)?;

            let type_ = AssetType::ReservedWithdrawal;

            debug!("[destroy]|normal destroy withdrawal token for account");
            let _imbalance = Self::inner_destroy(token, who, type_, value)?;
        }
        Ok(())
    }

    pub fn destroy_free(token: &Token, who: &T::AccountId, value: T::Balance) -> Result {
        {
            Self::should_not_chainx(token)?;
            Self::is_valid_asset(token)?;

            Self::can_destroy_free(token)?;

            let type_ = AssetType::Free;

            debug!("[destroy_free]|destroy free token for account directly");
            let _imbalance = Self::inner_destroy(token, who, type_, value)?;
        }
        Ok(())
    }

    fn new_account(who: &T::AccountId) {
        T::OnNewAccount::on_new_account(&who);
        // set empty balance for pcx
        let key = (who.clone(), Self::TOKEN.to_vec());
        assert!(
            !AssetBalance::<T>::exists(&key),
            "when new account, the pcx must not exist for this account!"
        );
        info!("[new_account]|create new account|who:{:?}", who);
        AssetBalance::<T>::insert(&key, BTreeMap::new());
        Self::deposit_event(RawEvent::NewAccount(who.clone()));
    }

    fn try_new_account(who_token: &(T::AccountId, Token)) {
        // lookup chainx balance
        let existed = if who_token.1.as_slice() == Self::TOKEN {
            AssetBalance::<T>::exists(who_token)
        } else {
            AssetBalance::<T>::exists(&(who_token.0.clone(), Self::TOKEN.to_vec()))
        };

        if !existed {
            // init account
            Self::new_account(&who_token.0);
        }
    }

    fn make_type_balance_be(
        who_token: &(T::AccountId, Token),
        type_: AssetType,
        new_balance: T::Balance,
    ) -> SignedImbalanceT<T> {
        let mut original: T::Balance = Zero::zero();
        AssetBalance::<T>::mutate(who_token, |balance_map| {
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
                who_token.1.clone(),
                type_,
            ))
        } else {
            SignedImbalance::Negative(NegativeImbalance::<T>::new(
                original - new_balance,
                who_token.1.clone(),
                type_,
            ))
        };
        imbalance
    }

    fn inner_issue(
        token: &Token,
        who: &T::AccountId,
        type_: AssetType,
        value: T::Balance,
    ) -> result::Result<PositiveImbalance<T>, &'static str> {
        let key = (who.clone(), token.clone());
        let current = Self::asset_type_balance(&key, type_);

        debug!(
            "[issue]|issue to account|token:{:}|who:{:?}|type:{:?}|current:{:}|value:{:}",
            token!(token),
            who,
            type_,
            current,
            value
        );
        // check
        let new = match current.checked_add(&value) {
            Some(b) => b,
            None => return Err("current balance too high to issue"),
        };

        AssetTriggerEventAfter::<T>::on_issue_before(token, who);

        // set to storage
        let imbalance = Self::make_type_balance_be(&key, type_, new);
        let positive = if let SignedImbalance::Positive(p) = imbalance {
            p
        } else {
            // Impossible, but be defensive.
            PositiveImbalance::<T>::new(Zero::zero(), token.clone(), type_)
        };

        AssetTriggerEventAfter::<T>::on_issue(token, who, value)?;
        Ok(positive)
    }

    fn inner_destroy(
        token: &Token,
        who: &T::AccountId,
        type_: AssetType,
        value: T::Balance,
    ) -> result::Result<NegativeImbalance<T>, &'static str> {
        let key = (who.clone(), token.clone());
        let current = Self::asset_type_balance(&key, type_);

        debug!("[destroy_directly]|destroy token for account|token:{:}|who:{:?}|type:{:?}|current:{:}|destroy:{:}",
               token!(token), who, type_, current, value);
        // check
        let new = match current.checked_sub(&value) {
            Some(b) => b,
            None => return Err("current balance too low to destroy"),
        };

        AssetTriggerEventAfter::<T>::on_destroy_before(token, who);

        let imbalance = Self::make_type_balance_be(&key, type_, new);
        let negative = if let SignedImbalance::Negative(n) = imbalance {
            n
        } else {
            // Impossible, but be defensive.
            NegativeImbalance::<T>::new(Zero::zero(), token.clone(), type_)
        };

        AssetTriggerEventAfter::<T>::on_destroy(token, who, value)?;
        Ok(negative)
    }

    pub fn move_balance(
        token: &Token,
        from: &T::AccountId,
        from_type: AssetType,
        to: &T::AccountId,
        to_type: AssetType,
        value: T::Balance,
    ) -> result::Result<(SignedImbalanceT<T>, SignedImbalanceT<T>), AssetErr> {
        if from == to && from_type == to_type {
            // same account, same type, return directly
            return Ok((
                SignedImbalance::Positive(PositiveImbalance::<T>::zero()),
                SignedImbalance::Positive(PositiveImbalance::<T>::zero()),
            ));
        }
        if value == Zero::zero() {
            // value is zero, do not read storage, no event
            return Ok((
                SignedImbalance::Positive(PositiveImbalance::<T>::zero()),
                SignedImbalance::Positive(PositiveImbalance::<T>::zero()),
            ));
        }
        // check
        Self::is_valid_asset(token).map_err(|_| AssetErr::InvalidToken)?;

        Self::can_move(token).map_err(|_| AssetErr::NotAllow)?;

        let from_key = (from.clone(), token.clone());
        let to_key = (to.clone(), token.clone());

        let from_balance = Self::asset_type_balance(&from_key, from_type);
        let to_balance = Self::asset_type_balance(&to_key, to_type);

        debug!("[move_balance]|token:{:}|from:{:?}|f_type:{:?}|f_balance:{:}|to:{:?}|t_type:{:?}|t_balance:{:}|value:{:}",
               token!(token), from, from_type, from_balance, to, to_type, to_balance, value);

        // test overflow
        let new_from_balance = match from_balance.checked_sub(&value) {
            Some(b) => b,
            None => return Err(AssetErr::NotEnough),
        };
        let new_to_balance = match to_balance.checked_add(&value) {
            Some(b) => b,
            None => return Err(AssetErr::OverFlow),
        };

        // for account to set storage
        if to_type == AssetType::Free {
            Self::try_new_account(&to_key);
        }

        AssetTriggerEventAfter::<T>::on_move_before(token, from, from_type, to, to_type, value);

        let from_imbalance = Self::make_type_balance_be(&from_key, from_type, new_from_balance);
        let to_imbalance = Self::make_type_balance_be(&to_key, to_type, new_to_balance);

        AssetTriggerEventAfter::<T>::on_move(token, from, from_type, to, to_type, value)?;

        Ok((from_imbalance, to_imbalance))
    }

    pub fn move_free_balance(
        token: &Token,
        from: &T::AccountId,
        to: &T::AccountId,
        value: T::Balance,
    ) -> result::Result<(SignedImbalanceT<T>, SignedImbalanceT<T>), AssetErr> {
        Self::move_balance(token, from, AssetType::Free, to, AssetType::Free, value)
    }

    pub fn set_balance_by_root(
        who: &T::AccountId,
        token: &Token,
        balances: BTreeMap<AssetType, T::Balance>,
    ) -> Result {
        for (type_, val) in balances.into_iter() {
            let key = (who.clone(), token.clone());

            let old_val = Self::asset_type_balance(&key, type_);
            if old_val == val {
                continue;
            }

            let _imbalance = Self::make_type_balance_be(&key, type_, val);

            AssetTriggerEventAfter::<T>::on_set_balance(token, who, type_, val)?;
        }
        Ok(())
    }
}

// wrapper for balances module
impl<T: Trait> Module<T> {
    pub fn pcx_free_balance(who: &T::AccountId) -> T::Balance {
        Self::asset_balance_of(who, &Self::TOKEN.to_vec(), AssetType::Free)
    }

    pub fn pcx_type_balance(who: &T::AccountId, type_: AssetType) -> T::Balance {
        Self::asset_balance_of(who, &Self::TOKEN.to_vec(), type_)
    }

    pub fn pcx_all_type_balance(who: &T::AccountId) -> T::Balance {
        Self::all_type_asset_balance(who, &Self::TOKEN.to_vec())
    }

    pub fn pcx_total_balance() -> T::Balance {
        Self::all_type_total_asset_balance(&Self::TOKEN.to_vec())
    }

    pub fn pcx_issue(who: &T::AccountId, value: T::Balance) -> Result {
        Self::issue(&Self::TOKEN.to_vec(), who, value)
    }

    pub fn pcx_move_balance(
        from: &T::AccountId,
        from_type: AssetType,
        to: &T::AccountId,
        to_type: AssetType,
        value: T::Balance,
    ) -> result::Result<(), AssetErr> {
        let _ = Self::move_balance(
            &<Self as ChainT>::TOKEN.to_vec(),
            from,
            from_type,
            to,
            to_type,
            value,
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
        let key = (who.clone(), <Self as ChainT>::TOKEN.to_vec());
        Self::try_new_account(&key);
        let imbalance = Self::make_type_balance_be(&key, AssetType::Free, value);
        let b = match imbalance {
            SignedImbalance::Positive(ref p) => SignedBalance::Positive(p.peek()),
            SignedImbalance::Negative(ref n) => SignedBalance::Negative(n.peek()),
        };
        Self::deposit_event(RawEvent::Change(
            <Self as ChainT>::TOKEN.to_vec(),
            who.clone(),
            AssetType::Free,
            b,
        ));
        imbalance
    }
}

impl<T: Trait> IsDeadAccount<T::AccountId> for Module<T>
where
    T::Balance: MaybeSerializeDebug,
{
    fn is_dead_account(_who: &T::AccountId) -> bool {
        false
    }
}
