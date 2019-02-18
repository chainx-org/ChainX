// Copyright 2019 Chainpool.

extern crate hex;

use self::types::Revocation;
use super::*;
use keys::DisplayLayout;
use runtime_primitives::traits::{Header, ProvideRuntimeApi};
use srml_support::storage::generator::{StorageMap, StorageValue};
use std::iter::FromIterator;
use xassets::ChainT;

impl<B, E, Block, RA>
    ChainXApi<NumberFor<Block>, AccountId, Balance, BlockNumber, SignedBlock<Block>>
    for ChainX<B, E, Block, RA>
where
    B: client::backend::Backend<Block, Blake2Hasher> + Send + Sync + 'static,
    E: client::CallExecutor<Block, Blake2Hasher> + Clone + Send + Sync + 'static,
    Block: BlockT<Hash = H256> + 'static,
    RA: std::marker::Send + std::marker::Sync + 'static,
    Client<B, E, Block, RA>: ProvideRuntimeApi,
    <Client<B, E, Block, RA> as ProvideRuntimeApi>::Api:
        Metadata<Block> + XAssetsApi<Block> + XMiningApi<Block> + XSpotApi<Block> + XFeeApi<Block>,
{
    fn block_info(&self, number: Option<NumberFor<Block>>) -> Result<Option<SignedBlock<Block>>> {
        let hash = match number.into() {
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
        who: AccountId,
        page_index: u32,
        page_size: u32,
    ) -> Result<Option<PageData<AssetInfo>>> {
        let b = self.best_number()?;
        let assets: Result<Vec<(Token, CodecBTreeMap<AssetType, Balance>)>> = self
            .client
            .runtime_api()
            .valid_assets_of(&b, who)
            .map_err(|e| e.into());

        let assets = assets?;
        let final_result = assets
            .into_iter()
            .map(|(token, map)| AssetInfo {
                name: String::from_utf8_lossy(&token).into_owned(),
                details: map,
            })
            .collect();
        into_pagedata(final_result, page_index, page_size)
    }

    fn assets(&self, page_index: u32, page_size: u32) -> Result<Option<PageData<TotalAssetInfo>>> {
        let b = self.best_number()?;
        let assets: Result<Vec<(Asset, bool)>> = self
            .client
            .runtime_api()
            .all_assets(&b)
            .map_err(|e| e.into());
        let assets = assets?;

        let state = self.best_state()?;

        let mut all_assets = Vec::new();

        for (asset, valid) in assets.into_iter() {
            let mut bmap = BTreeMap::<AssetType, Balance>::from_iter(
                xassets::AssetType::iterator().map(|t| (*t, Zero::zero())),
            );

            let key = <xassets::TotalAssetBalance<Runtime>>::key_for(asset.token().as_ref());
            if let Some(info) = Self::pickout::<CodecBTreeMap<AssetType, Balance>>(&state, &key)? {
                bmap.extend(info.0.iter());
            }
            // PCX free balance
            if asset.token().as_slice() == xassets::Module::<Runtime>::TOKEN {
                let key = <balances::TotalIssuance<Runtime>>::key();
                let total_issue = Self::pickout::<Balance>(&state, &key)?.unwrap_or(Zero::zero());
                let other_total: Balance = bmap
                    .iter()
                    .filter(|(&k, _)| k != AssetType::Free)
                    .fold(Zero::zero(), |acc, (_, v)| acc + *v);
                let free_issue = total_issue - other_total;
                bmap.insert(xassets::AssetType::Free, free_issue);
            }

            all_assets.push(TotalAssetInfo::new(asset, valid, CodecBTreeMap(bmap)));
        }

        into_pagedata(all_assets, page_index, page_size)
    }

    fn verify_addr(&self, token: String, addr: String, memo: String) -> Result<Option<String>> {
        let token: xassets::Token = token.as_bytes().to_vec();
        let addr: xrecords::AddrStr = addr.as_bytes().to_vec();
        let memo: xassets::Memo = memo.as_bytes().to_vec();

        // test valid before call runtime api
        if let Err(e) = xassets::is_valid_token(&token) {
            return Ok(Some(String::from_utf8_lossy(e.as_ref()).into_owned()));
        }

        if addr.len() > 256 || memo.len() > 256 {
            return Ok(Some("the addr or memo may be too long".to_string()));
        }

        let b = self.best_number()?;
        self.client
            .runtime_api()
            .verify_address(&b, token, addr, memo)
            .and_then(|r| match r {
                Ok(()) => Ok(None),
                Err(s) => Ok(Some(String::from_utf8_lossy(s.as_ref()).into_owned())),
            })
            .map_err(|e| e.into())
    }

    fn minimal_withdrawal_value(&self, token: String) -> Result<Option<Balance>> {
        let token: xassets::Token = token.as_bytes().to_vec();

        // test valid before call runtime api
        if let Err(_) = xassets::is_valid_token(&token) {
            return Ok(None);
        }
        let b = self.best_number()?;
        self.client
            .runtime_api()
            .minimal_withdrawal_value(&b, token)
            .map_err(|e| e.into())
    }

    fn withdrawal_list(
        &self,
        chain: Chain,
        page_index: u32,
        page_size: u32,
    ) -> Result<Option<PageData<WithdrawInfo>>> {
        let best_number = self.best_number()?;
        let state = self.best_state()?;
        let mut records = Vec::new();
        let key = <xaccounts::TrusteeAddress<Runtime>>::key_for(&chain);
        let trustee = match Self::pickout::<xaccounts::TrusteeAddressPair>(&state, &key)? {
            Some(a) => {
                let hot_address = Address::from_layout(&a.hot_address.as_slice())
                    .map_err(|_| "Invalid Address")?;
                hot_address
            }
            None => return into_pagedata(records, page_index, page_size),
        };
        let list: Vec<xrecords::Application<AccountId, Balance, Timestamp>> = self
            .client
            .runtime_api()
            .withdrawal_list_of(&best_number, chain)
            .unwrap_or_default();

        match chain {
            Chain::Bitcoin => {
                let key = <TxProposal<Runtime>>::key();
                let candidate =
                    if let Some(data) = Self::pickout::<CandidateTx<AccountId>>(&state, &key)? {
                        data
                    } else {
                        for appl in list {
                            let info = WithdrawInfo::new(
                                appl.time(),
                                appl.id(),
                                String::new(),
                                appl.balance(),
                                String::from_utf8(appl.token()).unwrap_or_default(),
                                appl.applicant(),
                                String::from_utf8(appl.addr()).unwrap_or_default(),
                                WithdrawStatus::Applying,
                                String::from_utf8(appl.ext()).unwrap_or_default(),
                            );
                            records.push(info)
                        }
                        return into_pagedata(records, page_index, page_size);
                    };

                let key = <IrrBlock<Runtime>>::key();
                let irr_count = if let Some(irr) = Self::pickout::<u32>(&state, &key)? {
                    irr
                } else {
                    for appl in list {
                        let info = WithdrawInfo::new(
                            appl.time(),
                            appl.id(),
                            String::new(),
                            appl.balance(),
                            String::from_utf8(appl.token()).unwrap_or_default(),
                            appl.applicant(),
                            String::from_utf8(appl.addr()).unwrap_or_default(),
                            WithdrawStatus::Applying,
                            String::from_utf8(appl.ext()).unwrap_or_default(),
                        );
                        records.push(info)
                    }
                    return into_pagedata(records, page_index, page_size);
                };

                let key = <BestIndex<Runtime>>::key();
                let mut tx_hash = String::new();
                if let Some(best_hash) = Self::pickout::<btc_chain::hash::H256>(&state, &key)? {
                    let mut block_hash = best_hash;
                    for _i in 0..irr_count {
                        let key = <BlockHeaderFor<Runtime>>::key_for(&block_hash);
                        if let Some(header_info) = Self::pickout::<BlockHeaderInfo>(&state, &key)? {
                            for txid in header_info.txid {
                                let key = <TxFor<Runtime>>::key_for(&txid);
                                if let Some(info) = Self::pickout::<TxInfo>(&state, &key)? {
                                    if info.input_address.hash != trustee.hash {
                                        continue;
                                    }
                                    tx_hash = info.raw_tx.hash().to_string();
                                }
                            }
                            block_hash = header_info.header.previous_header_hash;
                        }
                    }
                }

                for appl in list {
                    let mut status = WithdrawStatus::Applying;
                    if candidate.withdraw_id.iter().any(|id| *id == appl.id()) {
                        match candidate.sig_status {
                            VoteResult::Unfinish => status = WithdrawStatus::Signing,
                            VoteResult::FinishWithFavor => status = WithdrawStatus::Processing,
                            _ => status = WithdrawStatus::Applying,
                        }
                    }
                    let info = WithdrawInfo::new(
                        appl.time(),
                        appl.id(),
                        tx_hash.clone(),
                        appl.balance(),
                        String::from_utf8(appl.token()).unwrap_or_default(),
                        appl.applicant(),
                        String::from_utf8(appl.addr()).unwrap_or_default(),
                        status,
                        String::from_utf8(appl.ext()).unwrap_or_default(),
                    );
                    records.push(info)
                }
                into_pagedata(records, page_index, page_size)
            }
            _ => Ok(None),
        }
    }

    fn nomination_records(
        &self,
        who: AccountId,
    ) -> Result<Option<Vec<(AccountId, NominationRecord)>>> {
        let state = self.best_state()?;

        let mut records = Vec::new();

        let key = <xstaking::Intentions<Runtime>>::key();
        if let Some(intentions) = Self::pickout::<Vec<AccountId>>(&state, &key)? {
            for intention in intentions {
                let key = <xstaking::NominationRecords<Runtime>>::key_for(&(who, intention));
                if let Some(record) =
                    Self::pickout::<xstaking::NominationRecord<Balance, BlockNumber>>(&state, &key)?
                {
                    let revocations = record
                        .revocations
                        .iter()
                        .map(|x| Revocation {
                            block_numer: x.0,
                            value: x.1,
                        })
                        .collect::<Vec<_>>();
                    records.push((
                        intention,
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

        let key = <session::Validators<Runtime>>::key();
        let validators = Self::pickout::<Vec<(AccountId, u64)>>(&state, &key)?
            .expect("Validators can't be empty");
        let validators: Vec<AccountId> = validators.into_iter().map(|(who, _)| who).collect();

        let key = <xaccounts::TrusteeIntentions<Runtime>>::key();

        // FIXME trustees should not empty
        let trustees = Self::pickout::<Vec<AccountId>>(&state, &key)?.unwrap_or_default();

        let key = <xstaking::Intentions<Runtime>>::key();

        if let Some(intentions) = Self::pickout::<Vec<AccountId>>(&state, &key)? {
            let jackpot_addr_list: Result<Vec<H256>> = self
                .client
                .runtime_api()
                .multi_jackpot_accountid_for(&self.best_number()?, intentions.clone())
                .map_err(|e| e.into());
            let jackpot_addr_list: Vec<H256> = jackpot_addr_list?;

            for (intention, jackpot_addr) in intentions.into_iter().zip(jackpot_addr_list) {
                let mut info = IntentionInfo::default();

                let key = <xaccounts::IntentionNameOf<Runtime>>::key_for(&intention);
                if let Some(name) = Self::pickout::<xaccounts::Name>(&state, &key)? {
                    info.name = String::from_utf8_lossy(&name).into_owned();
                }

                let key = <xaccounts::IntentionPropertiesOf<Runtime>>::key_for(&intention);
                if let Some(props) = Self::pickout::<IntentionProps>(&state, &key)? {
                    info.url = String::from_utf8_lossy(&props.url).into_owned();
                    info.is_active = props.is_active;
                    info.about = String::from_utf8_lossy(&props.about).into_owned();
                }

                let key = <xstaking::IntentionProfiles<Runtime>>::key_for(&intention);
                if let Some(profs) =
                    Self::pickout::<IntentionProfs<Balance, BlockNumber>>(&state, &key)?
                {
                    let free = <balances::FreeBalance<Runtime>>::key_for(&jackpot_addr);
                    info.jackpot = Self::pickout::<Balance>(&state, &free)?.unwrap_or_default();
                    info.jackpot_address = jackpot_addr;
                    info.total_nomination = profs.total_nomination;
                    info.last_total_vote_weight = profs.last_total_vote_weight;
                    info.last_total_vote_weight_update = profs.last_total_vote_weight_update;
                }

                let key = <xstaking::NominationRecords<Runtime>>::key_for(&(intention, intention));
                if let Some(record) =
                    Self::pickout::<xstaking::NominationRecord<Balance, BlockNumber>>(&state, &key)?
                {
                    info.self_vote = record.nomination;
                }

                info.is_validator = validators.iter().any(|&i| i == intention);
                info.is_trustee = trustees.iter().any(|&i| i == intention);
                info.account = intention;

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
            let jackpot_addr_list: Result<Vec<H256>> = self
                .client
                .runtime_api()
                .multi_token_jackpot_accountid_for(&self.best_number()?, tokens.clone())
                .map_err(|e| e.into());
            let jackpot_addr_list: Vec<H256> = jackpot_addr_list?;

            for (token, jackpot_addr) in tokens.into_iter().zip(jackpot_addr_list) {
                let mut info = PseduIntentionInfo::default();

                let key = <xtokens::PseduIntentionProfiles<Runtime>>::key_for(&token);
                if let Some(vote_weight) =
                    Self::pickout::<PseduIntentionVoteWeight<Balance>>(&state, &key)?
                {
                    let free = <balances::FreeBalance<Runtime>>::key_for(&jackpot_addr);
                    info.jackpot = Self::pickout::<Balance>(&state, &free)?.unwrap_or_default();
                    info.jackpot_address = jackpot_addr;
                    info.last_total_deposit_weight = vote_weight.last_total_deposit_weight;
                    info.last_total_deposit_weight_update =
                        vote_weight.last_total_deposit_weight_update;
                }

                let b = self.best_number()?;
                if let Some(Some(price)) = self
                    .client
                    .runtime_api()
                    .aver_asset_price(&b, token.clone())
                    .ok()
                {
                    info.price = price;
                    //注意
                    //这里返回的是以PCX计价的"单位"token的价格，已含pcx精度
                    //譬如1BTC=10000PCX，则返回的是10000*（10.pow(pcx精度))
                    //因此，如果前端要换算折合投票数的时候
                    //应该=(资产数量[含精度的数字]*price)/(10^资产精度)=PCX[含PCX精度]
                };

                let key = <xassets::TotalAssetBalance<Runtime>>::key_for(&token);
                if let Some(total_asset_balance) =
                    Self::pickout::<CodecBTreeMap<AssetType, Balance>>(&state, &key)?
                {
                    info.circulation = total_asset_balance
                        .0
                        .iter()
                        .fold(Zero::zero(), |acc, (_, v)| acc + *v);
                }

                info.id = String::from_utf8_lossy(&token).into_owned();
                psedu_intentions.push(info);
            }
        }

        Ok(Some(psedu_intentions))
    }

    fn trustee_info(&self, who: AccountId) -> Result<Vec<TrusteeInfo>> {
        let state = self.best_state()?;
        let mut trustee_info = Vec::new();

        for chain in Chain::iterator() {
            let key = <xaccounts::TrusteeIntentionPropertiesOf<Runtime>>::key_for(&(who, *chain));

            if let Some(props) = Self::pickout::<TrusteeIntentionProps>(&state, &key)? {
                let hot_entity = match props.hot_entity {
                    TrusteeEntity::Bitcoin(pubkey) => hex::encode(&pubkey),
                };
                let cold_entity = match props.cold_entity {
                    TrusteeEntity::Bitcoin(pubkey) => hex::encode(&pubkey),
                };

                trustee_info.push(TrusteeInfo::new(chain.clone(), hot_entity, cold_entity))
            }
        }

        Ok(trustee_info)
    }

    fn psedu_nomination_records(
        &self,
        who: AccountId,
    ) -> Result<Option<Vec<PseduNominationRecord>>> {
        let state = self.best_state()?;
        let mut psedu_records = Vec::new();

        let key = <xtokens::PseduIntentions<Runtime>>::key();
        if let Some(tokens) = Self::pickout::<Vec<Token>>(&state, &key)? {
            for token in tokens {
                let mut record = PseduNominationRecord::default();

                let key = <xtokens::DepositRecords<Runtime>>::key_for(&(who, token.clone()));
                if let Some(vote_weight) =
                    Self::pickout::<DepositVoteWeight<BlockNumber>>(&state, &key)?
                {
                    record.last_total_deposit_weight = vote_weight.last_deposit_weight;
                    record.last_total_deposit_weight_update =
                        vote_weight.last_deposit_weight_update;
                }

                let key = <xassets::AssetBalance<Runtime>>::key_for(&(who, token.clone()));

                if let Some(balances) =
                    Self::pickout::<CodecBTreeMap<AssetType, Balance>>(&state, &key)?
                {
                    record.balance = balances.0.iter().fold(Zero::zero(), |acc, (_, v)| acc + *v);
                }

                record.id = String::from_utf8_lossy(&token).into_owned();

                psedu_records.push(record);
            }
        }

        Ok(Some(psedu_records))
    }

    fn order_pairs(&self) -> Result<Option<Vec<(PairInfo)>>> {
        let mut pairs = Vec::new();
        let state = self.best_state()?;

        let len_key = <xspot::OrderPairLen<Runtime>>::key();
        if let Some(len) = Self::pickout::<OrderPairID>(&state, &len_key)? {
            for i in 0..len {
                let key = <xspot::OrderPairOf<Runtime>>::key_for(&i);
                if let Some(pair) = Self::pickout::<OrderPair>(&state, &key)? {
                    let mut info = PairInfo::default();
                    info.id = pair.id;
                    info.assets = String::from_utf8_lossy(&pair.first).into_owned();
                    info.currency = String::from_utf8_lossy(&pair.second).into_owned();
                    info.precision = pair.precision;
                    info.online = pair.online;
                    info.unit_precision = pair.unit_precision;

                    let price_key = <xspot::OrderPairPriceOf<Runtime>>::key_for(&i);
                    if let Some(price) =
                        Self::pickout::<(Balance, Balance, BlockNumber)>(&state, &price_key)?
                    {
                        info.last_price = price.0;
                        info.aver_price = price.1;
                        info.update_height = price.2;
                    }

                    let handicap_key = <xspot::HandicapMap<Runtime>>::key_for(&i);
                    if let Some(handicap) =
                        Self::pickout::<HandicapT<Runtime>>(&state, &handicap_key)?
                    {
                        info.buy_one = handicap.buy;
                        info.sell_one = handicap.sell;
                    }

                    pairs.push(info);
                }
            }
        }

        Ok(Some(pairs))
    }
    fn quotationss(&self, id: OrderPairID, piece: u32) -> Result<Option<QuotationsList>> {
        if piece < 1 || piece > 10 {
            return Err(QuotationssPieceErr.into());
        }
        let mut quotationslist = QuotationsList::default();
        quotationslist.id = id;
        quotationslist.piece = piece;
        quotationslist.sell = Vec::new();
        quotationslist.buy = Vec::new();

        let state = self.best_state()?;
        let pair_key = <xspot::OrderPairOf<Runtime>>::key_for(&id);
        if let Some(pair) = Self::pickout::<OrderPair>(&state, &pair_key)? {
            let min_unit = 10_u64.pow(pair.unit_precision);

            //盘口
            let handicap_key = <xspot::HandicapMap<Runtime>>::key_for(&id);

            if let Some(handicap) = Self::pickout::<HandicapT<Runtime>>(&state, &handicap_key)? {
                //先买档
                let mut opponent_price: Balance = handicap.buy;
                let mut price_volatility: Balance = 10;

                let price_volatility_key = <xspot::PriceVolatility<Runtime>>::key();
                if let Some(p) = Self::pickout::<u32>(&state, &price_volatility_key)? {
                    price_volatility = p.into();
                }

                let max_price: Balance =
                    (handicap.sell * (100_u64 + price_volatility as Balance)) / 100_u64;
                let min_price: Balance =
                    (handicap.sell * (100_u64 - price_volatility as Balance)) / 100_u64;
                let mut n = 0;

                loop {
                    if n > piece || opponent_price == 0 || opponent_price < min_price {
                        break;
                    }

                    let quotations_key =
                        <xspot::Quotations<Runtime>>::key_for(&(id, opponent_price));
                    if let Some(list) =
                        Self::pickout::<Vec<(AccountId, ID)>>(&state, &quotations_key)?
                    {
                        let mut sum: Balance = 0;
                        for item in &list {
                            let order_key = <xspot::AccountOrder<Runtime>>::key_for(item);
                            if let Some(order) =
                                Self::pickout::<OrderT<Runtime>>(&state, &order_key)?
                            {
                                sum += match order.amount.checked_sub(order.hasfill_amount) {
                                    Some(v) => v,
                                    None => Default::default(),
                                };
                            }
                        }
                        quotationslist.buy.push((opponent_price, sum));
                        n += 1;
                    };

                    opponent_price = match opponent_price.checked_sub(As::sa(min_unit)) {
                        Some(v) => v,
                        None => Default::default(),
                    };
                }
                //再卖档
                opponent_price = handicap.sell;
                n = 0;
                loop {
                    if n > piece || opponent_price == 0 || opponent_price > max_price {
                        break;
                    }

                    let quotations_key =
                        <xspot::Quotations<Runtime>>::key_for(&(id, opponent_price));
                    if let Some(list) =
                        Self::pickout::<Vec<(AccountId, ID)>>(&state, &quotations_key)?
                    {
                        let mut sum: Balance = 0;
                        for item in &list {
                            let order_key = <xspot::AccountOrder<Runtime>>::key_for(item);
                            if let Some(order) =
                                Self::pickout::<OrderT<Runtime>>(&state, &order_key)?
                            {
                                sum += match order.amount.checked_sub(order.hasfill_amount) {
                                    Some(v) => v,
                                    None => Default::default(),
                                };
                            }
                        }
                        quotationslist.sell.push((opponent_price, sum));
                        n += 1;
                    };

                    opponent_price = match opponent_price.checked_add(As::sa(min_unit)) {
                        Some(v) => v,
                        None => Default::default(),
                    };
                }
            };
        } else {
            return Err(OrderPairIDErr.into());
        }

        Ok(Some(quotationslist))
    }

    fn orders(
        &self,
        who: AccountId,
        page_index: u32,
        page_size: u32,
    ) -> Result<Option<PageData<OrderT<Runtime>>>> {
        if page_size > MAX_PAGE_SIZE || page_size < 1 {
            return Err(PageSizeErr.into());
        }

        let mut orders = Vec::new();

        let mut page_total = 0;

        let state = self.best_state()?;

        let order_len_key = <xspot::AccountOrdersLen<Runtime>>::key_for(&who);
        if let Some(len) = Self::pickout::<ID>(&state, &order_len_key)? {
            let mut total: u32 = 0;
            for i in (0..len).rev() {
                let order_key = <xspot::AccountOrder<Runtime>>::key_for(&(who, i));
                if let Some(order) = Self::pickout::<OrderT<Runtime>>(&state, &order_key)? {
                    if total >= page_index * page_size && total < ((page_index + 1) * page_size) {
                        orders.push(order.clone());
                    }
                    total += 1;
                }
            }

            let total_page: u32 = (total + (page_size - 1)) / page_size;

            page_total = total_page;

            if page_index >= total_page && total_page > 0 {
                return Err(PageIndexErr.into());
            }
        }
        let d = PageData {
            page_total,
            page_index,
            page_size,
            data: orders,
        };

        Ok(Some(d))
    }

    fn deposit_list(
        &self,
        chain: Chain,
        page_index: u32,
        page_size: u32,
    ) -> Result<Option<PageData<DepositInfo>>> {
        let state = self.best_state()?;
        let mut records = Vec::new();
        let key = <xaccounts::TrusteeAddress<Runtime>>::key_for(&chain);
        let trustee = match Self::pickout::<xaccounts::TrusteeAddressPair>(&state, &key)? {
            Some(a) => {
                let hot_address = Address::from_layout(&a.hot_address.as_slice())
                    .map_err(|_| "Invalid Address")?;
                hot_address
            }
            None => return into_pagedata(records, page_index, page_size),
        };

        match chain {
            Chain::Bitcoin => {
                let key = <IrrBlock<Runtime>>::key();
                let irr_count = if let Some(irr) = Self::pickout::<u32>(&state, &key)? {
                    irr
                } else {
                    return Ok(None);
                };
                let token = xbitcoin::Module::<Runtime>::TOKEN;
                let stoken = String::from_utf8_lossy(&token).into_owned();
                let key = <BestIndex<Runtime>>::key();
                if let Some(best_hash) = Self::pickout::<btc_chain::hash::H256>(&state, &key)? {
                    let mut block_hash = best_hash;
                    for i in 0..irr_count {
                        let key = <BlockHeaderFor<Runtime>>::key_for(&block_hash);
                        if let Some(header_info) = Self::pickout::<BlockHeaderInfo>(&state, &key)? {
                            for txid in header_info.txid {
                                let key = <TxFor<Runtime>>::key_for(&txid);
                                if let Some(info) = Self::pickout::<TxInfo>(&state, &key)? {
                                    match get_btc_deposit_info(&info.raw_tx, &trustee) {
                                        Some(dep) => {
                                            let tx_hash = txid.to_string();
                                            let btc_address = info.input_address.to_string();
                                            let info = DepositInfo {
                                                time: header_info.header.time,
                                                txid: tx_hash,
                                                confirm: i,
                                                total_confirm: irr_count,
                                                address: btc_address,
                                                balance: dep.0,
                                                token: stoken.clone(),
                                                accountid: dep.1,
                                                memo: dep.2,
                                            };
                                            records.push(info);
                                        }
                                        None => continue,
                                    }
                                }
                            }
                            block_hash = header_info.header.previous_header_hash;
                        }
                    }
                }
                into_pagedata(records, page_index, page_size)
            }
            _ => return Ok(None),
        }
    }

    fn address(&self, who: AccountId, chain: Chain) -> Result<Option<Vec<String>>> {
        let state = self.best_state()?;
        let mut v = vec![];
        let key = <xaccounts::CrossChainBindOf<Runtime>>::key_for(&(chain, who));
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

    fn fee(&self, call_params: String, tx_length: u64) -> Result<Option<u64>> {
        use codec::Encode;

        if !call_params.starts_with("0x") {
            return Err(BinanryStartErr.into());
        }
        let call_params = if let Ok(hex_call) = hex::decode(&call_params[2..]) {
            hex_call
        } else {
            return Err(HexDecodeErr.into());
        };
        let call: Call = if let Some(call) = Decode::decode(&mut call_params.as_slice()) {
            call
        } else {
            return Err(DecodeErr.into());
        };
        let b = self.best_number()?;

        let transaction_fee: Result<Option<u64>> = self
            .client
            .runtime_api()
            .transaction_fee(&b, call.encode(), tx_length)
            .map_err(|e| e.into());
        let transaction_fee = transaction_fee?;
        Ok(transaction_fee)
    }
}

fn get_btc_deposit_info(
    raw_tx: &BTCTransaction,
    trustee: &Address,
) -> Option<(Balance, Option<AccountId>, String)> {
    let mut ops = String::default();
    let mut accountid = AccountId::default();
    let mut balance = 0;
    let mut bind_flag = true;
    let mut deposit_flag = false;
    for output in raw_tx.outputs.iter() {
        let script = &output.script_pubkey;
        let into_script: Script = script.clone().into();
        let s: Script = script.clone().into();
        if into_script.is_null_data_script() {
            let data = s.extract_pre('@');
            let mut account: Vec<u8> = match b58::from(data[2..].to_vec()) {
                Ok(a) => a,
                Err(_) => {
                    bind_flag = false;
                    Vec::new()
                }
            };

            if account.len() != 35 {
                bind_flag = false
            }

            if bind_flag {
                accountid =
                    Decode::decode(&mut account[1..33].to_vec().as_slice()).unwrap_or_default();
                ops = String::from_utf8(s[2..].to_vec()).unwrap_or_default();
            }
            continue;
        }

        // get deposit money
        let script_addresses = into_script.extract_destinations().unwrap_or_default();
        if script_addresses.len() == 1 && trustee.hash == script_addresses[0].hash {
            deposit_flag = true;
            if output.value > 0 {
                balance += output.value;
            }
        }
    }

    if deposit_flag {
        if bind_flag {
            Some((balance, Some(accountid), ops))
        } else {
            Some((balance, None, ops))
        }
    } else {
        None
    }
}

fn into_pagedata<T>(src: Vec<T>, page_index: u32, page_size: u32) -> Result<Option<PageData<T>>> {
    if page_size == 0 {
        return Err(PageSizeErr.into());
    }

    let page_total = (src.len() as u32 + (page_size - 1)) / page_size;
    if page_index >= page_total && page_total > 0 {
        return Err(PageIndexErr.into());
    }

    let mut list = vec![];
    for (index, item) in src.into_iter().enumerate() {
        let index = index as u32;
        if index >= page_index * page_size && index < ((page_index + 1) * page_size) {
            list.push(item);
        }
    }

    let d = PageData {
        page_total,
        page_index,
        page_size,
        data: list,
    };

    Ok(Some(d))
}
