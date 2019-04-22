// Copyright 2018-2019 Chainpool.
//! Virtual mining for holding tokens.

#![cfg_attr(not(feature = "std"), no_std)]

mod mock;
mod tests;
pub mod types;

use parity_codec::Encode;

// Substrate
use primitives::traits::{As, Hash};
use rstd::{prelude::*, result};
use substrate_primitives::crypto::UncheckedFrom;
use support::{
    decl_event, decl_module, decl_storage, dispatch::Result, ensure, StorageMap, StorageValue,
};

// ChainX
use xassets::{AssetErr, AssetType, ChainT, Token};
use xassets::{OnAssetChanged, OnAssetRegisterOrRevoke};
use xstaking::{ClaimType, OnReward, OnRewardCalculation, RewardHolder};
use xsupport::{debug, error, info};
#[cfg(feature = "std")]
use xsupport::{token, u8array_to_string};

pub use self::types::{
    DepositRecord, DepositVoteWeight, PseduIntentionProfs, PseduIntentionVoteWeight,
};

pub trait Trait: xsystem::Trait + xstaking::Trait + xspot::Trait + xsdot::Trait {
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
    T::BlockNumber: parity_codec::Codec,
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
        if <xassets::Module<T> as ChainT>::TOKEN.to_vec() == token.clone()
            || from.clone() == to.clone()
        {
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
        if <xassets::Module<T> as ChainT>::TOKEN.to_vec() == target.clone() {
            return;
        }

        Self::try_init_receiver_vote_weight(source, target);

        debug!(
            "[on_issue_before] deposit_records: ({:?}, {:?}) = {:?}",
            source,
            token!(target),
            Self::deposit_records((source.clone(), target.clone()))
        );

        Self::update_bare_vote_weight(source, target);
    }

    fn on_issue(target: &Token, source: &T::AccountId, value: T::Balance) -> Result {
        // Exclude PCX
        if <xassets::Module<T> as ChainT>::TOKEN.to_vec() == target.clone() {
            return Ok(());
        }

        debug!(
            "[on_issue] token: {:?}, who: {:?}, vlaue: {:?}",
            u8array_to_string(target),
            source,
            value
        );

        Self::issue_reward(source, target, value)
    }

    fn on_destroy(target: &Token, source: &T::AccountId, _value: T::Balance) -> Result {
        Self::update_bare_vote_weight(source, target);
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

    /// Transform outside irreversible blocks to native blocks.
    fn wait_blocks(token: &Token) -> result::Result<u64, &'static str> {
        let seconds_per_block: T::Moment = timestamp::Module::<T>::minimum_period();
        match token.as_slice() {
            // btc
            <xbitcoin::Module<T> as ChainT>::TOKEN => {
                let irr_block: u32 = <xbitcoin::Module<T>>::confirmation_number();
                let seconds = (irr_block * 10 * 60) as u64;
                Ok(seconds / seconds_per_block.as_())
            }
            <xsdot::Module<T> as ChainT>::TOKEN => Ok(0u64),
            _ => {
                error!(
                    "[wait_blocks] token {:?} is unsupported",
                    u8array_to_string(token)
                );
                Err("This token is not supported.")
            }
        }
    }

    fn issue_reward(source: &T::AccountId, token: &Token, value: T::Balance) -> Result {
        let psedu_intention = Self::psedu_intention_profiles(token);
        if psedu_intention.last_total_deposit_weight == 0 {
            info!(
                target: "tokens",
                "should issue reward to {:?}, but the last_total_deposit_weight of Token: {:?} is zero.",
                source,
                u8array_to_string(token)
            );
            return Ok(());
        }
        let blocks = Self::wait_blocks(token)?;

        let addr = T::DetermineTokenJackpotAccountId::accountid_for(token);
        let jackpot = xassets::Module::<T>::pcx_free_balance(&addr).as_();

        let depositor_vote_weight = blocks as u128 * value.as_() as u128;

        let reward = match depositor_vote_weight.checked_mul(jackpot as u128) {
            Some(x) => {
                let reward =
                    x / (depositor_vote_weight + psedu_intention.last_total_deposit_weight as u128);
                if reward > jackpot as u128 {
                    panic!("The issue reward is no more than the jackpot balance");
                }
                if reward < u64::max_value() as u128 {
                    T::Balance::sa(reward as u64)
                } else {
                    panic!("reward on issue definitely less than u64::max_value()")
                }
            }
            None => panic!("blocks * jackpot * value overflow on issue"),
        };

        xassets::Module::<T>::pcx_move_free_balance(&addr, source, reward).map_err(|e| e.info())?;

        Self::deposit_event(RawEvent::DepositorReward(
            source.clone(),
            token.clone(),
            value,
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
        let one_pcx = 10_u64.pow(Self::pcx_precision());

        if token.eq(&<xassets::Module<T> as ChainT>::TOKEN.to_vec()) {
            return Some(As::sa(one_pcx));
        }

        let discount = <TokenDiscount<T>>::get(token) as u64;

        if token.eq(&<xsdot::Module<T> as ChainT>::TOKEN.to_vec()) {
            return Some(As::sa(one_pcx * discount / 100));
        } else {
            if let Some(price) = <xspot::Module<T>>::aver_asset_price(token) {
                let power = match (price.as_() as u128).checked_mul(discount as u128) {
                    Some(x) => T::Balance::sa((x / 100) as u64),
                    None => panic!("price * discount overflow"),
                };

                return Some(power);
            }
        }

        None
    }

    // Convert the total issuance of some token to equivalent PCX, including the PCX precision.
    // aver_asset_price(token) * total_issuance(token) / 10^token.precision
    pub fn trans_pcx_stake(token: &Token) -> Option<T::Balance> {
        if let Some(power) = Self::asset_power(token) {
            match <xassets::Module<T>>::get_asset(token) {
                Ok(asset) => {
                    let pow_precision = 10_u128.pow(asset.precision() as u32);
                    let total_balance =
                        <xassets::Module<T>>::all_type_total_asset_balance(&token).as_();

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
