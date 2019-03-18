// Copyright 2018 Chainpool.
//! Virtual mining for holding tokens.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
use serde_derive::{Deserialize, Serialize};

use parity_codec as codec;
use substrate_primitives::crypto::UncheckedFrom;

use codec::{Decode, Encode};
use primitives::traits::{As, Hash};
use rstd::prelude::*;
use rstd::result::Result as StdResult;
use runtime_support::{
    decl_event, decl_module, decl_storage, dispatch::Result, ensure, StorageMap, StorageValue,
};

use xassets::{AssetErr, AssetType, ChainT, Token};
use xassets::{OnAssetChanged, OnAssetRegisterOrRevoke};
use xstaking::{ClaimType, OnReward, OnRewardCalculation, RewardHolder, VoteWeight};
#[cfg(feature = "std")]
use xsupport::u8array_to_string;
use xsupport::{debug, info};

/// This module only tracks the vote weight related changes.
/// All the amount related has been taken care by assets module.
#[derive(PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct PseduIntentionVoteWeight<BlockNumber: Default> {
    pub last_total_deposit_weight: u64,
    pub last_total_deposit_weight_update: BlockNumber,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct DepositVoteWeight<BlockNumber: Default> {
    pub last_deposit_weight: u64,
    pub last_deposit_weight_update: BlockNumber,
}

/// `PseduIntentionProfs` and `DepositRecord` is to wrap the vote weight of token,
/// sharing the vote weight calculation logic originated from staking module.
pub struct PseduIntentionProfs<'a, T: Trait> {
    pub token: &'a Token,
    pub staking: &'a mut PseduIntentionVoteWeight<T::BlockNumber>,
}

pub struct DepositRecord<'a, T: Trait> {
    pub depositor: &'a T::AccountId,
    pub token: &'a Token,
    pub staking: &'a mut DepositVoteWeight<T::BlockNumber>,
}

impl<'a, T: Trait> VoteWeight<T::BlockNumber> for PseduIntentionProfs<'a, T> {
    fn amount(&self) -> u64 {
        xassets::Module::<T>::all_type_balance(&self.token).as_()
    }

    fn last_acum_weight(&self) -> u64 {
        self.staking.last_total_deposit_weight
    }

    fn last_acum_weight_update(&self) -> u64 {
        self.staking.last_total_deposit_weight_update.as_()
    }

    fn set_amount(&mut self, _: u64, _: bool) {}

    fn set_last_acum_weight(&mut self, latest_deposit_weight: u64) {
        self.staking.last_total_deposit_weight = latest_deposit_weight;
    }

    fn set_last_acum_weight_update(&mut self, current_block: T::BlockNumber) {
        self.staking.last_total_deposit_weight_update = current_block;
    }
}

impl<'a, T: Trait> VoteWeight<T::BlockNumber> for DepositRecord<'a, T> {
    fn amount(&self) -> u64 {
        xassets::Module::<T>::all_type_balance_of(&self.depositor, &self.token).as_()
    }

    fn last_acum_weight(&self) -> u64 {
        self.staking.last_deposit_weight
    }

    fn last_acum_weight_update(&self) -> u64 {
        self.staking.last_deposit_weight_update.as_()
    }

    fn set_amount(&mut self, _: u64, _: bool) {}

    fn set_last_acum_weight(&mut self, latest_deposit_weight: u64) {
        self.staking.last_deposit_weight = latest_deposit_weight;
    }

    fn set_last_acum_weight_update(&mut self, current_block: T::BlockNumber) {
        self.staking.last_deposit_weight_update = current_block;
    }
}

pub trait Trait: xsystem::Trait + xstaking::Trait + xspot::Trait {
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;

    type DetermineTokenJackpotAccountId: TokenJackpotAccountIdFor<
        Self::AccountId,
        Self::BlockNumber,
    >;
}

pub trait TokenJackpotAccountIdFor<AccountId: Sized, BlockNumber> {
    fn accountid_for(token: &Token) -> AccountId;
}

pub struct SimpleAccountIdDeterminator<T: Trait>(::rstd::marker::PhantomData<T>);

impl<T: Trait> TokenJackpotAccountIdFor<T::AccountId, T::BlockNumber>
    for SimpleAccountIdDeterminator<T>
