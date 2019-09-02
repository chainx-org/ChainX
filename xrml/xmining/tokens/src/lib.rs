// Copyright 2018-2019 Chainpool.
//! Virtual mining for holding tokens.

#![cfg_attr(not(feature = "std"), no_std)]

pub mod cross_mining;
mod impls;
mod mock;
mod tests;
pub mod types;
mod vote_weight;

use crate as xtokens;

// Substrate
use primitives::traits::{As, Zero};
use rstd::{prelude::*, result};
use support::{
    decl_event, decl_module, decl_storage, dispatch::Result, ensure, StorageMap, StorageValue,
};

// ChainX
use xassets::{AssetErr, AssetType, ChainT, Token, TokenJackpotAccountIdFor};
use xassets::{OnAssetChanged, OnAssetRegisterOrRevoke};
use xstaking::{Claim, ComputeWeight};
#[cfg(feature = "std")]
use xsupport::token;
use xsupport::{debug, ensure_with_errorlog, warn};

pub use self::types::*;

pub trait Trait:
    xstaking::Trait + xspot::Trait + xbridge_features::Trait + xbitcoin::lockup::Trait
{
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

decl_event!(
    pub enum Event<T> where <T as xassets::Trait>::Balance, <T as system::Trait>::AccountId {
        DepositorReward(AccountId, Token, Balance),
        DepositorClaim(AccountId, Token, u64, u64, Balance),
        DepositorClaimV1(AccountId, Token, u128, u128, Balance),
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
            <Self as Claim<T::AccountId, T::Balance>>::claim(&who, &token)?;
        }

        /// Set the discount for converting the cross-chain asset to PCX based on the market value.
        fn set_token_discount(token: Token, value: u32) {
            ensure!(value <= 100, "TokenDiscount cannot exceed 100.");
            <TokenDiscount<T>>::insert(token, value);
        }

        /// Set the reward for the newly issued cross-chain assets.
        fn set_deposit_reward(value: T::Balance) {
            DepositReward::<T>::put(value);
        }

        fn set_claim_restriction(token: Token, new: (u32, T::BlockNumber)) {
            <ClaimRestrictionOf<T>>::insert(token, new);
        }

        fn set_deposit_record(
            depositor: T::AccountId,
            token: Token,
            new_last_deposit_weight: Option<u64>,
            new_last_deposit_weight_update: Option<T::BlockNumber>
        ) {
            let key = (depositor, token);
            ensure!(
                <DepositRecords<T>>::exists(&key),
                "The DepositVoteWeight must already exist."
            );
            let old = <DepositRecords<T>>::get(&key);
            let last_deposit_weight = new_last_deposit_weight.unwrap_or(old.last_deposit_weight);
            let last_deposit_weight_update =
                new_last_deposit_weight_update.unwrap_or(old.last_deposit_weight_update);
            <DepositRecords<T>>::insert(
                &key,
                DepositVoteWeight {
                    last_deposit_weight,
                    last_deposit_weight_update,
                },
            );
        }

        fn set_deposit_record_v1(
            depositor: T::AccountId,
            token: Token,
            new_last_deposit_weight: Option<u128>,
            new_last_deposit_weight_update: Option<T::BlockNumber>
        ) {
            let key = (depositor, token);
            if let Some(old) = <DepositRecordsV1<T>>::get(&key) {
                let last_deposit_weight = new_last_deposit_weight.unwrap_or(old.last_deposit_weight);
                let last_deposit_weight_update =
                    new_last_deposit_weight_update.unwrap_or(old.last_deposit_weight_update);
                <DepositRecordsV1<T>>::insert(
                    &key,
                    DepositVoteWeightV1 {
                        last_deposit_weight,
                        last_deposit_weight_update,
                    },
                );
            } else {
                return Err("The DepositVoteWeightV1 must already exist.");
            }
        }

        fn set_psedu_intention_profs(
            token: Token,
            new_last_total_deposit_weight: Option<u64>,
            new_last_total_deposit_weight_update: Option<T::BlockNumber>
        ) {
            ensure!(
                <PseduIntentionProfiles<T>>::exists(&token),
                "The PseduIntentionVoteWeight must already exist."
            );
            let old = <PseduIntentionProfiles<T>>::get(&token);
            let last_total_deposit_weight =
                new_last_total_deposit_weight.unwrap_or(old.last_total_deposit_weight);
            let last_total_deposit_weight_update =
                new_last_total_deposit_weight_update.unwrap_or(old.last_total_deposit_weight_update);
            <PseduIntentionProfiles<T>>::insert(
                &token,
                PseduIntentionVoteWeight {
                    last_total_deposit_weight,
                    last_total_deposit_weight_update,
                },
            );
        }

        fn set_psedu_intention_profs_v1(
            token: Token,
            new_last_total_deposit_weight: Option<u128>,
            new_last_total_deposit_weight_update: Option<T::BlockNumber>
        ) {
            if let Some(old) = <PseduIntentionProfilesV1<T>>::get(&token) {
                let last_total_deposit_weight =
                    new_last_total_deposit_weight.unwrap_or(old.last_total_deposit_weight);
                let last_total_deposit_weight_update = new_last_total_deposit_weight_update
                    .unwrap_or(old.last_total_deposit_weight_update);
                <PseduIntentionProfilesV1<T>>::insert(
                    &token,
                    PseduIntentionVoteWeightV1 {
                        last_total_deposit_weight,
                        last_total_deposit_weight_update,
                    },
                );
            } else {
                return Err("The PseduIntentionVoteWeightV1 must already exist.");
            }
        }
    }
}

