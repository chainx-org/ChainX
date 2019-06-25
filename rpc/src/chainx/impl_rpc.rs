// Copyright 2018-2019 Chainpool.

use serde_json::{json, Value};

use std::collections::btree_map::BTreeMap;
use std::convert::Into;
use std::iter::FromIterator;
use std::result;

use parity_codec::{Decode, Encode};
use rustc_hex::FromHex;
// substrate
use client::runtime_api::Metadata;
use primitives::crypto::UncheckedInto;
use primitives::{Blake2Hasher, H160, H256};
use runtime_primitives::generic::SignedBlock;
use runtime_primitives::traits::{Block as BlockT, NumberFor, ProvideRuntimeApi, Zero};
use support::storage::{StorageMap, StorageValue};
// chainx
use chainx_primitives::{AccountId, AccountIdForRpc, AuthorityId, Balance, BlockNumber};
use chainx_runtime::{Call, Runtime};
use xr_primitives::{AddrStr, Name};

use xaccounts::IntentionProps;
use xassets::{AssetType, Chain, ChainT, Token};
use xbridge_common::types::GenericAllSessionInfo;
use xbridge_features::{
    self,
    crosschain_binding::{BitcoinAddress, EthereumAddress},
};
use xprocess::WithdrawalLimit;
use xspot::{HandicapInfo, OrderIndex, OrderInfo, TradingPair, TradingPairIndex};
use xstaking::IntentionProfs;
use xtokens::{DepositVoteWeight, PseduIntentionVoteWeight};

use runtime_api::{
    xassets_api::XAssetsApi, xbridge_api::XBridgeApi, xfee_api::XFeeApi, xmining_api::XMiningApi,
    xspot_api::XSpotApi, xstaking_api::XStakingApi,
};

use super::error::{ErrorKind, Result};
use super::types::*;
use super::{ChainX, ChainXApi};

impl<B, E, Block, RA>
    ChainXApi<
        NumberFor<Block>,
        <Block as BlockT>::Hash,
        AccountIdForRpc,
        Balance,
        BlockNumber,
        SignedBlock<Block>,
    > for ChainX<B, E, Block, RA>
