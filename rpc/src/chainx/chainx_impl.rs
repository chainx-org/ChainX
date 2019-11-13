// Copyright 2018-2019 Chainpool.

use super::*;

use serde_json::{json, Value};

use std::iter::FromIterator;

use parity_codec::{Decode, Encode};
use rustc_hex::FromHex;
// substrate
use primitives::{Blake2Hasher, H160, H256};
use runtime_primitives::generic::SignedBlock;
use runtime_primitives::traits::{
    Block as BlockT, NumberFor, ProvideRuntimeApi, SaturatedConversion,
};

// chainx
use chainx_runtime::Call;
use xr_primitives::AddrStr;

use xassets::{AssetLimit, AssetType, Chain, ChainT};
use xbridge_common::types::GenericAllSessionInfo;
use xbridge_features::{
    self,
    crosschain_binding::{BitcoinAddress, EthereumAddress},
};
use xspot::{HandicapInfo, OrderIndex, OrderInfo, TradingPair, TradingPairIndex};

use crate::chainx::chainx_trait::ChainXApi;
use crate::chainx::utils::*;

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
        + XBridgeApi<Block>
        + XContractsApi<Block>,
{
    fn block_info(&self, number: Option<NumberFor<Block>>) -> Result<Option<SignedBlock<Block>>> {
        Ok(self.client.block(&self.block_id_by_number(number)?)?)
    }

    fn extrinsics_events(&self, hash: Option<<Block as BlockT>::Hash>) -> Result<Value> {
        let hash = hash.unwrap_or(self.client.info().chain.best_hash);
        let number = self.block_number_by_hash(hash)?;

        let state = self.state_at(Some(hash))?;
        let events = self.get_events(&state)?;
        let mut result = BTreeMap::<u32, Vec<String>>::new();
        for event_record in events {
            match event_record.phase {
                system::Phase::ApplyExtrinsic(index) => {
                    let event = format!("{:?}", event_record.event);
                    match result.get_mut(&index) {
                        Some(v) => v.push(event),
                        None => {
                            result.insert(index, vec![event]);
                        }
                    }
                }
                system::Phase::Finalization => {
                    // do nothing
                }
            }
        }
        Ok(json!({
            "events": result,
            "blockHash": hash,
            "number": number,
        }))
    }

    fn events(&self, hash: Option<<Block as BlockT>::Hash>) -> Result<Value> {
        let hash = hash.unwrap_or(self.client.info().chain.best_hash);
        let number = self.block_number_by_hash(hash)?;

        let state = self.state_at(Some(hash))?;
        let events = BTreeMap::<usize, String>::from_iter(
            self.get_events(&state)?
                .iter()
                .enumerate()
                .map(|(index, event_record)| (index, format!("{:?}", event_record.event))),
        );

        Ok(json!({
            "events": events,
            "blockHash": hash,
            "number": number,
        }))
    }

    fn next_renominate(
        &self,
        who: AccountIdForRpc,
        hash: Option<<Block as BlockT>::Hash>,
    ) -> Result<Option<BlockNumber>> {
        let state = self.state_at(hash)?;
        let who: AccountId = who.unchecked_into();
        let key = <xstaking::LastRenominationOf<Runtime>>::key_for(&who);
        if let Some(last_renomination) =
            Self::pickout::<BlockNumber>(&state, &key, Hasher::BLAKE2256)?
        {
            let key = <xstaking::BondingDuration<Runtime>>::key();
            if let Some(bonding_duration) =
                Self::pickout::<BlockNumber>(&state, &key, Hasher::TWOX128)?
            {
                return Ok(Some(last_renomination + bonding_duration));
            }
        }
        Ok(None)
    }

    fn staking_dividend(
        &self,
        who: AccountIdForRpc,
        hash: Option<<Block as BlockT>::Hash>,
    ) -> Result<BTreeMap<AccountIdForRpc, Balance>> {
        let state = self.state_at(hash)?;
        let block_id = self.block_id_by_hash(hash)?;

        let block_number =
            (*self.client.header(&block_id)?.unwrap().number()).saturated_into::<u64>();

        let mut dividends = Vec::new();

        for (intention, record_wrapper) in self.get_nomination_records_wrapper(who, hash)? {
            let record_v1 = record_wrapper.into();
            let intention_profs_v1 = self.into_or_get_intention_profs_v1(&state, &intention)?;

            let jackpot_balance =
                self.get_intention_jackpot_balance(&state, block_id, intention.clone())?;

            let dividend = calculate_staking_dividend(
                &record_v1,
                &intention_profs_v1,
                jackpot_balance,
                block_number,
            );

            dividends.push((intention.into(), dividend as u64));
        }

        Ok(BTreeMap::from_iter(dividends.into_iter()))
    }

    fn cross_mining_dividend(
        &self,
        who: AccountIdForRpc,
        hash: Option<<Block as BlockT>::Hash>,
    ) -> Result<BTreeMap<String, Value>> {
        let state = self.state_at(hash)?;
        let block_id = self.block_id_by_hash(hash)?;
        let who: AccountId = who.unchecked_into();

        let block_number =
            (*self.client.header(&block_id)?.unwrap().number()).saturated_into::<u64>();

        let mut dividends = Vec::new();

        for record_wrapper in self.get_psedu_nomination_records_wrapper(&state, who.clone())? {
            let token = record_wrapper.common.id.clone().into_bytes();
            let p1 = self.into_or_get_psedu_intention_profs_v1(&state, &token)?;
            let d1 = record_wrapper.into();

            let jackpot_balance =
                self.get_psedu_intention_jackpot_balance(&state, block_id, token.clone())?;
            let miner_balance = self.get_token_free_balance(&state, who.clone(), token.clone())?;
            let total_token_balance = self.get_token_total_asset_balance(&state, &token)?;

            let (referral, unclaimed) = calculate_cross_mining_dividend(
                d1,
                p1,
                jackpot_balance,
                block_number,
                total_token_balance,
                miner_balance,
            );

            dividends.push((
                to_string!(&token),
                json!({
                    "referral": referral,
                    "unclaimed": unclaimed
                }),
            ));
        }

        Ok(BTreeMap::from_iter(dividends.into_iter()))
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
                    name: to_string!(&token),
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

            let mut lmap = BTreeMap::<AssetLimit, bool>::from_iter(
                xassets::AssetLimit::iterator().map(|t| (*t, true)),
            );
            let key = <xassets::AssetLimitProps<Runtime>>::key_for(asset.token().as_ref());
            if let Some(limit) =
                Self::pickout::<BTreeMap<AssetLimit, bool>>(&state, &key, Hasher::BLAKE2256)?
            {
                lmap.extend(limit.iter());
            }

            all_assets.push(TotalAssetInfo::new(asset, valid, bmap, lmap));
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
                Err(s) => Ok(Some(to_string!(s.as_ref()))),
            });
        let is_valid = match ret {
            Err(_) | Ok(Some(_)) => false,
            Ok(None) => true,
        };
        Ok(Some(is_valid))
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

    fn deposit_limit(
        &self,
        token: String,
        hash: Option<<Block as BlockT>::Hash>,
    ) -> Result<Option<DepositLimit>> {
        let token: xassets::Token = token.as_bytes().to_vec();

        if xassets::is_valid_token(&token).is_err() {
            return Ok(None);
        }
        let state = self.state_at(hash)?;
        // todo use `cando` to refactor if
        if token.as_slice() == xbitcoin::Module::<Runtime>::TOKEN {
            let key = <xbitcoin::BtcMinDeposit<Runtime>>::key();
            Self::pickout::<u64>(&state, &key, Hasher::TWOX128).map(|value| {
                Some(DepositLimit {
                    minimal_deposit: value.unwrap_or(100000),
                })
            })
        } else {
            return Ok(None);
        }
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
    ) -> Result<Option<Vec<(AccountIdForRpc, NominationRecordForRpc)>>> {
        let mut records = Vec::new();
        for (nominee, record_wrapper) in self.get_nomination_records_wrapper(who, hash)? {
            if record_wrapper.0.is_err() {
                return Err(Error::DeprecatedV0Err("chainx_getNominationRecords".into()).into());
            }
            records.push((nominee.into(), record_wrapper.into()));
        }

        Ok(Some(records))
    }

    fn nomination_records_v1(
        &self,
        who: AccountIdForRpc,
        hash: Option<<Block as BlockT>::Hash>,
    ) -> Result<Option<Vec<(AccountIdForRpc, NominationRecordV1ForRpc)>>> {
        Ok(Some(
            self.get_nomination_records_wrapper(who, hash)?
                .into_iter()
                .map(|(a, b)| (a.into(), b.into()))
                .collect(),
        ))
    }

    fn intentions(
        &self,
        hash: Option<<Block as BlockT>::Hash>,
    ) -> Result<Option<Vec<IntentionInfo>>> {
        let r = lru_cache!(Option<Vec<IntentionInfo>>; hash; self {
        let state = self.state_at(hash)?;
        let block_id = self.block_id_by_hash(hash)?;

        let mut intentions_info = Vec::new();
        for info_wrapper in self.get_intentions_info_wrapper(&state, (block_id, hash))? {
            if info_wrapper.intention_profs_wrapper.is_err() {
                return Err(Error::DeprecatedV0Err("chainx_getIntentions".into()).into());
            }
            intentions_info.push(info_wrapper.into());
        }

        Some(intentions_info)
        });
        Ok(r)
    }

    fn intentions_v1(
        &self,
        hash: Option<<Block as BlockT>::Hash>,
    ) -> Result<Option<Vec<IntentionInfoV1>>> {
        let r = lru_cache!(Option<Vec<IntentionInfoV1>>; hash; self {
        let state = self.state_at(hash)?;
        let block_id = self.block_id_by_hash(hash)?;

        Some(
            self.get_intentions_info_wrapper(&state, (block_id, hash))?
                .into_iter()
                .map(Into::into)
                .collect(),
        )
        });
        Ok(r)
    }

    fn intention(
        &self,
        who: AccountIdForRpc,
        hash: Option<<Block as BlockT>::Hash>,
    ) -> Result<Option<IntentionInfo>> {
        let state = self.state_at(hash)?;
        let block_id = self.block_id_by_hash(hash)?;
        let who: AccountId = who.unchecked_into();

        let info_wrapper = self.get_intention_info_wrapper(&state, (block_id, hash), who)?;
        if let Some(ref info) = info_wrapper {
            if info.intention_profs_wrapper.is_err() {
                return Err(Error::DeprecatedV0Err("chainx_getIntentionByAccount".into()).into());
            }
        }
        Ok(info_wrapper.map(Into::into))
    }

    fn intention_v1(
        &self,
        who: AccountIdForRpc,
        hash: Option<<Block as BlockT>::Hash>,
    ) -> Result<Option<IntentionInfoV1>> {
        let state = self.state_at(hash)?;
        let block_id = self.block_id_by_hash(hash)?;
        let who: AccountId = who.unchecked_into();

        Ok(self
            .get_intention_info_wrapper(&state, (block_id, hash), who)?
            .map(Into::into))
    }

    fn psedu_intentions(
        &self,
        hash: Option<<Block as BlockT>::Hash>,
    ) -> Result<Option<Vec<PseduIntentionInfo>>> {
        let state = self.state_at(hash)?;
        let block_id = self.block_id_by_hash(hash)?;

        let mut psedu_intentions_info = Vec::new();
        for info_wrapper in self.get_psedu_intentions_info_wrapper(&state, block_id)? {
            if info_wrapper.psedu_intention_profs_wrapper.is_err() {
                return Err(Error::DeprecatedV0Err("chainx_getPseduIntentions".into()).into());
            }
            psedu_intentions_info.push(info_wrapper.into());
        }

        Ok(Some(psedu_intentions_info))
    }

    fn psedu_intentions_v1(
        &self,
        hash: Option<<Block as BlockT>::Hash>,
    ) -> Result<Option<Vec<PseduIntentionInfoV1>>> {
        let state = self.state_at(hash)?;
        let block_id = self.block_id_by_hash(hash)?;

        Ok(Some(
            self.get_psedu_intentions_info_wrapper(&state, block_id)?
                .into_iter()
                .map(Into::into)
                .collect(),
        ))
    }

    fn psedu_nomination_records(
        &self,
        who: AccountIdForRpc,
        hash: Option<<Block as BlockT>::Hash>,
    ) -> Result<Option<Vec<PseduNominationRecord>>> {
        let state = self.state_at(hash)?;
        let who: AccountId = who.unchecked_into();

        let mut psedu_records = Vec::new();
        for record_wrapper in self.get_psedu_nomination_records_wrapper(&state, who)? {
            if record_wrapper.deposit_vote_weight_wrapper.is_err() {
                return Err(
                    Error::DeprecatedV0Err("chainx_getPseduNominationRecords".into()).into(),
                );
            }
            psedu_records.push(record_wrapper.into());
        }

        Ok(Some(psedu_records))
    }

    fn psedu_nomination_records_v1(
        &self,
        who: AccountIdForRpc,
        hash: Option<<Block as BlockT>::Hash>,
    ) -> Result<Option<Vec<PseduNominationRecordV1>>> {
        let state = self.state_at(hash)?;
        let who: AccountId = who.unchecked_into();

        Ok(Some(
            self.get_psedu_nomination_records_wrapper(&state, who)?
                .into_iter()
                .map(Into::into)
                .collect(),
        ))
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
                    info.assets = to_string!(pair.base_as_ref());
                    info.currency = to_string!(pair.quote_as_ref());
                    info.precision = pair.pip_precision;
                    info.online = pair.online;
                    info.unit_precision = pair.tick_precision;

                    let price_key = <xspot::TradingPairInfoOf<Runtime>>::key_for(&i);
                    if let Some((last_price, aver_price, update_height)) =
                        Self::pickout::<(Balance, Balance, BlockNumber)>(
                            &state,
                            &price_key,
                            Hasher::BLAKE2256,
                        )?
                    {
                        info.last_price = last_price;
                        info.aver_price = aver_price;
                        info.update_height = update_height;
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
                        info.minimum_offer = if handicap.highest_bid > pair.fluctuation() {
                            handicap.highest_bid - pair.fluctuation()
                        } else {
                            pair.tick()
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
            return Err(Error::QuotationsPieceErr(pair_index).into());
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
            return Err(Error::TradingPairIndexErr(pair_index).into());
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
            return Err(Error::PageSizeErr(page_size).into());
        }

        let mut orders = Vec::new();
        let mut page_total = 0;

        let state = self.state_at(hash)?;

        let who: AccountId = who.unchecked_into();

        let order_len_key = <xspot::OrderCountOf<Runtime>>::key_for(&who);
        if let Some(len) = Self::pickout::<OrderIndex>(&state, &order_len_key, Hasher::BLAKE2256)? {
            let mut total: u32 = 0;
            for i in (0..len).rev() {
                let order_key = <xspot::OrderInfoOf<Runtime>>::key_for(&(who.clone(), i));
                if let Some(order) =
                    Self::pickout::<OrderInfo<Runtime>>(&state, &order_key, Hasher::BLAKE2256)?
                {
                    if total >= page_index * page_size && total < ((page_index + 1) * page_size) {
                        orders.push(order.into());
                    }
                    total += 1;
                }
            }

            let total_page: u32 = (total + (page_size - 1)) / page_size;

            page_total = total_page;

            if page_index >= total_page && total_page > 0 {
                return Err(Error::PageIndexErr(page_index).into());
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
            _ => Err(Error::RuntimeErr(b"not support for this chain".to_vec(), None).into()),
        }
    }

    fn trustee_session_info_for(
        &self,
        chain: Chain,
        number: Option<u32>,
        hash: Option<<Block as BlockT>::Hash>,
    ) -> Result<Option<Value>> {
        if let Some((number, info)) =
            self.trustee_session_info_for(self.block_id_by_hash(hash)?, chain, number)?
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
            return Err(Error::BinaryStartErr.into());
        }
        let call_params: Vec<u8> = if let Ok(hex_call) = call_params[2..].from_hex() {
            hex_call
        } else {
            return Err(Error::HexDecodeErr.into());
        };
        let call: Call = if let Some(call) = Decode::decode(&mut call_params.as_slice()) {
            call
        } else {
            return Err(Error::DecodeErr.into());
        };

        let transaction_fee =
            self.transaction_fee(self.block_id_by_hash(hash)?, call.encode(), tx_length)?;

        Ok(transaction_fee)
    }

    fn fee_weight_map(&self, hash: Option<<Block as BlockT>::Hash>) -> Result<Value> {
        let fee_weight: Result<BTreeMap<String, Balance>> = self
            .client
            .runtime_api()
            .fee_weight_map(&self.block_id_by_hash(hash)?)
            .map(|m| m.into_iter().map(|(k, v)| (to_string!(&k), v)).collect())
            .map_err(Into::into);
        let fee_weight = fee_weight?;
        let state = self.state_at(hash)?;
        let key = <xfee_manager::TransactionBaseFee<Runtime>>::key();
        let transaction_base_fee =
            Self::pickout::<Balance>(&state, &key, Hasher::TWOX128)?.unwrap_or(10000);
        let key = <xfee_manager::TransactionByteFee<Runtime>>::key();
        let transaction_byte_fee =
            Self::pickout::<Balance>(&state, &key, Hasher::TWOX128)?.unwrap_or(100);
        Ok(json!(
            {
                "transactionBaseFee": transaction_base_fee,
                "transactionByteFee": transaction_byte_fee,
                "feeWeight": fee_weight,
            }
        ))
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
            .map_err(|e| Error::RuntimeErr(e, None).into())
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

    fn contract_call(
        &self,
        call_request: CallRequest,
        at: Option<<Block as BlockT>::Hash>,
    ) -> Result<Value> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(||
            // If the block hash is not supplied assume the best block.
            self.client.info().chain.best_hash));

        let CallRequest {
            origin,
            dest,
            gas_limit,
            input_data,
        } = call_request;

        let exec_result = api
            .call(
                &at,
                origin,
                dest,
                Zero::zero(),
                gas_limit,
                input_data.to_vec(),
            )
            .map_err(|e| {
                Error::RuntimeErr(
                    b"Runtime trapped while executing a contract.".to_vec(),
                    Some(format!("{:?}", e)),
                )
            })?;

        match exec_result {
            ContractExecResult::Success { status, data } => {
                let real_data: Vec<u8> =
                    Decode::decode(&mut data.as_slice()).ok_or(Error::DecodeErr)?;

                Ok(json!({
                    "status": status,
                    "data": Bytes(real_data),
                }))
            }
            ContractExecResult::Error(e) => Err(Error::RuntimeErr(e, None)),
        }
    }

    fn contract_get_storage(
        &self,
        address: AccountIdForRpc,
        key: H256,
        at: Option<<Block as BlockT>::Hash>,
    ) -> Result<Option<Bytes>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(||
            // If the block hash is not supplied assume the best block.
            self.client.info().chain.best_hash));
        let address: AccountId = address.unchecked_into();

        let get_storage_result = api
            .get_storage(&at, address, key.into())
            .map_err(|e|
                // Handle general API calling errors.
                Error::RuntimeErr(
                    b"Runtime trapped while querying storage.".to_vec(),
                    Some(format!("{:?}", e)),
                ))?
            .map_err(|e| Error::ContractGetStorageError(e))?
            .map(Bytes);

        Ok(get_storage_result)
    }

    fn contract_erc20_call(
        &self,
        call_request: Erc20CallRequest,
        at: Option<<Block as BlockT>::Hash>,
    ) -> Result<Value> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(||
            // If the block hash is not supplied assume the best block.
            self.client.info().chain.best_hash));
        let token = call_request.token.as_bytes().to_vec();
        xassets::is_valid_token(&token).map_err(|e| {
            Error::RuntimeErr(
                e.as_bytes().to_vec(),
                Some("not allow this token for this rpc call".to_string()),
            )
        })?;
        let exec_result = api
            .erc20_call(
                &at,
                token,
                call_request.selector,
                call_request.input_data.to_vec(),
            )
            .map_err(|e| {
                Error::RuntimeErr(
                    b"Runtime trapped while executing a contract.".to_vec(),
                    Some(format!("{:?}", e)),
                )
            })?;

        match exec_result {
            ContractExecResult::Success { status, data } => {
                let real_data: Vec<u8> =
                    Decode::decode(&mut data.as_slice()).ok_or(Error::DecodeErr)?;
                // todo decode dependency on selector
                let result = match call_request.selector {
                    ERC20Selector::BalanceOf | ERC20Selector::TotalSupply => {
                        let v: u64 =
                            Decode::decode(&mut real_data.as_slice()).ok_or(Error::DecodeErr)?;
                        json!({
                            "status": status,
                            "data": v,
                        })
                    }
                    ERC20Selector::Name | ERC20Selector::Symbol => {
                        let v: Vec<u8> =
                            Decode::decode(&mut real_data.as_slice()).ok_or(Error::DecodeErr)?;
                        json!({
                            "status": status,
                            "data": to_string!(&v),
                        })
                    }
                    ERC20Selector::Decimals => {
                        let v: u16 =
                            Decode::decode(&mut real_data.as_slice()).ok_or(Error::DecodeErr)?;
                        json!({
                            "status": status,
                            "data": v,
                        })
                    }
                    _ => json!({
                        "status": status,
                        "data": Bytes(real_data),
                    }),
                };

                Ok(result)
            }
            ContractExecResult::Error(e) => Err(Error::RuntimeErr(e, None)),
        }
    }

    fn contract_erc20_info(
        &self,
        hash: Option<<Block as BlockT>::Hash>,
    ) -> Result<BTreeMap<String, Value>> {
        let state = self.state_at(hash)?;

        let assets = self.all_assets(self.block_id_by_hash(hash)?)?;

        let mut b = BTreeMap::new();

        for (asset, _valid) in assets.into_iter() {
            let token = asset.token();
            let key = <xcontracts::Erc20InfoOfToken<Runtime>>::key_for(token.as_ref());
            if let Some(info) = Self::pickout::<(
                AccountId,
                BTreeMap<xcontracts::ERC20Selector, xcontracts::Selector>,
            )>(&state, &key, Hasher::BLAKE2256)?
            {
                b.insert(to_string!(&token), json!({
                    "erc20": json!({
                        "address": info.0,
                        "selectors": info.1.into_iter().map(|(k, v)| (k, Bytes(v.to_vec()))).collect::<BTreeMap<_, _>>(),
                    })
                }));
            }
        }
        Ok(b)
    }
}

fn into_pagedata<T>(src: Vec<T>, page_index: u32, page_size: u32) -> Result<Option<PageData<T>>> {
    if page_size == 0 {
        return Err(Error::PageSizeErr(page_size).into());
    }

    let page_total = (src.len() as u32 + (page_size - 1)) / page_size;
    if page_index >= page_total && page_total > 0 {
        return Err(Error::PageIndexErr(page_index).into());
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