/// 302_400 blocks per week.
pub const BLOCKS_PER_WEEK: u64 = 60 * 60 * 24 * 7 / 2;

decl_storage! {
    trait Store for Module<T: Trait> as XTokens {
        pub TokenDiscount get(token_discount) build(|config: &GenesisConfig<T>| {
            config.token_discount.clone()
        }): map Token => u32;

        /// Cross-chain assets that are able to participate in the assets mining.
        pub PseduIntentions get(psedu_intentions) : Vec<Token>;

        pub ClaimRestrictionOf get(claim_restriction_of): map Token => (u32, T::BlockNumber) = (10u32, T::BlockNumber::sa(BLOCKS_PER_WEEK));

        /// Block height of last claim for some cross miner per token.
        pub LastClaimOf get(last_claim_of): map (T::AccountId, Token) => Option<T::BlockNumber>;

        pub PseduIntentionProfiles get(psedu_intention_profiles): map Token => PseduIntentionVoteWeight<T::BlockNumber>;

        pub PseduIntentionProfilesV1 get(psedu_intention_profiles_v1): map Token => Option<PseduIntentionVoteWeightV1<T::BlockNumber>>;

        pub DepositRecords get(deposit_records): map (T::AccountId, Token) => DepositVoteWeight<T::BlockNumber>;

        pub DepositRecordsV1 get(deposit_records_v1): map (T::AccountId, Token) => Option<DepositVoteWeightV1<T::BlockNumber>>;

        /// when deposit success, reward some pcx to user for claiming. Default is 100000 = 0.001 PCX; 0.001*100000000
        pub DepositReward get(deposit_reward): T::Balance = As::sa(100_000);
    }

    add_extra_genesis {
        config(token_discount): Vec<(Token, u32)>;
    }
}

impl<T: Trait> Module<T> {
    pub fn last_claim(who: &T::AccountId, token: &Token) -> Option<T::BlockNumber> {
        Self::last_claim_of(&(who.clone(), token.clone()))
    }

    /// This rule doesn't take effect if the interval is zero.
    fn passed_enough_interval(
        who: &T::AccountId,
        token: &Token,
        interval: T::BlockNumber,
        current_block: T::BlockNumber,
    ) -> Result {
        if !interval.is_zero() {
            if let Some(last_claim) = Self::last_claim(who, token) {
                if current_block <= last_claim + interval {
                    warn!("{:?} cannot claim until {:?}", who, last_claim + interval);
                    return Err("Can only claim once per claim limiting period.");
                }
            }
        }
        Ok(())
    }

    /// This rule doesn't take effect if the staking requirement is zero.
    fn contribute_enough_staking(
        who: &T::AccountId,
        dividend: T::Balance,
        staking_requirement: u32,
    ) -> Result {
        if !staking_requirement.is_zero() {
            let staked = <xassets::Module<T>>::pcx_type_balance(who, AssetType::ReservedStaking);
            if staked < T::Balance::sa(u64::from(staking_requirement)) * dividend {
                warn!(
                    "cannot claim due to the insufficient staking, current dividend: {:?}, current staking: {:?}, required staking: {:?}",
                    dividend,
                    staked,
                    T::Balance::sa(u64::from(staking_requirement)) * dividend
                );
                return Err("Cannot claim if what you have staked is too little.");
            }
        }
        Ok(())
    }

