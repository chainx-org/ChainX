// Copyright 2018-2019 Chainpool.

use super::*;

use std::iter::FromIterator;

use log::error;
use rustc_hex::ToHex;

use serde_json::{json, Map, Value};

use btc_keys::DisplayLayout;

// chainx
use chainx_primitives::AccountIdForRpc;
use xr_primitives::generic::b58;

use xbridge_common::{
    traits::IntoVecu8,
    types::{GenericAllSessionInfo, GenericTrusteeIntentionProps},
};
use xbridge_features::trustees::{BitcoinPublic, BitcoinTrusteeAddrInfo};

/// Convert &[u8] to String
macro_rules! to_string {
    ($str:expr) => {
        String::from_utf8_lossy($str).into_owned()
    };
}

pub fn try_hex_or_str(src: &[u8]) -> String {
    let check_is_str = |src: &[u8]| -> bool {
        for c in src {
            if 0x21 <= *c && *c <= 0x7E {
                continue;
            } else {
                return false;
            }
        }
        true
    };
    if check_is_str(src) {
        to_string!(src)
    } else {
        format!("0x{:}", src.to_hex::<String>())
    }
}

fn parse_generic_trustee_props(
    chain: Chain,
    props: &GenericTrusteeIntentionProps,
) -> Option<Value> {
    let result = match chain {
        Chain::Bitcoin => {
            let hot_public = BitcoinPublic::from_vecu8(props.0.hot_entity.as_slice());
            let cold_public = BitcoinPublic::from_vecu8(props.0.cold_entity.as_slice());
            if hot_public.is_none() || cold_public.is_none() {
                error!(
                    "parse_generic_trustee_props for bitcoin error|hot_entity:{:}|cold_entity:{:}",
                    try_hex_or_str(&props.0.hot_entity),
                    try_hex_or_str(&props.0.cold_entity)
                );
                return None;
            } else {
                let format_public = |public: &BitcoinPublic| -> Option<String> {
                    match public {
                        BitcoinPublic::Normal(_) => {
                            error!("bitcoin TrusteeIntentionProps entity should be `Compressed`, not `Normal`, something wrong in chain!|public:{:?}", public);
                            None
                        }
                        BitcoinPublic::Compressed(ref hash) => Some(format!("{:?}", hash)),
                    }
                };
                json!({
                    "about": to_string!(&props.0.about),
                    "hotEntity": format_public(&hot_public.unwrap()),
                    "coldEntity": format_public(&cold_public.unwrap()),
                })
            }
        }
        // TODO when add other trustee, must add related parse here
        _ => unimplemented!("not support for other chain"),
    };
    Some(result)
}

pub fn parse_trustee_props(map: BTreeMap<Chain, GenericTrusteeIntentionProps>) -> Option<Value> {
    let map = Map::from_iter(map.into_iter().map(|(chain, generic_props)| {
        (
            format!("{:?}", chain),
            parse_generic_trustee_props(chain, &generic_props).unwrap_or(Value::Null),
        )
    }));
    Some(Value::Object(map))
}

pub fn parse_trustee_session_addr(chain: Chain, addr: &[u8]) -> Option<Value> {
    let result = match chain {
        Chain::Bitcoin => {
            let trustee_addr_info = BitcoinTrusteeAddrInfo::from_vecu8(addr);
            let trustee_addr_info = if trustee_addr_info.is_none() {
                return None;
            } else {
                trustee_addr_info.unwrap()
            };

            let address = to_string!(&b58::to_base58(trustee_addr_info.addr.layout().to_vec()));
            json!({
                "addr": address,
                "redeemScript": try_hex_or_str(&trustee_addr_info.redeem_script)
            })
        }
        // TODO when add other trustee, must add related parse here
        _ => unimplemented!("not support for other chain"),
    };
    Some(result)
}

pub fn parse_trustee_session_info(
    chain: Chain,
    number: u32,
    info: GenericAllSessionInfo<AccountId>,
) -> Option<Value> {
    let hot = parse_trustee_session_addr(chain, &info.hot_entity);
    let cold = parse_trustee_session_addr(chain, &info.cold_entity);
    Some(json!({
        "sessionNumber": number,
        "hotEntity": hot,
        "coldEntity": cold,
        "counts": info.counts,
        "trusteeList": info.trustees_info.into_iter().map(|(accountid, generic_props)| {
            let accountid :AccountIdForRpc= accountid.into();
            json!({
                "accountId": accountid,
                "props": parse_generic_trustee_props(chain, &generic_props),
            })
        }).collect::<Vec<_>>()
    }))
}

pub fn calculate_staking_dividend(
    record_v1: &xstaking::NominationRecordV1<Balance, BlockNumber>,
    intention_profs_v1: &xstaking::IntentionProfsV1<Balance, BlockNumber>,
    jackpot_balance: Balance,
    current_block: BlockNumber,
) -> Balance {
    let target_latest_vote_weight =
        intention_profs_v1.settle_latest_vote_weight_safe(current_block);

    if target_latest_vote_weight == 0 {
        return 0;
    }

    let source_latest_vote_weight = record_v1.settle_latest_vote_weight_safe(current_block);

    let dividend =
        source_latest_vote_weight * u128::from(jackpot_balance) / target_latest_vote_weight;

    dividend as u64
}

pub fn calculate_cross_mining_dividend(
    d1: xtokens::DepositVoteWeightV1<Balance>,
    p1: xtokens::PseduIntentionVoteWeightV1<Balance>,
    jackpot_balance: Balance,
    current_block: BlockNumber,
    total_token_balance: Balance,
    miner_balance: Balance,
) -> (Balance, Balance) {
    if current_block < p1.last_total_deposit_weight_update {
        return (0, 0);
    }

    let duration = current_block - p1.last_total_deposit_weight_update;
    let target_latest_vote_weight =
        u128::from(total_token_balance) * u128::from(duration) + p1.last_total_deposit_weight;

    if target_latest_vote_weight == 0 || current_block < d1.last_deposit_weight_update {
        return (0, 0);
    }

    let duration = current_block - d1.last_deposit_weight_update;
    let source_latest_vote_weight =
        u128::from(miner_balance) * u128::from(duration) + d1.last_deposit_weight;

    let dividend = (source_latest_vote_weight * u128::from(jackpot_balance)
        / target_latest_vote_weight) as u64;

    let for_referral = dividend / 10;

    (for_referral, dividend - for_referral)
}
