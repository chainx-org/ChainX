// Copyright 2018-2019 Chainpool.

use std::convert::Into;
use std::iter::FromIterator;

use parity_codec::{Decode, Encode};
use rustc_hex::{FromHex, ToHex};

use client::runtime_api::Metadata;
use primitives::crypto::UncheckedInto;
use runtime_primitives::traits::{Header, ProvideRuntimeApi};
use support::storage::generator::{StorageMap, StorageValue};
use xassets::ChainT;

use btc_keys::{Address, DisplayLayout};

use runtime_api::{
    xassets_api::XAssetsApi, xbridge_api::XBridgeApi, xfee_api::XFeeApi, xmining_api::XMiningApi,
    xspot_api::XSpotApi,
};

use super::*;
use parity_codec::alloc::collections::btree_map::BTreeMap;

impl<B, E, Block, RA>
    ChainXApi<NumberFor<Block>, AccountIdForRpc, Balance, BlockNumber, SignedBlock<Block>>
    for ChainX<B, E, Block, RA>
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
        + XBridgeApi<Block>,
{
    fn block_info(&self, number: Option<NumberFor<Block>>) -> Result<Option<SignedBlock<Block>>> {
        let hash = match number {
            None => Some(self.client.info()?.chain.best_hash),
            Some(number) => self
                .client
                .header(&BlockId::number(number))?
                .map(|h| h.hash()),
        };
        let block_hash = match hash {
            None => self.client.info()?.chain.best_hash,
            Some(h) => h,
        };

        Ok(self.client.block(&BlockId::Hash(block_hash))?)
    }

    fn assets_of(
        &self,
        who: AccountIdForRpc,
        page_index: u32,
        page_size: u32,
    ) -> Result<Option<PageData<AssetInfo>>> {
        let b = self.best_number()?;
        let assets: Result<Vec<(Token, BTreeMap<AssetType, Balance>)>> = self
            .client
            .runtime_api()
            .valid_assets_of(&b, who.unchecked_into())
            .map_err(Into::into);

        let assets = assets?;
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

    fn assets(&self, page_index: u32, page_size: u32) -> Result<Option<PageData<TotalAssetInfo>>> {
        let b = self.best_number()?;
        let assets: Result<Vec<(Asset, bool)>> =
            self.client.runtime_api().all_assets(&b).map_err(Into::into);
        let assets = assets?;

        let state = self.best_state()?;

        let mut all_assets = Vec::new();

        for (asset, valid) in assets.into_iter() {
            let mut bmap = BTreeMap::<AssetType, Balance>::from_iter(
                xassets::AssetType::iterator().map(|t| (*t, Zero::zero())),
            );

            let key = <xassets::TotalAssetBalance<Runtime>>::key_for(asset.token().as_ref());
            if let Some(info) = Self::pickout::<BTreeMap<AssetType, Balance>>(&state, &key)? {
                bmap.extend(info.iter());
            }

            all_assets.push(TotalAssetInfo::new(asset, valid, bmap));
        }

        into_pagedata(all_assets, page_index, page_size)
    }

    fn verify_addr(&self, token: String, addr: String, memo: String) -> Result<Option<bool>> {
        let token: xassets::Token = token.as_bytes().to_vec();
        let addr: xrecords::AddrStr = addr.as_bytes().to_vec();
        let memo: xassets::Memo = memo.as_bytes().to_vec();

        // test valid before call runtime api
        if let Err(_e) = xassets::is_valid_token(&token) {
            //            return Ok(Some(String::from_utf8_lossy(e.as_ref()).into_owned()));
            return Ok(Some(false));
        }

        if addr.len() > 256 || memo.len() > 256 {
            //            return Ok(Some("the addr or memo may be too long".to_string()));
            return Ok(Some(false));
        }

        let b = self.best_number()?;
        let ret = self
            .client
            .runtime_api()
            .verify_address(&b, token, addr, memo)
            .and_then(|r| match r {
                Ok(()) => Ok(None),
                Err(s) => Ok(Some(String::from_utf8_lossy(s.as_ref()).into_owned())),
            });
        //            .map_err(|e| e.into());
        // Err() => substrate inner err
        // Ok(None) => runtime_api return true
        // Ok(Some(err_info)) => runtime_api return false, Some() contains err info
        match ret {
            Err(_) => Ok(Some(false)),
            Ok(ret) => match ret {
                None => Ok(Some(true)),
                Some(_) => Ok(Some(false)),
            },
        }
    }

    fn minimal_withdrawal_value(&self, token: String) -> Result<Option<Balance>> {
        let token: xassets::Token = token.as_bytes().to_vec();

        // test valid before call runtime api
        if xassets::is_valid_token(&token).is_err() {
            return Ok(None);
        }
        let b = self.best_number()?;
        self.client
            .runtime_api()
            .minimal_withdrawal_value(&b, token)
            .map_err(Into::into)
    }

    fn deposit_list(
        &self,
        chain: Chain,
        page_index: u32,
        page_size: u32,
    ) -> Result<Option<PageData<DepositInfo>>> {
        let best_number = self.best_number()?;

        let list: Vec<xrecords::RecordInfo<AccountId, Balance, BlockNumber, Timestamp>> = self
            .client
            .runtime_api()
            .deposit_list_of(&best_number, chain)
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
    ) -> Result<Option<PageData<WithdrawInfo>>> {
        let best_number = self.best_number()?;
        let list: Vec<xrecords::RecordInfo<AccountId, Balance, BlockNumber, Timestamp>> = self
            .client
            .runtime_api()
            .withdrawal_list_of(&best_number, chain)
            .unwrap_or_default();

        let records: Vec<WithdrawInfo> = list.into_iter().map(Into::into).collect();
        into_pagedata(records, page_index, page_size)
    }

    fn nomination_records(
        &self,
        who: AccountIdForRpc,
    ) -> Result<Option<Vec<(AccountIdForRpc, NominationRecord)>>> {
        let state = self.best_state()?;

        let mut records = Vec::new();

        let key = <xstaking::Intentions<Runtime>>::key();
        if let Some(intentions) = Self::pickout::<Vec<AccountId>>(&state, &key)? {
            for intention in intentions {
                let key = <xstaking::NominationRecords<Runtime>>::key_for(&(
                    who.clone().unchecked_into(),
                    intention.clone(),
                ));
                if let Some(record) =
                    Self::pickout::<xstaking::NominationRecord<Balance, BlockNumber>>(&state, &key)?
                {
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
        }

        Ok(Some(records))
    }

    fn intentions(&self) -> Result<Option<Vec<IntentionInfo>>> {
        let state = self.best_state()?;
        let mut intention_info = Vec::new();

        let key = <xsession::Validators<Runtime>>::key();
        let validators = Self::pickout::<Vec<(AccountId, u64)>>(&state, &key)?
            .expect("Validators can't be empty");
        let validators: Vec<AccountId> = validators.into_iter().map(|(who, _)| who).collect();

        // get all bridge trustee list
        let mut all_trustees: BTreeMap<Chain, Vec<AccountId>> = BTreeMap::new();
        for chain in Chain::iterator() {
            let (info, _) = Self::current_trustee_session_info(&state, *chain)?.unwrap_or_default();
            all_trustees.insert(*chain, info.trustee_list);
        }
        let is_trustee = |who: &AccountId| -> Vec<Chain> {
            let mut ret = vec![];
            for (chain, trustees) in all_trustees.iter() {
                if trustees.contains(who) {
                    ret.push(*chain);
                }
            }
            ret
        };

        let key = <xstaking::Intentions<Runtime>>::key();

        if let Some(intentions) = Self::pickout::<Vec<AccountId>>(&state, &key)? {
            let jackpot_addr_list: Result<Vec<AccountId>> = self
                .client
                .runtime_api()
                .multi_jackpot_accountid_for(&self.best_number()?, intentions.clone())
                .map_err(Into::into);
            let jackpot_addr_list: Vec<AccountId> = jackpot_addr_list?;

            for (intention, jackpot_addr) in intentions.into_iter().zip(jackpot_addr_list) {
                let mut info = IntentionInfo::default();

                let key = <xaccounts::IntentionNameOf<Runtime>>::key_for(&intention);
                if let Some(name) = Self::pickout::<xaccounts::Name>(&state, &key)? {
                    info.name = String::from_utf8_lossy(&name).into_owned();
                }

                let key = <xaccounts::IntentionPropertiesOf<Runtime>>::key_for(&intention);
                if let Some(props) = Self::pickout::<IntentionProps<AuthorityId>>(&state, &key)? {
                    info.url = String::from_utf8_lossy(&props.url).into_owned();
                    info.is_active = props.is_active;
                    info.about = String::from_utf8_lossy(&props.about).into_owned();
                    info.session_key = match props.session_key {
                        Some(s) => s.into(),
                        None => intention.clone().into(),
                    };
                }

                let key = <xstaking::IntentionProfiles<Runtime>>::key_for(&intention);
                if let Some(profs) =
                    Self::pickout::<IntentionProfs<Balance, BlockNumber>>(&state, &key)?
                {
                    let key = (
                        jackpot_addr.clone(),
                        xassets::Module::<Runtime>::TOKEN.to_vec(),
                    );
                    let balances_key = <xassets::AssetBalance<Runtime>>::key_for(&key);
                    let map = Self::pickout::<BTreeMap<AssetType, Balance>>(&state, &balances_key)?
                        .unwrap_or_default();
                    let free = map
                        .get(&AssetType::Free)
                        .map(|free| *free)
                        .unwrap_or_default();
                    info.jackpot = free;
                    info.jackpot_address = jackpot_addr.into();
                    info.total_nomination = profs.total_nomination;
                    info.last_total_vote_weight = profs.last_total_vote_weight;
                    info.last_total_vote_weight_update = profs.last_total_vote_weight_update;
                }

                let key = <xstaking::NominationRecords<Runtime>>::key_for(&(
                    intention.clone(),
                    intention.clone(),
                ));
                if let Some(record) =
                    Self::pickout::<xstaking::NominationRecord<Balance, BlockNumber>>(&state, &key)?
                {
                    info.self_vote = record.nomination;
                }

                info.is_validator = validators.iter().any(|i| i == &intention);
                info.is_trustee = is_trustee(&intention);
                info.account = intention.into();

                intention_info.push(info);
            }
        }

        Ok(Some(intention_info))
    }

    fn psedu_intentions(&self) -> Result<Option<Vec<PseduIntentionInfo>>> {
        let state = self.best_state()?;
        let mut psedu_intentions = Vec::new();

        let key = <xtokens::PseduIntentions<Runtime>>::key();
        if let Some(tokens) = Self::pickout::<Vec<Token>>(&state, &key)? {
            let jackpot_addr_list: Result<Vec<AccountId>> = self
                .client
                .runtime_api()
                .multi_token_jackpot_accountid_for(&self.best_number()?, tokens.clone())
                .map_err(Into::into);
            let jackpot_addr_list = jackpot_addr_list?;

            for (token, jackpot_addr) in tokens.into_iter().zip(jackpot_addr_list) {
                let mut info = PseduIntentionInfo::default();

                let key = <xtokens::PseduIntentionProfiles<Runtime>>::key_for(&token);
                if let Some(vote_weight) =
                    Self::pickout::<PseduIntentionVoteWeight<Balance>>(&state, &key)?
                {
                    let key = (
                        jackpot_addr.clone(),
                        xassets::Module::<Runtime>::TOKEN.to_vec(),
                    );
                    let balances_key = <xassets::AssetBalance<Runtime>>::key_for(&key);
                    let map = Self::pickout::<BTreeMap<AssetType, Balance>>(&state, &balances_key)?
                        .unwrap_or_default();
                    let free = map
                        .get(&AssetType::Free)
                        .map(|free| *free)
                        .unwrap_or_default();
                    info.jackpot = free;
                    info.jackpot_address = jackpot_addr.into();
                    info.last_total_deposit_weight = vote_weight.last_total_deposit_weight;
                    info.last_total_deposit_weight_update =
                        vote_weight.last_total_deposit_weight_update;
                }

                let key = <xtokens::TokenDiscount<Runtime>>::key_for(&token);
                if let Some(discount) = Self::pickout::<u32>(&state, &key)? {
                    info.discount = discount;
                }

                //注意
                //这里返回的是以PCX计价的"单位"token的价格，已含pcx精度
                //譬如1BTC=10000PCX，则返回的是10000*（10.pow(pcx精度))
                //因此，如果前端要换算折合投票数的时候
                //应该=(资产数量[含精度的数字]*price)/(10^资产精度)=PCX[含PCX精度]

                let b = self.best_number()?;
                if let Ok(Some(price)) = self
                    .client
                    .runtime_api()
                    .aver_asset_price(&b, token.clone())
                {
                    info.price = price;
                };

                let b = self.best_number()?;
                if let Ok(Some(power)) = self.client.runtime_api().asset_power(&b, token.clone()) {
                    info.power = power;
                };

                let key = <xassets::TotalAssetBalance<Runtime>>::key_for(&token);
                if let Some(total_asset_balance) =
                    Self::pickout::<BTreeMap<AssetType, Balance>>(&state, &key)?
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
    ) -> Result<Option<Vec<PseduNominationRecord>>> {
        let state = self.best_state()?;
        let mut psedu_records = Vec::new();

        let key = <xtokens::PseduIntentions<Runtime>>::key();
        if let Some(tokens) = Self::pickout::<Vec<Token>>(&state, &key)? {
            for token in tokens {
                let mut record = PseduNominationRecord::default();

                let key = <xtokens::DepositRecords<Runtime>>::key_for(&(
                    who.clone().unchecked_into(),
                    token.clone(),
                ));
                if let Some(vote_weight) =
                    Self::pickout::<DepositVoteWeight<BlockNumber>>(&state, &key)?
                {
                    record.last_total_deposit_weight = vote_weight.last_deposit_weight;
                    record.last_total_deposit_weight_update =
                        vote_weight.last_deposit_weight_update;
                }

                let key = <xassets::AssetBalance<Runtime>>::key_for(&(
                    who.clone().unchecked_into(),
                    token.clone(),
                ));

                if let Some(balances) = Self::pickout::<BTreeMap<AssetType, Balance>>(&state, &key)?
                {
                    record.balance = balances.iter().fold(Zero::zero(), |acc, (_, v)| acc + *v);
                }

                record.id = String::from_utf8_lossy(&token).into_owned();

                psedu_records.push(record);
            }
        }

        Ok(Some(psedu_records))
    }

    fn trading_pairs(&self) -> Result<Option<Vec<(PairInfo)>>> {
        let mut pairs = Vec::new();
        let state = self.best_state()?;

        let len_key = <xspot::TradingPairCount<Runtime>>::key();
        if let Some(len) = Self::pickout::<TradingPairIndex>(&state, &len_key)? {
            for i in 0..len {
                let key = <xspot::TradingPairOf<Runtime>>::key_for(&i);
                if let Some(pair) = Self::pickout::<TradingPair>(&state, &key)? {
                    let mut info = PairInfo::default();
                    info.id = pair.index;
                    info.assets = String::from_utf8_lossy(pair.base_as_ref()).into_owned();
                    info.currency = String::from_utf8_lossy(pair.quote_as_ref()).into_owned();
                    info.precision = pair.pip_precision;
                    info.online = pair.online;
                    info.unit_precision = pair.tick_precision;

                    let price_key = <xspot::TradingPairInfoOf<Runtime>>::key_for(&i);
                    if let Some(price) =
                        Self::pickout::<(Balance, Balance, BlockNumber)>(&state, &price_key)?
                    {
                        info.last_price = price.0;
                        info.aver_price = price.1;
                        info.update_height = price.2;
                    }

                    let price_volatility_key = <xspot::PriceVolatility<Runtime>>::key();
                    let price_volatility =
                        Self::pickout::<u32>(&state, &price_volatility_key)?.unwrap() as u64;

                    let handicap_key = <xspot::HandicapOf<Runtime>>::key_for(&i);
                    if let Some(handicap) =
                        Self::pickout::<HandicapInfo<Runtime>>(&state, &handicap_key)?
                    {
                        info.buy_one = handicap.highest_bid;
                        info.maximum_bid =
                            handicap.lowest_offer + handicap.lowest_offer * price_volatility / 100;

                        info.sell_one = handicap.lowest_offer;
                        info.minimum_offer =
                            handicap.highest_bid - handicap.highest_bid * price_volatility / 100;
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
    ) -> Result<Option<QuotationsList>> {
        if piece < 1 || piece > 10 {
            return Err(ErrorKind::QuotationsPieceErr.into());
        }

        let mut quotationslist = QuotationsList::default();
        quotationslist.id = pair_index;
        quotationslist.piece = piece;
        quotationslist.sell = Vec::new();
        quotationslist.buy = Vec::new();

        let state = self.best_state()?;
        let pair_key = <xspot::TradingPairOf<Runtime>>::key_for(&pair_index);
        if let Some(pair) = Self::pickout::<TradingPair>(&state, &pair_key)? {
            let min_unit = 10_u64.pow(pair.tick_precision);

            //盘口
            let handicap_key = <xspot::HandicapOf<Runtime>>::key_for(&pair_index);

            if let Some(handicap) = Self::pickout::<HandicapInfo<Runtime>>(&state, &handicap_key)? {
                //先买档
                let mut opponent_price = handicap.highest_bid;

                // price_volatility definitely exists.
                let price_volatility_key = <xspot::PriceVolatility<Runtime>>::key();
                let price_volatility =
                    Self::pickout::<u32>(&state, &price_volatility_key)?.unwrap() as u64;

                let max_price = (handicap.lowest_offer * (100_u64 + price_volatility)) / 100_u64;
                let min_price = (handicap.lowest_offer * (100_u64 - price_volatility)) / 100_u64;

                let mut n = 0;

                let sum_of_quotations = |quotations: Vec<(AccountId, OrderIndex)>| {
                    quotations
                        .iter()
                        .map(|q| {
                            let order_key = <xspot::OrderInfoOf<Runtime>>::key_for(q);
                            Self::pickout::<OrderInfo<Runtime>>(&state, &order_key).unwrap()
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

                let push_sum_quotations_given_price =
                    |n: u32,
                     price: Balance,
                     quotations_info: &mut Vec<(Balance, Balance)>|
                     -> Result<u32> {
                        let quotations_key =
                            <xspot::QuotationsOf<Runtime>>::key_for(&(pair_index, price));

                        if let Some(quotations) =
                            Self::pickout::<Vec<(AccountId, OrderIndex)>>(&state, &quotations_key)?
                        {
                            if !quotations.is_empty() {
                                quotations_info.push((price, sum_of_quotations(quotations)));
                                return Ok(n + 1);
                            }
                        };

                        Ok(n)
                    };

                loop {
                    if n > piece || opponent_price == 0 || opponent_price < min_price {
                        break;
                    }

                    n = push_sum_quotations_given_price(
                        n,
                        opponent_price,
                        &mut quotationslist.buy,
                    )?;

                    opponent_price = opponent_price
                        .checked_sub(As::sa(min_unit))
                        .unwrap_or_default();
                }

                //再卖档
                opponent_price = handicap.lowest_offer;
                n = 0;
                loop {
                    if n > piece || opponent_price == 0 || opponent_price > max_price {
                        break;
                    }

                    n = push_sum_quotations_given_price(
                        n,
                        opponent_price,
                        &mut quotationslist.sell,
                    )?;

                    opponent_price = opponent_price
                        .checked_add(As::sa(min_unit))
                        .unwrap_or_default();
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
    ) -> Result<Option<PageData<OrderDetails>>> {
        if page_size > MAX_PAGE_SIZE || page_size < 1 {
            return Err(ErrorKind::PageSizeErr.into());
        }

        let mut orders = Vec::new();

        let mut page_total = 0;

        let state = self.best_state()?;

        let order_len_key = <xspot::OrderCountOf<Runtime>>::key_for(&who.unchecked_into());
        if let Some(len) = Self::pickout::<OrderIndex>(&state, &order_len_key)? {
            let mut total: u32 = 0;
            for i in (0..len).rev() {
                let order_key =
                    <xspot::OrderInfoOf<Runtime>>::key_for(&(who.clone().unchecked_into(), i));
                if let Some(order) = Self::pickout::<OrderInfo<Runtime>>(&state, &order_key)? {
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

    fn address(&self, who: AccountIdForRpc, chain: Chain) -> Result<Option<Vec<String>>> {
        let state = self.best_state()?;
        let mut v = vec![];
        let key = <xaccounts::CrossChainBindOf<Runtime>>::key_for(&(chain, who.unchecked_into()));
        match Self::pickout::<Vec<Vec<u8>>>(&state, &key)? {
            Some(addrs) => {
                for addr in addrs {
                    let a = Address::from_layout(&addr.as_slice()).unwrap_or_default();
                    v.push(a.to_string());
                }
                Ok(Some(v))
            }
            None => Ok(Some(v)),
        }
    }

    fn trustee_session_info(&self, chain: Chain) -> Result<Option<CurrentTrusteeSessionInfo>> {
        let state = self.best_state()?;
        let info_option = Self::current_trustee_session_info(&state, chain)?;
        Ok(info_option
            .map(|(info, session_number)| match chain {
                Chain::Bitcoin => {
                    let hot_addr_info: BtcTrusteeAddrInfo =
                        Decode::decode(&mut info.hot_address.as_slice()).unwrap_or_default();
                    let cold_addr_info: BtcTrusteeAddrInfo =
                        Decode::decode(&mut info.cold_address.as_slice()).unwrap_or_default();
                    let hot_addr = hot_addr_info.addr;
                    let hot_str = hot_addr.to_string();
                    let cold_address = cold_addr_info.addr;
                    let cold_str = cold_address.to_string();
                    Some(CurrentTrusteeSessionInfo {
                        session_number,
                        trustee_list: info
                            .trustee_list
                            .into_iter()
                            .map(|accountid| accountid.into())
                            .collect(),
                        hot_entity: hot_str,
                        cold_entity: cold_str,
                    })
                }
                _ => None,
            })
            .and_then(|result| result))
    }

    fn trustee_info_for_accountid(&self, who: AccountIdForRpc) -> Result<Vec<TrusteeInfo>> {
        let who: AccountId = who.unchecked_into();
        let state = self.best_state()?;
        let mut trustee_info = Vec::new();

        for chain in Chain::iterator() {
            let key =
                <xaccounts::TrusteeIntentionPropertiesOf<Runtime>>::key_for(&(who.clone(), *chain));

            if let Some(props) = Self::pickout::<TrusteeIntentionProps>(&state, &key)? {
                let hot_entity = match props.hot_entity {
                    TrusteeEntity::Bitcoin(pubkey) => pubkey.to_hex(),
                };
                let cold_entity = match props.cold_entity {
                    TrusteeEntity::Bitcoin(pubkey) => pubkey.to_hex(),
                };

                trustee_info.push(TrusteeInfo::new(*chain, hot_entity, cold_entity))
            }
        }

        Ok(trustee_info)
    }

    fn fee(&self, call_params: String, tx_length: u64) -> Result<Option<u64>> {
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
        let b = self.best_number()?;

        let transaction_fee: Result<Option<u64>> = self
            .client
            .runtime_api()
            .transaction_fee(&b, call.encode(), tx_length)
            .map_err(Into::into);
        let transaction_fee = transaction_fee?;
        Ok(transaction_fee)
    }

    fn withdraw_tx(&self, chain: Chain) -> Result<Option<WithdrawTxInfo>> {
        let state = self.best_state()?;
        let (trustee_session_info, _) =
            Self::current_trustee_session_info(&state, chain)?.unwrap_or_default();
        match chain {
            Chain::Bitcoin => {
                let hot_addr_info: BtcTrusteeAddrInfo =
                    Decode::decode(&mut trustee_session_info.hot_address.as_slice())
                        .unwrap_or_default();
                let key = <xbitcoin::CurrentWithdrawalProposal<Runtime>>::key();
                if let Some(proposal) =
                    Self::pickout::<xbitcoin::WithdrawalProposal<AccountId>>(&state, &key)?
                {
                    let script: String = hot_addr_info.redeem_script.to_hex();
                    Ok(Some(WithdrawTxInfo::new(proposal, script)))
                } else {
                    Ok(None)
                }
            }
            _ => Ok(None),
        }
    }

    fn mock_bitcoin_new_trustees(
        &self,
        candidates: Vec<AccountIdForRpc>,
    ) -> Result<Option<MockBitcoinTrustee>> {
        let b = self.best_number()?;

        let candidates: Vec<AccountId> = candidates
            .into_iter()
            .map(|a| a.unchecked_into())
            .collect::<Vec<_>>();

        // result is (Vec<(accountid, (hot pubkey, cold pubkey)), (required count, total count), hot_trustee_addr, cold_trustee_addr)>)
        // StdResult<(Vec<(AccountId, (Vec<u8>, Vec<u8>))>, (u32, u32), BtcTrusteeAddrInfo, BtcTrusteeAddrInfo), Vec<u8>>
        let runtime_result = self
            .client
            .runtime_api()
            .mock_bitcoin_new_trustees(&b, candidates)?;

        let mock = match runtime_result {
            Err(e) => return Err(ErrorKind::RuntimeErr(e).into()),
            Ok(item) => item.into(),
        };

        Ok(Some(mock))
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
