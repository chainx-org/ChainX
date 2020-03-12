// Copyright 2018-2019 Chainpool.

use super::*;
use xstaking::{OnDistributeAirdropAsset, OnDistributeCrossChainAsset, OnReward};

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

impl<T: Trait> OnReward<T::AccountId, T::Balance> for Module<T> {
    fn reward(token: &Token, value: T::Balance) {
        debug!(
            "[OnReward::reward]token: {:?}, value:{}",
            xsupport::u8array_to_string(&token),
            value
        );
        let addr = T::DetermineTokenJackpotAccountId::accountid_for_unsafe(token);
        let _ = xassets::Module::<T>::pcx_issue(&addr, value);
    }
}

impl<T: Trait> OnDistributeAirdropAsset for Module<T> {
    fn collect_airdrop_assets_info() -> Vec<(Token, u32)> {
        <AirdropDistributionRatioMap<T>>::enumerate().collect::<Vec<_>>()
    }
}

impl<T: Trait> OnDistributeCrossChainAsset for Module<T> {
    fn collect_cross_chain_assets_info() -> Vec<(Token, u128)> {
        <FixedCrossChainAssetPowerMap<T>>::enumerate()
            .map(|(token, power)| {
                let total_balance =
                    <xassets::Module<T>>::all_type_total_asset_balance(&token).into();
                (token, u128::from(total_balance) * u128::from(power))
            })
            .collect::<Vec<_>>()
    }
}
