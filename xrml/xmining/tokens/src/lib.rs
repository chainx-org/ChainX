// Copyright 2018-2019 Chainpool.
//! Virtual mining for holding tokens.

#![cfg_attr(not(feature = "std"), no_std)]

mod mock;
mod tests;
pub mod types;

// Substrate
use primitives::traits::As;
use rstd::{prelude::*, result};
use support::{
    decl_event, decl_module, decl_storage, dispatch::Result, ensure, StorageMap, StorageValue,
};

// ChainX
use xassets::{AssetErr, AssetType, ChainT, Token, TokenJackpotAccountIdFor};
use xassets::{OnAssetChanged, OnAssetRegisterOrRevoke};
use xstaking::{ClaimType, OnReward, OnRewardCalculation, RewardHolder};
#[cfg(feature = "std")]
use xsupport::token;
use xsupport::{debug, ensure_with_errorlog};

pub use self::types::{
    DepositRecord, DepositVoteWeight, PseduIntentionProfs, PseduIntentionVoteWeight,
};

pub trait Trait:
    xsystem::Trait
    + xstaking::Trait
    + xspot::Trait
    + xsdot::Trait
    + xbridge_common::Trait
    + xbitcoin::lockup::Trait
{
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

decl_event!(
    pub enum Event<T> where <T as xassets::Trait>::Balance, <T as system::Trait>::AccountId {
        DepositorReward(AccountId, Token, Balance),
        DepositorClaim(AccountId, Token, u64, u64, Balance),
    }
);

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event<T>() = default;

        fn claim(origin, token: Token) {
            let who = system::ensure_signed(origin)?;

            ensure!(
                <xassets::Module<T> as ChainT>::TOKEN.to_vec() != token,
                "Cannot claim from native asset via tokens module."
            );

            ensure!(
                Self::psedu_intentions().contains(&token),
                "Cannot claim from unsupport token."
            );

            debug!("[claim] who: {:?}, token: {:?}", who, token!(token));
            Self::apply_claim(&who, &token)?;
        }

        fn set_token_discount(token: Token, value: u32) {
            ensure!(value <= 100, "TokenDiscount cannot exceed 100.");
            <TokenDiscount<T>>::insert(token, value);
        }

        fn set_deposit_reward(value: T::Balance) {
            DepositReward::<T>::put(value);
        }
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as XTokens {
        pub TokenDiscount get(token_discount) build(|config: &GenesisConfig<T>| {
            config.token_discount.clone()
        }): map Token => u32;

        pub PseduIntentions get(psedu_intentions) : Vec<Token>;

        pub PseduIntentionProfiles get(psedu_intention_profiles): map Token => PseduIntentionVoteWeight<T::BlockNumber>;

        pub DepositRecords get(deposit_records): map (T::AccountId, Token) => DepositVoteWeight<T::BlockNumber>;

        /// when deposit success, reward some pcx to user for claiming. Default is 100000 = 0.001 PCX; 0.001*100000000
        pub DepositReward get(deposit_reward): T::Balance = As::sa(100_000);
    }

    add_extra_genesis {
        config(token_discount): Vec<(Token, u32)>;
    }
}

impl<T: Trait> OnAssetChanged<T::AccountId, T::Balance> for Module<T> {
    fn on_move_before(
        token: &Token,
        from: &T::AccountId,
        _: AssetType,
        to: &T::AccountId,
        _: AssetType,
        _value: T::Balance,
    ) {
        // Exclude PCX and asset type changes on same account.
        if <xassets::Module<T> as ChainT>::TOKEN == token.as_slice() || from.clone() == to.clone() {
            return;
        }

        Self::try_init_receiver_vote_weight(to, token);

        Self::update_depositor_vote_weight_only(from, token);
        Self::update_depositor_vote_weight_only(to, token);
    }

    fn on_move(
        _token: &Token,
        _from: &T::AccountId,
        _: AssetType,
        _to: &T::AccountId,
        _: AssetType,
        _value: T::Balance,
    ) -> result::Result<(), AssetErr> {
        Ok(())
    }

    fn on_issue_before(target: &Token, source: &T::AccountId) {
        // Exclude PCX
        if <xassets::Module<T> as ChainT>::TOKEN == target.as_slice() {
            return;
        }

        Self::try_init_receiver_vote_weight(source, target);

        debug!(
            "[on_issue_before] deposit_records: ({:?}, {:?}) = {:?}",
            token!(target),
            source,
            Self::deposit_records((source.clone(), target.clone()))
        );

        Self::update_bare_vote_weight(source, target);
    }

    fn on_issue(target: &Token, source: &T::AccountId, value: T::Balance) -> Result {
        // Exclude PCX
        if <xassets::Module<T> as ChainT>::TOKEN == target.as_slice() {
            return Ok(());
        }

        debug!(
            "[on_issue] token: {:?}, who: {:?}, vlaue: {:?}",
            token!(target),
            source,
            value
        );

        Self::issue_reward(source, target, value)
    }