where
    T::AccountId: UncheckedFrom<T::Hash>,
    T::BlockNumber: codec::Codec,
{
    fn accountid_for(token: &Token) -> T::AccountId {
        let (_, _, init_number) =
            xassets::Module::<T>::asset_info(token).expect("the asset must be existed before");
        let token_hash = T::Hashing::hash(token);
        let block_num_hash = T::Hashing::hash(init_number.encode().as_ref());

        let mut buf = Vec::new();
        buf.extend_from_slice(token_hash.as_ref());
        buf.extend_from_slice(block_num_hash.as_ref());
        UncheckedFrom::unchecked_from(T::Hashing::hash(&buf[..]))
    }
}

/// An event in this module.
decl_event!(
    pub enum Event<T> where <T as balances::Trait>::Balance, <T as system::Trait>::AccountId {
        Issue(AccountId, Token, Balance),
        Claim(AccountId, Token, u64, u64, Balance),
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
                Self::psedu_intentions().into_iter().find(|i| i.clone() == token).is_some(),
                "Cannot claim from unsupport token."
            );

            Self::apply_claim(&who, &token)?;
        }
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as XTokens {
        pub TokenDiscount get(token_discount) config(): u32 = 50;

        pub PseduIntentions get(psedu_intentions) : Vec<Token>;

        pub PseduIntentionProfiles get(psedu_intention_profiles): map Token => PseduIntentionVoteWeight<T::BlockNumber>;

        pub DepositRecords get(deposit_records): map (T::AccountId, Token) => DepositVoteWeight<T::BlockNumber>;
    }
}

impl<T: Trait> OnAssetChanged<T::AccountId, T::Balance> for Module<T> {
    fn on_move(
        token: &Token,
        from: &T::AccountId,
        _: AssetType,
        to: &T::AccountId,
        _: AssetType,
        value: T::Balance,
    ) -> StdResult<(), AssetErr> {
        // Exclude PCX and asset type changes on same account.
        if <xassets::Module<T> as ChainT>::TOKEN.to_vec() == token.clone()
            || from.clone() == to.clone()
        {
            return Ok(());
        }

        Self::update_vote_weight(from, token, value, false);
        Self::update_vote_weight(to, token, value, true);
        Ok(())
    }

    fn on_issue(target: &Token, source: &T::AccountId, value: T::Balance) -> Result {
        // Exclude PCX
        if <xassets::Module<T> as ChainT>::TOKEN.to_vec() == target.clone() {
            return Ok(());
        }

        debug!(
            "on_issue token: {:?}, who: {:?}, vlaue: {:?}",
            u8array_to_string(target),
            source,
            value
        );
        Self::issue_reward(source, target, value)?;
        Self::update_vote_weight(source, target, value, true);

        Ok(())
    }

