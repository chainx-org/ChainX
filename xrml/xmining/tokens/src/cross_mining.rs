// Copyright 2018-2019 Chainpool.
use super::*;
use xstaking::{OnReward, OnRewardCalculation, RewardHolder};

impl<T: Trait> Module<T> {
    fn one_pcx() -> u64 {
        let pcx = <xassets::Module<T> as ChainT>::TOKEN.to_vec();
        let pcx_asset = <xassets::Module<T>>::get_asset(&pcx).expect("PCX definitely exist.");

        10_u64.pow(pcx_asset.precision().into())
    }

    /// This calculation doesn't take the DistributionRatio of cross-chain assets and native assets into account.
    pub fn internal_cross_chain_asset_power(token: &Token) -> Option<T::Balance> {
        let discount = u64::from(<TokenDiscount<T>>::get(token));

        // One SDOT 0.1 vote.
        if <xsdot::Module<T> as ChainT>::TOKEN == token.as_slice() {
            return Some((Self::one_pcx() * discount / 100).into());
        } else {
            // L-BTC shares the price of X-BTC as it doesn't have a trading pair.
            let token = if <xbitcoin::lockup::Module<T> as ChainT>::TOKEN == token.as_slice() {
                <xbitcoin::Module<T> as ChainT>::TOKEN.to_vec()
            } else {
                token.clone()
            };

            if let Some(price) = <xspot::Module<T>>::aver_asset_price(&token) {
                let power = match (u128::from(price.into())).checked_mul(u128::from(discount)) {
                    Some(x) => ((x / 100) as u64).into(),
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
            return Some(Self::one_pcx().into());
        }

        Self::internal_cross_chain_asset_power(token)
    }

    pub fn asset_power(token: &Token) -> Option<T::Balance> {
        let power = Self::internal_asset_power(token);

        if <xassets::Module<T> as ChainT>::TOKEN != token.as_slice() {
            if let Ok((num, denom)) =
                <xstaking::Module<T>>::cross_chain_assets_are_growing_too_fast()
            {
                let double_discounted = power.map(|p| u128::from(p.into()) * num / denom);
                debug!(
                    "[asset_power] should reduce the power again: original power: {:?}, double discount: {:?}/{:?} => final power: {:?}",
                    power,
                    num, denom,
                    double_discounted
                );
                return double_discounted.map(|p| (p as u64).into());
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
                    <xassets::Module<T>>::all_type_total_asset_balance(&token).into();

                let total = match (u128::from(total_balance)).checked_mul(u128::from(power.into()))
                {
                    Some(x) => ((x / pow_precision) as u64).into(),
                    None => panic!("total_balance * price overflow"),
                };

                return Some(total);
            }
        }

        None
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