    fn on_destroy_before(target: &Token, source: &T::AccountId) {
        Self::update_bare_vote_weight(source, target);
    }

    fn on_destroy(_target: &Token, _source: &T::AccountId, _value: T::Balance) -> Result {
        Ok(())
    }
}

impl<T: Trait> Module<T> {
    fn apply_claim(who: &T::AccountId, token: &Token) -> Result {
        let key = (who.clone(), token.clone());
        let mut p_vote_weight = <PseduIntentionProfiles<T>>::get(token);
        let mut d_vote_weight: DepositVoteWeight<T::BlockNumber> = Self::deposit_records(&key);

        {
            let mut prof = PseduIntentionProfs::<T> {
                token,
                staking: &mut p_vote_weight,
            };
            let addr = T::DetermineTokenJackpotAccountId::accountid_for_unsafe(token);

            let mut record = DepositRecord::<T> {
                depositor: who,
                token,
                staking: &mut d_vote_weight,
            };

            let (source_vote_weight, target_vote_weight, dividend) =
                <xstaking::Module<T>>::generic_claim(
                    &mut record,
                    who,
                    &mut prof,
                    &addr,
                    ClaimType::PseduIntention(token.clone()),
                )?;
            Self::deposit_event(RawEvent::DepositorClaim(
                who.clone(),
                token.clone(),
                source_vote_weight,
                target_vote_weight,
                dividend,
            ));
        }

        <PseduIntentionProfiles<T>>::insert(token, p_vote_weight);
        <DepositRecords<T>>::insert(&key, d_vote_weight);

        Ok(())
    }

    /// Ensure the vote weight of some depositor or transfer receiver is initialized.
    fn try_init_receiver_vote_weight(who: &T::AccountId, token: &Token) {
        let key = (who.clone(), token.clone());
        if !<DepositRecords<T>>::exists(&key) {
            <DepositRecords<T>>::insert(
                &key,
                DepositVoteWeight {
                    last_deposit_weight: 0,
                    last_deposit_weight_update: <system::Module<T>>::block_number(),
                },
            );
        }
    }

    fn issue_reward(source: &T::AccountId, token: &Token, _value: T::Balance) -> Result {
        ensure_with_errorlog!(
            Self::psedu_intentions().contains(&token),
            "Cannot issue deposit reward since this token is not a psedu intention.",
            "Cannot issue deposit reward since this token is not a psedu intention.|token:{:}",
            token!(token)
        );

        // when deposit(issue) success, reward some pcx for account to claim
        let reward_value = Self::deposit_reward();
        xbridge_common::Module::<T>::reward_from_jackpot(token, source, reward_value);

        Self::deposit_event(RawEvent::DepositorReward(
            source.clone(),
            token.clone(),
            reward_value,
        ));

        Ok(())
    }

    fn update_depositor_vote_weight_only(from: &T::AccountId, target: &Token) {
        let key = (from.clone(), target.clone());
        let mut d_vote_weight: DepositVoteWeight<T::BlockNumber> = Self::deposit_records(&key);

        {
            let mut record = DepositRecord::<T> {
                depositor: from,
                staking: &mut d_vote_weight,
                token: target,
            };

            <xstaking::Module<T>>::generic_update_vote_weight(&mut record);
        }

        <DepositRecords<T>>::insert(&key, d_vote_weight);
    }

    fn update_bare_vote_weight(source: &T::AccountId, target: &Token) {
        let key = (source.clone(), target.clone());
        let mut p_vote_weight = <PseduIntentionProfiles<T>>::get(target);
        let mut d_vote_weight: DepositVoteWeight<T::BlockNumber> = Self::deposit_records(&key);

        {
            let mut prof = PseduIntentionProfs::<T> {
                token: target,
                staking: &mut p_vote_weight,
            };
            let mut record = DepositRecord::<T> {
                depositor: source,
                token: target,
                staking: &mut d_vote_weight,
            };

            <xstaking::Module<T>>::update_bare_vote_weight_both_way(&mut prof, &mut record);
        }

        <PseduIntentionProfiles<T>>::insert(target, p_vote_weight);
        <DepositRecords<T>>::insert(&key, d_vote_weight);
    }

    #[cfg(feature = "std")]
    pub fn bootstrap_update_vote_weight(source: &T::AccountId, target: &Token) {
        Self::try_init_receiver_vote_weight(source, target);
        Self::update_bare_vote_weight(source, target);
    }
}

impl<T: Trait> OnAssetRegisterOrRevoke for Module<T> {
    fn on_register(token: &Token, is_psedu_intention: bool) -> Result {
        if !is_psedu_intention {
            return Ok(());
        }

        ensure!(
            !Self::psedu_intentions().contains(token),
            "Cannot register psedu intention repeatedly."
        );

        <PseduIntentions<T>>::mutate(|i| i.push(token.clone()));

        <PseduIntentionProfiles<T>>::insert(
            token,
            PseduIntentionVoteWeight {
                last_total_deposit_weight: 0,
                last_total_deposit_weight_update: <system::Module<T>>::block_number(),
            },
        );

        Ok(())
    }