where
    B: client::backend::Backend<Block, Blake2Hasher> + Send + Sync + 'static,
    E: client::CallExecutor<Block, Blake2Hasher> + Clone + Send + Sync + 'static,
    Block: BlockT<Hash = H256> + 'static,
    RA: Send + Sync + 'static,
    client::Client<B, E, Block, RA>: ProvideRuntimeApi,
    <client::Client<B, E, Block, RA> as ProvideRuntimeApi>::Api: Metadata<Block>
        + XAssetsApi<Block>
        + XMiningApi<Block>
        + XSpotApi<Block>
        + XFeeApi<Block>
        + XStakingApi<Block>
        + XBridgeApi<Block>,
{
    fn block_info(&self, number: Option<NumberFor<Block>>) -> Result<Option<SignedBlock<Block>>> {
        Ok(self.client.block(&self.block_id_by_number(number)?)?)
    }

    fn assets_of(
        &self,
        who: AccountIdForRpc,
        page_index: u32,
        page_size: u32,
        hash: Option<<Block as BlockT>::Hash>,
    ) -> Result<Option<PageData<AssetInfo>>> {
        let assets = self.valid_assets_of(self.block_id_by_hash(hash)?, who.unchecked_into())?;
        let final_result = assets
            .into_iter()
            .map(|(token, map)| {
                let mut bmap = BTreeMap::<AssetType, Balance>::from_iter(
                    xassets::AssetType::iterator().map(|t| (*t, Zero::zero())),
                );
                bmap.extend(map.iter());
                AssetInfo {
                    name: String::from_utf8_lossy(&token).into_owned(),
                    details: bmap,
                }
            })
            .collect();
        into_pagedata(final_result, page_index, page_size)
    }

    fn assets(
        &self,
        page_index: u32,
        page_size: u32,
        hash: Option<<Block as BlockT>::Hash>,
    ) -> Result<Option<PageData<TotalAssetInfo>>> {
        let assets = self.all_assets(self.block_id_by_hash(hash)?)?;

        let state = self.state_at(hash)?;

        let mut all_assets = Vec::new();

        for (asset, valid) in assets.into_iter() {
            let mut bmap = BTreeMap::<AssetType, Balance>::from_iter(
                xassets::AssetType::iterator().map(|t| (*t, Zero::zero())),
            );

            let key = <xassets::TotalAssetBalance<Runtime>>::key_for(asset.token().as_ref());
            if let Some(info) =
                Self::pickout::<BTreeMap<AssetType, Balance>>(&state, &key, Hasher::BLAKE2256)?
            {
                bmap.extend(info.iter());
            }

            all_assets.push(TotalAssetInfo::new(asset, valid, bmap));
        }

        into_pagedata(all_assets, page_index, page_size)
    }

    fn verify_addr(
        &self,
        token: String,
        addr: String,
        memo: String,
        hash: Option<<Block as BlockT>::Hash>,
    ) -> Result<Option<bool>> {
        let token: xassets::Token = token.as_bytes().to_vec();
        let addr: AddrStr = addr.as_bytes().to_vec();
        let memo: xassets::Memo = memo.as_bytes().to_vec();

        if let Err(_e) = xassets::is_valid_token(&token) {
            return Ok(Some(false));
        }

        if addr.len() > 256 || memo.len() > 256 {
            return Ok(Some(false));
        }

        let ret = self
            .client
            .runtime_api()
            .verify_address(&self.block_id_by_hash(hash)?, token, addr, memo)
            .and_then(|r| match r {
                Ok(()) => Ok(None),
                Err(s) => Ok(Some(String::from_utf8_lossy(s.as_ref()).into_owned())),
            });
        match ret {
            Err(_) => Ok(Some(false)),
            Ok(ret) => match ret {
                None => Ok(Some(true)),
                Some(_) => Ok(Some(false)),
            },
        }
    }

    fn withdrawal_limit(
        &self,
        token: String,
        hash: Option<<Block as BlockT>::Hash>,
    ) -> Result<Option<WithdrawalLimit<Balance>>> {
        let token: xassets::Token = token.as_bytes().to_vec();

        if xassets::is_valid_token(&token).is_err() {
            return Ok(None);
        }
        self.withdrawal_limit(self.block_id_by_hash(hash)?, token)
    }

    fn deposit_list(
        &self,
        chain: Chain,
        page_index: u32,
        page_size: u32,
        hash: Option<<Block as BlockT>::Hash>,
    ) -> Result<Option<PageData<DepositInfo>>> {
        let list = self
            .deposit_list_of(self.block_id_by_hash(hash)?, chain)
            .unwrap_or_default();

        // convert recordinfo to deposit
        let records: Vec<DepositInfo> = list.into_iter().map(Into::into).collect();
        into_pagedata(records, page_index, page_size)
    }

    fn withdrawal_list(
        &self,
        chain: Chain,
        page_index: u32,
        page_size: u32,
        hash: Option<<Block as BlockT>::Hash>,
    ) -> Result<Option<PageData<WithdrawInfo>>> {
        let list = self
            .withdrawal_list_of(self.block_id_by_hash(hash)?, chain)
            .unwrap_or_default();
        let records: Vec<WithdrawInfo> = list.into_iter().map(Into::into).collect();
        into_pagedata(records, page_index, page_size)
    }

    fn nomination_records(
        &self,
        who: AccountIdForRpc,
        hash: Option<<Block as BlockT>::Hash>,
    ) -> Result<Option<Vec<(AccountIdForRpc, NominationRecord)>>> {
        let state = self.state_at(hash)?;

        let mut records = Vec::new();

        let intentions = self.intention_set(self.block_id_by_hash(hash)?)?;

        for intention in intentions {
            let key = <xstaking::NominationRecords<Runtime>>::key_for(&(
                who.clone().unchecked_into(),
                intention.clone(),
            ));
            if let Some(record) = Self::pickout::<xstaking::NominationRecord<Balance, BlockNumber>>(
                &state,
                &key,
                Hasher::BLAKE2256,
            )? {
                let revocations = record
                    .revocations
                    .iter()
                    .map(|x| Revocation {
                        block_number: x.0,
                        value: x.1,
                    })
                    .collect::<Vec<_>>();
                records.push((
                    intention.into(),
                    NominationRecord {
                        nomination: record.nomination,
                        last_vote_weight: record.last_vote_weight,
                        last_vote_weight_update: record.last_vote_weight_update,
                        revocations,
                    },
                ));
            }
        }

        Ok(Some(records))
    }

    fn intention(
        &self,
        who: AccountIdForRpc,
        hash: Option<<Block as BlockT>::Hash>,
    ) -> Result<Option<Value>> {
        let state = self.state_at(hash)?;
        let who: AccountId = who.unchecked_into();
        let key = <xaccounts::IntentionPropertiesOf<Runtime>>::key_for(&who);
        let session_key: AccountIdForRpc = if let Some(props) = Self::pickout::<
            IntentionProps<AuthorityId, BlockNumber>,
        >(
            &state, &key, Hasher::BLAKE2256
        )? {
            props.session_key.unwrap_or(who.clone()).into()
        } else {
            return Ok(None);
        };

        let jackpot_account =
            self.jackpot_accountid_for_unsafe(self.block_id_by_hash(hash)?, who.clone())?;
        Ok(Some(json!({
            "sessionKey": session_key,
            "jackpotAccount": jackpot_account,
        })))
    }

    fn intentions(
        &self,
        hash: Option<<Block as BlockT>::Hash>,
    ) -> Result<Option<Vec<IntentionInfo>>> {
        let state = self.state_at(hash)?;

        let mut intention_info = Vec::new();

        let key = <xsession::Validators<Runtime>>::key();
        let validators = Self::pickout::<Vec<(AccountId, u64)>>(&state, &key, Hasher::TWOX128)?
            .expect("Validators can't be empty");
        let validators: Vec<AccountId> = validators.into_iter().map(|(who, _)| who).collect();

        let block_id = self.block_id_by_hash(hash)?;

        // get all bridge trustee list
        let all_session_info = self.trustee_session_info(block_id)?;
        let all_trustees = all_session_info
            .into_iter()
            .map(|(chain, info)| {
                (
                    chain,
                    info.trustees_info
                        .into_iter()
                        .map(|(accountid, _)| accountid)
                        .collect::<Vec<_>>(),
                )
            })
            .collect::<BTreeMap<_, _>>();

        let is_trustee = |who: &AccountId| -> Vec<Chain> {
            let mut ret = vec![];
            for (chain, trustees) in all_trustees.iter() {
                if trustees.contains(who) {
                    ret.push(*chain);
                }
            }
            ret
        };

        let intentions = self.intention_set(block_id)?;
        let jackpot_account_list =
            self.multi_jackpot_accountid_for_unsafe(block_id, intentions.clone())?;

        for (intention, jackpot_account) in intentions.into_iter().zip(jackpot_account_list) {
            let mut info = IntentionInfo::default();

            let key = <xaccounts::IntentionNameOf<Runtime>>::key_for(&intention);
            if let Some(name) = Self::pickout::<Name>(&state, &key, Hasher::BLAKE2256)? {
                info.name = String::from_utf8_lossy(&name).into_owned();
            }

            let key = <xaccounts::IntentionPropertiesOf<Runtime>>::key_for(&intention);
            if let Some(props) = Self::pickout::<IntentionProps<AuthorityId, BlockNumber>>(
                &state,
                &key,
                Hasher::BLAKE2256,
            )? {
                info.url = String::from_utf8_lossy(&props.url).into_owned();
                info.is_active = props.is_active;
                info.about = String::from_utf8_lossy(&props.about).into_owned();
                info.session_key = props.session_key.unwrap_or(intention.clone()).into();
            }

            let key = <xstaking::Intentions<Runtime>>::key_for(&intention);
            if let Some(profs) = Self::pickout::<IntentionProfs<Balance, BlockNumber>>(
                &state,
                &key,
                Hasher::BLAKE2256,
            )? {
                let key = (
                    jackpot_account.clone(),
                    xassets::Module::<Runtime>::TOKEN.to_vec(),
                );
                let balances_key = <xassets::AssetBalance<Runtime>>::key_for(&key);
                let map = Self::pickout::<BTreeMap<AssetType, Balance>>(
                    &state,
                    &balances_key,
                    Hasher::BLAKE2256,
                )?
                .unwrap_or_default();
                let free = map
                    .get(&AssetType::Free)
                    .map(|free| *free)
                    .unwrap_or_default();
                info.jackpot = free;
                info.jackpot_account = jackpot_account.into();
                info.total_nomination = profs.total_nomination;
                info.last_total_vote_weight = profs.last_total_vote_weight;
                info.last_total_vote_weight_update = profs.last_total_vote_weight_update;
            }

            let key = <xstaking::NominationRecords<Runtime>>::key_for(&(
                intention.clone(),
                intention.clone(),
            ));
            if let Some(record) = Self::pickout::<xstaking::NominationRecord<Balance, BlockNumber>>(
                &state,
                &key,
                Hasher::BLAKE2256,
            )? {
                info.self_vote = record.nomination;
            }

            info.is_validator = validators.iter().any(|i| i == &intention);
            info.is_trustee = is_trustee(&intention);
            info.account = intention.into();

            intention_info.push(info);
        }

        Ok(Some(intention_info))
    }

    fn psedu_intentions(
        &self,
        hash: Option<<Block as BlockT>::Hash>,
    ) -> Result<Option<Vec<PseduIntentionInfo>>> {
        let block_id = self.block_id_by_hash(hash)?;
        let state = self.state_at(hash)?;

        let mut psedu_intentions = Vec::new();

        let key = <xtokens::PseduIntentions<Runtime>>::key();
        if let Some(tokens) = Self::pickout::<Vec<Token>>(&state, &key, Hasher::TWOX128)? {
            let jackpot_account_list =
                self.multi_token_jackpot_accountid_for_unsafe(block_id, tokens.clone())?;

            for (token, jackpot_account) in tokens.into_iter().zip(jackpot_account_list) {
                let mut info = PseduIntentionInfo::default();

                let key = <xtokens::PseduIntentionProfiles<Runtime>>::key_for(&token);
                if let Some(vote_weight) = Self::pickout::<PseduIntentionVoteWeight<Balance>>(
                    &state,
                    &key,
                    Hasher::BLAKE2256,
                )? {
                    let key = (
                        jackpot_account.clone(),
                        xassets::Module::<Runtime>::TOKEN.to_vec(),
                    );
                    let balances_key = <xassets::AssetBalance<Runtime>>::key_for(&key);
                    let map = Self::pickout::<BTreeMap<AssetType, Balance>>(
                        &state,
                        &balances_key,
                        Hasher::BLAKE2256,
                    )?
                    .unwrap_or_default();
                    let free = map
                        .get(&AssetType::Free)
                        .map(|free| *free)
                        .unwrap_or_default();
                    info.jackpot = free;
                    info.jackpot_account = jackpot_account.into();
                    info.last_total_deposit_weight = vote_weight.last_total_deposit_weight;
                    info.last_total_deposit_weight_update =
                        vote_weight.last_total_deposit_weight_update;
                }

                let key = <xtokens::TokenDiscount<Runtime>>::key_for(&token);
                if let Some(discount) = Self::pickout::<u32>(&state, &key, Hasher::BLAKE2256)? {
                    info.discount = discount;
                }

                //注意
                //这里返回的是以PCX计价的"单位"token的价格，已含pcx精度
                //譬如1BTC=10000PCX，则返回的是10000*（10.pow(pcx精度))
                //因此，如果前端要换算折合投票数的时候
                //应该=(资产数量[含精度的数字]*price)/(10^资产精度)=PCX[含PCX精度]

                if let Ok(Some(price)) = self.aver_asset_price(block_id, token.clone()) {
                    info.price = price;
                };

                if let Ok(Some(power)) = self.asset_power(block_id, token.clone()) {
                    info.power = power;
                };

                let key = <xassets::TotalAssetBalance<Runtime>>::key_for(&token);
                if let Some(total_asset_balance) =
                    Self::pickout::<BTreeMap<AssetType, Balance>>(&state, &key, Hasher::BLAKE2256)?
                {
                    info.circulation = total_asset_balance
                        .iter()
                        .fold(Zero::zero(), |acc, (_, v)| acc + *v);
                }

                info.id = String::from_utf8_lossy(&token).into_owned();
                psedu_intentions.push(info);
            }
        }

        Ok(Some(psedu_intentions))
    }

    fn psedu_nomination_records(
        &self,
        who: AccountIdForRpc,
        hash: Option<<Block as BlockT>::Hash>,
    ) -> Result<Option<Vec<PseduNominationRecord>>> {
        let state = self.state_at(hash)?;
        let mut psedu_records = Vec::new();

        let key = <xtokens::PseduIntentions<Runtime>>::key();
        if let Some(tokens) = Self::pickout::<Vec<Token>>(&state, &key, Hasher::TWOX128)? {
            for token in tokens {
                let mut record = PseduNominationRecord::default();

                let key = <xtokens::DepositRecords<Runtime>>::key_for(&(
                    who.clone().unchecked_into(),
                    token.clone(),
                ));
                if let Some(vote_weight) = Self::pickout::<DepositVoteWeight<BlockNumber>>(
                    &state,
                    &key,
                    Hasher::BLAKE2256,
                )? {
                    record.last_total_deposit_weight = vote_weight.last_deposit_weight;
                    record.last_total_deposit_weight_update =
                        vote_weight.last_deposit_weight_update;
                }

                let key = <xassets::AssetBalance<Runtime>>::key_for(&(
                    who.clone().unchecked_into(),
                    token.clone(),
                ));

                if let Some(balances) =
                    Self::pickout::<BTreeMap<AssetType, Balance>>(&state, &key, Hasher::BLAKE2256)?
                {
                    record.balance = balances.iter().fold(Zero::zero(), |acc, (_, v)| acc + *v);
                }

                record.id = String::from_utf8_lossy(&token).into_owned();

                psedu_records.push(record);
            }
        }

        Ok(Some(psedu_records))
    }

    fn trading_pairs(
        &self,
        hash: Option<<Block as BlockT>::Hash>,
    ) -> Result<Option<Vec<(PairInfo)>>> {
        let mut pairs = Vec::new();
        let state = self.state_at(hash)?;

        let len_key = <xspot::TradingPairCount<Runtime>>::key();
        if let Some(len) = Self::pickout::<TradingPairIndex>(&state, &len_key, Hasher::TWOX128)? {
            for i in 0..len {
                let key = <xspot::TradingPairOf<Runtime>>::key_for(&i);
                if let Some(pair) = Self::pickout::<TradingPair>(&state, &key, Hasher::BLAKE2256)? {
                    let mut info = PairInfo::default();
                    info.id = pair.index;
                    info.assets = String::from_utf8_lossy(pair.base_as_ref()).into_owned();
                    info.currency = String::from_utf8_lossy(pair.quote_as_ref()).into_owned();
                    info.precision = pair.pip_precision;
                    info.online = pair.online;
                    info.unit_precision = pair.tick_precision;

                    let price_key = <xspot::TradingPairInfoOf<Runtime>>::key_for(&i);
                    if let Some(price) = Self::pickout::<(Balance, Balance, BlockNumber)>(
                        &state,
                        &price_key,
                        Hasher::BLAKE2256,
                    )? {
                        info.last_price = price.0;
                        info.aver_price = price.1;
                        info.update_height = price.2;
                    }

                    let handicap_key = <xspot::HandicapOf<Runtime>>::key_for(&i);
                    if let Some(handicap) = Self::pickout::<HandicapInfo<Runtime>>(
                        &state,
                        &handicap_key,
                        Hasher::BLAKE2256,
                    )? {
                        info.buy_one = handicap.highest_bid;
                        info.sell_one = handicap.lowest_offer;

                        if !handicap.lowest_offer.is_zero() {
                            info.maximum_bid = handicap.lowest_offer + pair.fluctuation();
                        }
                        if handicap.highest_bid > pair.fluctuation() {
                            info.minimum_offer = handicap.highest_bid - pair.fluctuation();
                        } else {
                            info.minimum_offer = 10_u64.pow(pair.tick_precision); //tick
                        }
                    }

                    pairs.push(info);
                }
            }
        }

        Ok(Some(pairs))
    }

    fn quotations(
        &self,
        pair_index: TradingPairIndex,
        piece: u32,
        hash: Option<<Block as BlockT>::Hash>,
    ) -> Result<Option<QuotationsList>> {
        if piece < 1 || piece > 10 {
            return Err(ErrorKind::QuotationsPieceErr.into());
        }

        let mut quotationslist = QuotationsList::default();
        quotationslist.id = pair_index;
        quotationslist.piece = piece;

        let state = self.state_at(hash)?;

        let sum_of_quotations = |orders: Vec<(AccountId, OrderIndex)>| {
            orders
                .iter()
                .map(|q| {
                    let order_key = <xspot::OrderInfoOf<Runtime>>::key_for(q);
                    Self::pickout::<OrderInfo<Runtime>>(&state, &order_key, Hasher::BLAKE2256)
                        .unwrap()
                })
                .map(|order| {
                    let order = order.unwrap();
                    order
                        .amount()
                        .checked_sub(order.already_filled)
                        .unwrap_or_default()
                })
                .sum::<Balance>()
        };

        let push_sum_quotations_at =
            |price: Balance, quotations_info: &mut Vec<(Balance, Balance)>| -> Result<()> {
                let quotations_key = <xspot::QuotationsOf<Runtime>>::key_for(&(pair_index, price));

                if let Some(orders) = Self::pickout::<Vec<(AccountId, OrderIndex)>>(
                    &state,
                    &quotations_key,
                    Hasher::BLAKE2256,
                )? {
                    if !orders.is_empty() {
                        quotations_info.push((price, sum_of_quotations(orders)));
                    }
                };

                Ok(())
            };

        quotationslist.sell = Vec::new();
        quotationslist.buy = Vec::new();

        let pair_key = <xspot::TradingPairOf<Runtime>>::key_for(&pair_index);
        if let Some(pair) = Self::pickout::<TradingPair>(&state, &pair_key, Hasher::BLAKE2256)? {
            let tick = pair.tick();

            let handicap_key = <xspot::HandicapOf<Runtime>>::key_for(&pair_index);
            if let Some(handicap) =
                Self::pickout::<HandicapInfo<Runtime>>(&state, &handicap_key, Hasher::BLAKE2256)?
            {
                let (lowest_offer, highest_bid) = (handicap.lowest_offer, handicap.highest_bid);

                let maximum_bid = if lowest_offer.is_zero() {
                    0
                } else {
                    lowest_offer + pair.fluctuation()
                };

                let minimum_offer = if highest_bid > pair.fluctuation() {
                    highest_bid - pair.fluctuation()
                } else {
                    10_u64.pow(pair.tick_precision)
                };

                for price in (lowest_offer..=maximum_bid).step_by(tick as usize) {
                    push_sum_quotations_at(price, &mut quotationslist.sell)?;
                    if quotationslist.buy.len() == piece as usize {
                        break;
                    }
                }

                for price in (minimum_offer..=highest_bid).step_by(tick as usize) {
                    push_sum_quotations_at(price, &mut quotationslist.buy)?;
                    if quotationslist.sell.len() == piece as usize {
                        break;
                    }
                }
            };
        } else {
            return Err(ErrorKind::TradingPairIndexErr.into());
        }

        Ok(Some(quotationslist))
    }

    fn orders(
        &self,
        who: AccountIdForRpc,
        page_index: u32,
        page_size: u32,
        hash: Option<<Block as BlockT>::Hash>,
    ) -> Result<Option<PageData<OrderDetails>>> {
        if page_size > MAX_PAGE_SIZE || page_size < 1 {
            return Err(ErrorKind::PageSizeErr.into());
        }

        let mut orders = Vec::new();

        let mut page_total = 0;

        let state = self.state_at(hash)?;

        let order_len_key = <xspot::OrderCountOf<Runtime>>::key_for(&who.unchecked_into());
        if let Some(len) = Self::pickout::<OrderIndex>(&state, &order_len_key, Hasher::BLAKE2256)? {
            let mut total: u32 = 0;
            for i in (0..len).rev() {
                let order_key =
                    <xspot::OrderInfoOf<Runtime>>::key_for(&(who.clone().unchecked_into(), i));
                if let Some(order) =
                    Self::pickout::<OrderInfo<Runtime>>(&state, &order_key, Hasher::BLAKE2256)?
                {
                    if total >= page_index * page_size && total < ((page_index + 1) * page_size) {
                        orders.push(order.clone().into());
                    }
                    total += 1;
                }
            }

            let total_page: u32 = (total + (page_size - 1)) / page_size;

            page_total = total_page;

            if page_index >= total_page && total_page > 0 {
                return Err(ErrorKind::PageIndexErr.into());
            }
        }

        Ok(Some(PageData {
            page_total,
            page_index,
            page_size,
            data: orders,
        }))
    }

    fn address(
        &self,
        who: AccountIdForRpc,
        chain: Chain,
        hash: Option<<Block as BlockT>::Hash>,
    ) -> Result<Option<Vec<String>>> {
        let state = self.state_at(hash)?;

        let who: AccountId = who.unchecked_into();
        match chain {
            Chain::Bitcoin => {
                let key = <xbridge_features::BitcoinCrossChainBinding<Runtime>>::key_for(&who);
                match Self::pickout::<Vec<BitcoinAddress>>(&state, &key, Hasher::BLAKE2256)? {
                    Some(addrs) => {
                        let v = addrs
                            .into_iter()
                            .map(|addr| addr.to_string())
                            .collect::<Vec<_>>();
                        Ok(Some(v))
                    }
                    None => Ok(Some(vec![])),
                }
            }
            Chain::Ethereum => {
                let key = <xbridge_features::EthereumCrossChainBinding<Runtime>>::key_for(&who);
                match Self::pickout::<Vec<EthereumAddress>>(&state, &key, Hasher::BLAKE2256)? {
                    Some(addrs) => {
                        let v = addrs
                            .into_iter()
                            .map(|addr| {
                                let addr: H160 = addr.into();
                                format!("{:?}", addr)
                            })
                            .collect::<Vec<_>>();
                        Ok(Some(v))
                    }
                    None => Ok(Some(vec![])),
                }
            }
            _ => Err(ErrorKind::RuntimeErr(b"not support for this chain".to_vec()).into()),
        }
    }

    fn trustee_session_info_for(
        &self,
        chain: Chain,
        hash: Option<<Block as BlockT>::Hash>,
    ) -> Result<Option<Value>> {
        if let Some((number, info)) =
            self.trustee_session_info_for(self.block_id_by_hash(hash)?, chain)?
        {
            return Ok(parse_trustee_session_info(chain, number, info));
        } else {
            return Ok(None);
        }
    }

    fn trustee_info_for_accountid(
        &self,
        who: AccountIdForRpc,
        hash: Option<<Block as BlockT>::Hash>,
    ) -> Result<Option<Value>> {
        let who: AccountId = who.unchecked_into();
        let props_info = self.trustee_props_for(self.block_id_by_hash(hash)?, who)?;
        Ok(parse_trustee_props(props_info))
    }

    fn fee(
        &self,
        call_params: String,
        tx_length: u64,
        hash: Option<<Block as BlockT>::Hash>,
    ) -> Result<Option<u64>> {
        if !call_params.starts_with("0x") {
            return Err(ErrorKind::BinanryStartErr.into());
        }
        let call_params: Vec<u8> = if let Ok(hex_call) = call_params[2..].from_hex() {
            hex_call
        } else {
            return Err(ErrorKind::HexDecodeErr.into());
        };
        let call: Call = if let Some(call) = Decode::decode(&mut call_params.as_slice()) {
            call
        } else {
            return Err(ErrorKind::DecodeErr.into());
        };

        let transaction_fee =
            self.transaction_fee(self.block_id_by_hash(hash)?, call.encode(), tx_length)?;

        Ok(transaction_fee)
    }

    fn withdraw_tx(
        &self,
        chain: Chain,
        hash: Option<<Block as BlockT>::Hash>,
    ) -> Result<Option<WithdrawTxInfo>> {
        let state = self.state_at(hash)?;
        match chain {
            Chain::Bitcoin => {
                let key = <xbitcoin::CurrentWithdrawalProposal<Runtime>>::key();
                Self::pickout::<xbitcoin::WithdrawalProposal<AccountId>>(
                    &state,
                    &key,
                    Hasher::TWOX128,
                )
                .map(|option_data| {
                    option_data.map(|proposal| WithdrawTxInfo::from_bitcoin_proposal(proposal))
                })
            }
            _ => Ok(None),
        }
    }

    fn mock_bitcoin_new_trustees(
        &self,
        candidates: Vec<AccountIdForRpc>,
        hash: Option<<Block as BlockT>::Hash>,
    ) -> Result<Option<Value>> {
        let candidates: Vec<AccountId> = candidates
            .into_iter()
            .map(|a| a.unchecked_into())
            .collect::<Vec<_>>();

        let runtime_result: result::Result<GenericAllSessionInfo<AccountId>, Vec<u8>> = self
            .client
            .runtime_api()
            .mock_new_trustees(&self.block_id_by_hash(hash)?, Chain::Bitcoin, candidates)?;

        runtime_result
            .map(|all_session_info| parse_trustee_session_info(Chain::Bitcoin, 0, all_session_info))
            .map_err(|e| ErrorKind::RuntimeErr(e).into())
    }

    fn particular_accounts(&self, hash: Option<<Block as BlockT>::Hash>) -> Result<Option<Value>> {
        let state = self.state_at(hash)?;

        // team addr
        let key = xaccounts::TeamAccount::<Runtime>::key();
        let team_account = Self::pickout::<AccountId>(&state, &key, Hasher::TWOX128)?;

        let key = xaccounts::CouncilAccount::<Runtime>::key();
        let council_account = Self::pickout::<AccountId>(&state, &key, Hasher::TWOX128)?;

        let mut map = BTreeMap::new();
        for chain in Chain::iterator() {
            let key = xbridge_features::TrusteeMultiSigAddr::<Runtime>::key_for(chain);
            let addr = Self::pickout::<AccountId>(&state, &key, Hasher::BLAKE2256)?;
            if let Some(a) = addr {
                map.insert(chain, a);
            }
        }

        Ok(Some(json!(
        {
            "teamAccount": team_account,
            "councilAccount": council_account,
            "trusteesAccount": map
        }
        )))
    }
}

fn into_pagedata<T>(src: Vec<T>, page_index: u32, page_size: u32) -> Result<Option<PageData<T>>> {
    if page_size == 0 {
        return Err(ErrorKind::PageSizeErr.into());
    }

    let page_total = (src.len() as u32 + (page_size - 1)) / page_size;
    if page_index >= page_total && page_total > 0 {
        return Err(ErrorKind::PageIndexErr.into());
    }

    let mut list = vec![];
    for (index, item) in src.into_iter().enumerate() {
        let index = index as u32;
        if index >= page_index * page_size && index < ((page_index + 1) * page_size) {
            list.push(item);
        }
    }

    Ok(Some(PageData {
        page_total,
        page_index,
        page_size,
        data: list,
    }))
}