    /// Whether the claimer is able to claim the dividend at the given height.
    fn can_claim(
        who: &T::AccountId,
        token: &Token,
        dividend: T::Balance,
        current_block: T::BlockNumber,
    ) -> Result {
        let (staking_requirement, interval) = Self::claim_restriction_of(token);
        Self::contribute_enough_staking(who, dividend, staking_requirement)?;
        Self::passed_enough_interval(who, token, interval, current_block)?;
        Ok(())
    }

    fn deposit_claim_event(
        source_weight_info: (u128, bool),
        target_weight_info: (u128, bool),
        source: &T::AccountId,
        target: &Token,
        dividend: T::Balance,
    ) {
        let (source_vote_weight, source_overflow) = source_weight_info;
        let (target_vote_weight, target_overflow) = target_weight_info;
        if !source_overflow && !target_overflow {
            Self::deposit_event(RawEvent::DepositorClaim(
                source.clone(),
                target.clone(),
                source_vote_weight as u64,
                target_vote_weight as u64,
                dividend,
            ));
        } else {
            Self::deposit_event(RawEvent::DepositorClaimV1(
                source.clone(),
                target.clone(),
                source_vote_weight,
                target_vote_weight,
                dividend,
            ));
        }
    }

    fn try_get_deposit_record(
        key: &(T::AccountId, Token),
    ) -> result::Result<DepositVoteWeight<T::BlockNumber>, DepositVoteWeightV1<T::BlockNumber>>
    {
        if let Some(d1) = <DepositRecordsV1<T>>::get(key) {
            Err(d1)
        } else {
            Ok(<DepositRecords<T>>::get(key))
        }
    }

    fn try_get_psedu_intention_profs(
        target: &Token,
    ) -> result::Result<
        PseduIntentionVoteWeight<T::BlockNumber>,
        PseduIntentionVoteWeightV1<T::BlockNumber>,
    > {
        if let Some(p1) = <PseduIntentionProfilesV1<T>>::get(target) {
            Err(p1)
        } else {
            Ok(<PseduIntentionProfiles<T>>::get(target))
        }
    }

    /// Ensure the vote weight of some depositor or transfer receiver is initialized.
    fn try_init_receiver_vote_weight(
        who: &T::AccountId,
        token: &Token,
        current_block: T::BlockNumber,
    ) {
        let key = (who.clone(), token.clone());
        if !<DepositRecords<T>>::exists(&key) {
            <DepositRecords<T>>::insert(&key, DepositVoteWeight::new(0u64, current_block));
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

    fn update_bare_vote_weight(
        source: &T::AccountId,
        target: &Token,
        current_block: T::BlockNumber,
    ) {
        Self::update_depositor_vote_weight(source, target, current_block);
        Self::update_psedu_intention_vote_weight(target, current_block);
    }

    #[cfg(feature = "std")]
    pub fn bootstrap_update_vote_weight(source: &T::AccountId, target: &Token) {
        let current_block = <system::Module<T>>::block_number();
        Self::try_init_receiver_vote_weight(source, target, current_block);
        Self::update_bare_vote_weight(source, target, current_block);
    }
}

impl<T: Trait> Module<T> {
    pub fn referral_or_council_of(who: &T::AccountId, token: &Token) -> T::AccountId {
        use xbridge_common::traits::CrossChainBindingV2;

        // Get referral from xbridge_common since v1.0.3.
        let referral = if <xsdot::Module<T> as ChainT>::TOKEN == token.as_slice()
            || <xbitcoin::Module<T> as ChainT>::TOKEN == token.as_slice()
        {
            if let Some(asset_info) = <xassets::AssetInfo<T>>::get(token) {
                let asset = asset_info.0;
                let chain = asset.chain();
                xbridge_features::Module::<T>::get_first_binding_channel(who, chain)
            } else {
                None
            }
        } else {
            xbridge_common::Module::<T>::get_binding_info(token, who)
        };

        referral.unwrap_or_else(xaccounts::Module::<T>::council_account)
    }

    pub fn token_jackpot_accountid_for_unsafe(token: &Token) -> T::AccountId {
        T::DetermineTokenJackpotAccountId::accountid_for_unsafe(token)
    }

    pub fn multi_token_jackpot_accountid_for_unsafe(tokens: &[Token]) -> Vec<T::AccountId> {
        tokens
            .iter()
            .map(T::DetermineTokenJackpotAccountId::accountid_for_unsafe)
            .collect()
    }
}