    fn on_revoke(token: &Token) -> Result {
        <PseduIntentions<T>>::mutate(|v| {
            v.retain(|t| t != token);
        });
        Ok(())
    }
}

impl<T: Trait> OnRewardCalculation<T::AccountId, T::Balance> for Module<T> {
    fn psedu_intentions_info() -> Vec<(RewardHolder<T::AccountId>, T::Balance)> {
        Self::psedu_intentions()
            .into_iter()
            .filter(|token| Self::internal_asset_power(token).is_some())
            .map(|token| {
                let stake = Self::trans_pcx_stake(&token);
                (RewardHolder::PseduIntention(token), stake)
            })
            .filter(|(_, stake)| stake.is_some())
            .map(|(holder, stake)| (holder, stake.unwrap()))
            .collect()
    }
}

impl<T: Trait> OnReward<T::AccountId, T::Balance> for Module<T> {
    fn reward(token: &Token, value: T::Balance) {
        let addr = T::DetermineTokenJackpotAccountId::accountid_for_unsafe(token);
        let _ = xassets::Module::<T>::pcx_issue(&addr, value);
    }
}

impl<T: Trait> Module<T> {
    pub fn token_jackpot_accountid_for_unsafe(token: &Token) -> T::AccountId {
        T::DetermineTokenJackpotAccountId::accountid_for_unsafe(token)
    }

    pub fn multi_token_jackpot_accountid_for_unsafe(tokens: &[Token]) -> Vec<T::AccountId> {
        tokens
            .iter()
            .map(|t| T::DetermineTokenJackpotAccountId::accountid_for_unsafe(t))
            .collect()
    }

    fn one_pcx() -> u64 {
        let pcx = <xassets::Module<T> as ChainT>::TOKEN.to_vec();
        let pcx_asset = <xassets::Module<T>>::get_asset(&pcx).expect("PCX definitely exist.");

        10_u64.pow(pcx_asset.precision().as_())
    }

    /// This calculation doesn't take the DistributionRatio of cross-chain assets and native assets into account.
    pub fn internal_cross_chain_asset_power(token: &Token) -> Option<T::Balance> {
        let discount = u64::from(<TokenDiscount<T>>::get(token));

        // One SDOT 0.1 vote.
        if <xsdot::Module<T> as ChainT>::TOKEN == token.as_slice() {
            return Some(As::sa(Self::one_pcx() * discount / 100));
        } else {
            // L-BTC shares the price of X-BTC as it doesn't have a trading pair.
            let token = if <xbitcoin::lockup::Module<T> as ChainT>::TOKEN == token.as_slice() {
                <xbitcoin::Module<T> as ChainT>::TOKEN.to_vec()
            } else {
                token.clone()
            };

            if let Some(price) = <xspot::Module<T>>::aver_asset_price(&token) {
                let power = match (u128::from(price.as_())).checked_mul(u128::from(discount)) {
                    Some(x) => T::Balance::sa((x / 100) as u64),
                    None => panic!("price * discount overflow"),
                };

                return Some(power);
            }
        }

        None
    }

    /// Compute the mining power of the given token.
    pub fn internal_asset_power(token: &Token) -> Option<T::Balance> {
        // One PCX one vote.
        if <xassets::Module<T> as ChainT>::TOKEN == token.as_slice() {
            return Some(As::sa(Self::one_pcx()));
        }

        Self::internal_cross_chain_asset_power(token)
    }

    pub fn asset_power(token: &Token) -> Option<T::Balance> {
        let power = Self::internal_asset_power(token);

        if <xassets::Module<T> as ChainT>::TOKEN != token.as_slice() {
            if let Ok((num, denom)) =
                <xstaking::Module<T>>::cross_chain_assets_are_growing_too_fast()
            {
                let double_discounted = power.map(|p| u128::from(p.as_()) * num / denom);
                debug!(
                    "[asset_power] should reduce the power again: original power: {:?}, double discount: {:?}/{:?} => final power: {:?}",
                    power,
                    num, denom,
                    double_discounted
                );
                return double_discounted.map(|p| T::Balance::sa(p as u64));
            }
        }

        power
    }

    /// Convert the total issuance of some token to equivalent PCX, including the PCX precision.
    /// aver_asset_price(token) * total_issuance(token) / 10^token.precision
    pub fn trans_pcx_stake(token: &Token) -> Option<T::Balance> {
        if let Some(power) = Self::internal_asset_power(token) {
            if let Ok(asset) = <xassets::Module<T>>::get_asset(token) {
                let pow_precision = 10_u128.pow(u32::from(asset.precision()));
                let total_balance =
                    <xassets::Module<T>>::all_type_total_asset_balance(&token).as_();

                let total = match (u128::from(total_balance)).checked_mul(u128::from(power.as_())) {
                    Some(x) => T::Balance::sa((x / pow_precision) as u64),
                    None => panic!("total_balance * price overflow"),
                };

                return Some(total);
            }
        }

        None
    }
}