    fn on_destroy(target: &Token, source: &T::AccountId, value: T::Balance) -> Result {
        Self::update_vote_weight(source, target, value, false);
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
            let addr = T::DetermineTokenJackpotAccountId::accountid_for(token);

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
            Self::deposit_event(RawEvent::Claim(
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

    /// Transform outside irreversible blocks to native blocks.
    fn wait_blocks(token: &Token) -> StdResult<u64, &'static str> {
        let seconds_per_block: T::Moment = timestamp::Module::<T>::block_period();
        match token.as_slice() {
            // btc
            <xbitcoin::Module<T> as ChainT>::TOKEN => {
                let irr_block: u32 = <xbitcoin::Module<T>>::confirmation_number();
                let seconds = (irr_block * 10 * 60) as u64;
                Ok(seconds / seconds_per_block.as_())
            }
            _ => Err("This token is not supported."),
        }
    }

    fn issue_reward(source: &T::AccountId, token: &Token, value: T::Balance) -> Result {
        let psedu_intention = Self::psedu_intention_profiles(token);
        if psedu_intention.last_total_deposit_weight == 0 {
            info!("should issue reward to {:?}, but the last_total_deposit_weight of Token: {:?} is zero.", source, u8array_to_string(token));
            return Ok(());
        }
        let blocks = Self::wait_blocks(token)?;

        let addr = T::DetermineTokenJackpotAccountId::accountid_for(token);
        let jackpot = xassets::Module::<T>::pcx_free_balance(&addr).as_();

        let reward = match (blocks as u128 * value.as_() as u128).checked_mul(jackpot as u128) {
            Some(x) => {
                let reward = x / psedu_intention.last_total_deposit_weight as u128;
                if reward < u64::max_value() as u128 {
                    T::Balance::sa(reward as u64)
                } else {
                    panic!("reward on issue definitely less than u64::max_value()")
                }
            }
            None => panic!("blocks * jackpot * value overflow on issue"),
        };

        xassets::Module::<T>::pcx_move_free_balance(&addr, source, reward).map_err(|e| e.info())?;

        Self::deposit_event(RawEvent::Issue(source.clone(), token.clone(), value));

        Ok(())
    }

    /// Actually update the vote weight and nomination balance of source and target.
    fn update_vote_weight(source: &T::AccountId, target: &Token, value: T::Balance, to_add: bool) {
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

            <xstaking::Module<T>>::update_vote_weight_both_way(
                &mut prof,
                &mut record,
                value.as_(),
                to_add,
            );
        }

        <PseduIntentionProfiles<T>>::insert(target, p_vote_weight);
        <DepositRecords<T>>::insert(&key, d_vote_weight);
    }

    #[cfg(feature = "std")]
    pub fn bootstrap_update_vote_weight(
        source: &T::AccountId,
        target: &Token,
        value: T::Balance,
        to_add: bool,
    ) {
        Self::update_vote_weight(source, target, value, to_add)
    }
}

impl<T: Trait> OnAssetRegisterOrRevoke for Module<T> {
    fn on_register(token: &Token, is_psedu_intention: bool) -> Result {
        if !is_psedu_intention {
            return Ok(());
        }

        ensure!(
            Self::psedu_intentions()
                .into_iter()
                .find(|i| i == token)
                .is_none(),
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
            .filter(|token| Self::asset_power(token).is_some())
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
        let addr = T::DetermineTokenJackpotAccountId::accountid_for(token);
        let _ = xassets::Module::<T>::pcx_issue(&addr, value);
    }
}

impl<T: Trait> Module<T> {
    pub fn token_jackpot_accountid_for(token: &Token) -> T::AccountId {
        T::DetermineTokenJackpotAccountId::accountid_for(token)
    }

    pub fn multi_token_jackpot_accountid_for(tokens: &Vec<Token>) -> Vec<T::AccountId> {
        tokens
            .into_iter()
            .map(|t| T::DetermineTokenJackpotAccountId::accountid_for(t))
            .collect()
    }

    fn pcx_precision() -> u32 {
        let pcx = <xassets::Module<T> as ChainT>::TOKEN.to_vec();
        let pcx_asset = <xassets::Module<T>>::get_asset(&pcx).expect("PCX definitely exist.");

        return pcx_asset.precision().as_();
    }
    pub fn asset_power(token: &Token) -> Option<T::Balance> {
        if token.eq(&b"SDOT".to_vec()) {
            return Some(As::sa(10_u64.pow(Self::pcx_precision() - 1))); //0.1 PCX
        } else if token.eq(&<xassets::Module<T> as ChainT>::TOKEN.to_vec()) {
            return Some(As::sa(10_u64.pow(Self::pcx_precision())));
        } else {
            if let Some(price) = <xspot::Module<T>>::aver_asset_price(token) {
                let discount = <TokenDiscount<T>>::get();

                let power = match (price.as_() as u128).checked_mul(discount as u128) {
                    Some(x) => T::Balance::sa((x / 100) as u64),
                    None => panic!("price * discount overflow"),
                };

                return Some(power);
            }
        }

        None
    }
    //资产总发行折合成PCX，已含PCX精度
    //aver_asset_price(token)*token的总发行量[含token的精度]/(10^token的精度)
    pub fn trans_pcx_stake(token: &Token) -> Option<T::Balance> {
        if let Some(power) = Self::asset_power(token) {
            match <xassets::Module<T>>::get_asset(token) {
                Ok(asset) => {
                    let pow_precision = 10_u128.pow(asset.precision() as u32);
                    let total_balance = <xassets::Module<T>>::all_type_balance(&token).as_();

                    let total = match (total_balance as u128).checked_mul(power.as_() as u128) {
                        Some(x) => T::Balance::sa((x / pow_precision) as u64),
                        None => panic!("total_balance * price overflow"),
                    };

                    return Some(total);
                }
                _ => {}
            }
        }

        None
    }
}
