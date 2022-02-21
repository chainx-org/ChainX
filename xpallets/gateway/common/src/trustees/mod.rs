// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

pub mod bitcoin;

use frame_support::{
    dispatch::DispatchError,
    log::{error, warn},
};
use sp_runtime::{
    traits::{CheckedDiv, Saturating, Zero},
    SaturatedConversion,
};
use sp_std::{convert::TryFrom, marker::PhantomData, prelude::*};

use xp_assets_registrar::Chain;
use xp_protocol::X_BTC;
use xpallet_assets::BalanceOf;

use crate::{
    traits::{BytesLike, ChainProvider, TrusteeInfoUpdate, TrusteeSession},
    types::TrusteeSessionInfo,
    Config, Error, Event, Pallet, TrusteeSessionInfoOf, TrusteeSigRecord, TrusteeTransitionStatus,
};

pub struct TrusteeSessionManager<T: Config, TrusteeAddress>(
    PhantomData<T>,
    PhantomData<TrusteeAddress>,
);

impl<T: Config, TrusteeAddress: BytesLike + ChainProvider>
    TrusteeSession<T::AccountId, T::BlockNumber, TrusteeAddress>
    for TrusteeSessionManager<T, TrusteeAddress>
{
    fn trustee_session(
        number: u32,
    ) -> Result<TrusteeSessionInfo<T::AccountId, T::BlockNumber, TrusteeAddress>, DispatchError>
    {
        let chain = TrusteeAddress::chain();
        let generic_info =
            Pallet::<T>::trustee_session_info_of(chain, number).ok_or_else(|| {
                error!(
                    target: "runtime::gateway::common",
                    "[trustee_session] Can not find session info, chain:{:?}, number:{}",
                    chain,
                    number
                );
                Error::<T>::InvalidTrusteeSession
            })?;
        let info = TrusteeSessionInfo::<T::AccountId, T::BlockNumber, TrusteeAddress>::try_from(
            generic_info,
        )
        .map_err(|_| Error::<T>::InvalidGenericData)?;
        Ok(info)
    }

    fn current_trustee_session(
    ) -> Result<TrusteeSessionInfo<T::AccountId, T::BlockNumber, TrusteeAddress>, DispatchError>
    {
        let chain = TrusteeAddress::chain();
        let number = Pallet::<T>::trustee_session_info_len(chain);
        Self::trustee_session(number)
    }

    fn current_proxy_account() -> Result<Vec<T::AccountId>, DispatchError> {
        Ok(Self::current_trustee_session()?
            .trustee_list
            .iter()
            .filter_map(|info| {
                match Pallet::<T>::trustee_intention_props_of(&info.0, TrusteeAddress::chain()) {
                    None => None,
                    Some(n) => n.0.proxy_account,
                }
            })
            .collect::<Vec<T::AccountId>>())
    }

    fn last_trustee_session(
    ) -> Result<TrusteeSessionInfo<T::AccountId, T::BlockNumber, TrusteeAddress>, DispatchError>
    {
        let chain = TrusteeAddress::chain();
        let number = match Pallet::<T>::trustee_session_info_len(chain).checked_sub(1) {
            Some(r) => r,
            None => u32::MAX,
        };
        Self::trustee_session(number).map_err(|err| {
            warn!(
                target: "runtime::gateway::common",
                "[last_trustee_session] Last trustee session not exist yet for chain:{:?}",
                chain
            );
            err
        })
    }

    fn trustee_transition_state() -> bool {
        Pallet::<T>::trustee_transition_status(TrusteeAddress::chain())
    }

    #[cfg(feature = "std")]
    fn genesis_trustee(chain: Chain, trustees: &[T::AccountId]) {
        Pallet::<T>::transition_trustee_session_impl(chain, trustees.to_vec())
            .expect("trustee session transition can not fail; qed");
    }
}

impl<T: Config> TrusteeInfoUpdate for Pallet<T> {
    fn update_transition_status(chain: Chain, status: bool, trans_amount: Option<u64>) {
        // The renewal of the trustee is completed, the current trustee information is replaced
        // and the number of multiple signings is archived. Currently only supports bitcoin
        if chain == Chain::Bitcoin && Self::trustee_transition_status(chain) && !status {
            let last_session_num = Self::trustee_session_info_len(chain).saturating_sub(1);
            TrusteeSessionInfoOf::<T>::mutate(chain, last_session_num, |info| match info {
                None => {
                    warn!(
                        target: "runtime::gateway::common",
                        "[last_trustee_session] Last trustee session not exist for chain:{:?}, session_num:{}",
                        chain, last_session_num
                    );
                }
                Some(trustee) => {
                    for i in 0..trustee.0.trustee_list.len() {
                        trustee.0.trustee_list[i].1 =
                            Self::trustee_sig_record(chain, &trustee.0.trustee_list[i].0);
                    }
                    let total_apply = Self::pre_total_supply(chain, last_session_num);

                    let reward_amount = trans_amount
                        .unwrap_or(0u64)
                        .saturated_into::<BalanceOf<T>>()
                        .saturating_sub(total_apply)
                        .max(0u64.saturated_into())
                        .saturating_mul(6u64.saturated_into())
                        .checked_div(&10u64.saturated_into::<BalanceOf<T>>())
                        .unwrap_or_else(|| 0u64.saturated_into());

                    if let Some(multi_account) = trustee.0.multi_account.clone() {
                        if !reward_amount.is_zero() {
                            match xpallet_assets::Pallet::<T>::issue(
                                &X_BTC,
                                &multi_account,
                                reward_amount,
                            ) {
                                Ok(()) => {
                                    Pallet::<T>::deposit_event(Event::<T>::TransferAssetReward(
                                        multi_account,
                                        X_BTC,
                                        reward_amount,
                                    ));
                                }
                                Err(err) => {
                                    warn!(
                                        target: "runtime::gateway::common",
                                        "[issue] Issue fail:{:?}!",
                                        err
                                    );
                                }
                            };
                        }
                    }
                    let end_height = frame_system::Pallet::<T>::block_number();
                    trustee.0.end_height = Some(end_height);
                }
            });
            TrusteeSigRecord::<T>::remove_prefix(chain, None);
        }

        TrusteeTransitionStatus::<T>::insert(chain, status);
    }

    fn update_trustee_sig_record(chain: Chain, script: &[u8], withdraw_amount: u64) {
        let signed_trustees = Self::agg_pubkey_info(chain, script);
        signed_trustees.into_iter().for_each(|trustee| {
            let amount = if trustee == Self::trustee_admin(chain) {
                withdraw_amount
                    .saturating_mul(Self::trustee_admin_multiply(chain))
                    .checked_div(10)
                    .unwrap_or(0)
            } else {
                withdraw_amount
            };
            if TrusteeSigRecord::<T>::contains_key(chain, &trustee) {
                TrusteeSigRecord::<T>::mutate(chain, &trustee, |record| *record += amount);
            } else {
                TrusteeSigRecord::<T>::insert(chain, trustee, amount);
            }
        });
    }
}
