// Copyright 2019 Chainpool.

use self::types::Revocation;
use super::*;
use runtime_primitives::traits::{Header, ProvideRuntimeApi};
use std::iter::FromIterator;
use std::str::FromStr;
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
        Metadata<Block> + XAssetsApi<Block> + XMiningApi<Block> + XSpotApi<Block>,
{
    fn block_info(&self, number: Trailing<NumberFor<Block>>) -> Result<Option<SignedBlock<Block>>> {
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
                is_native: true,
                details: map,
            })
            .collect();
        into_pagedata(final_result, page_index, page_size)
    }

    fn assets(&self, page_index: u32, page_size: u32) -> Result<Option<PageData<TotalAssetInfo>>> {
        let b = self.best_number()?;
        let tokens: Result<Vec<Token>> = self
            .client
            .runtime_api()
            .valid_assets(&b)
            .map_err(|e| e.into());
        let tokens = tokens?;

        let state = self.best_state()?;

        let mut assets = Vec::new();

        for token in tokens.into_iter() {
            let mut bmap = BTreeMap::<AssetType, Balance>::from_iter(
                xassets::AssetType::iterator().map(|t| (*t, Zero::zero())),
            );

            let key = <xassets::TotalAssetBalance<Runtime>>::key_for(token.as_ref());
            if let Some(info) = Self::pickout::<CodecBTreeMap<AssetType, Balance>>(&state, &key)? {
                bmap.extend(info.0.iter());
            }

            let asset = match Self::get_asset(&state, &token)? {
                Some(info) => info,
                None => unreachable!("should not reach this branch, the token info must be exists"),
            };

            // PCX free balance
            if token.as_slice() == xassets::Module::<Runtime>::TOKEN {
                let key = <balances::TotalIssuance<Runtime>>::key();
                let total_issue = Self::pickout::<Balance>(&state, &key)?.unwrap_or(Zero::zero());
                let other_total: Balance = bmap
                    .iter()
                    .filter(|(&k, _)| k != AssetType::Free)
                    .fold(Zero::zero(), |acc, (_, v)| acc + *v);
                let free_issue = total_issue - other_total;
                bmap.insert(xassets::AssetType::Free, free_issue);
            }
            let mut trustee_addr = String::default();
            if token.as_slice() == xbitcoin::Module::<Runtime>::TOKEN {
                let key = <TrusteeAddress<Runtime>>::key();
                let trustee = match Self::pickout::<keys::Address>(&state, &key)? {
                    Some(a) => a,
                    None => unreachable!(
                        "should not reach this branch, the trustee address info must be exists"
                    ),
                };
                trustee_addr = trustee.to_string();
            }

            assets.push(TotalAssetInfo::new(
                asset,
                trustee_addr,
                CodecBTreeMap(bmap),
            ));
        }

        into_pagedata(assets, page_index, page_size)
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
        page_index: u32,
        page_size: u32,
    ) -> Result<Option<PageData<ApplicationWrapper>>> {
        let mut v = vec![];
        let best_number = self.best_number()?;
        let state = self.best_state()?;
        for c in xassets::Chain::iterator() {
            let list: Result<Vec<xrecords::Application<AccountId, Balance, Timestamp>>> = self
                .client
                .runtime_api()
                .withdrawal_list_of(&best_number, *c)
                .map_err(|e| e.into());

            let list = list?;
            let applications = Self::get_applications_with_state(&state, list)?;
            v.extend(applications);
        }

        into_pagedata(v, page_index, page_size)
    }

    fn withdrawal_list_of(
        &self,
        who: AccountId,
        page_index: u32,
        page_size: u32,
    ) -> Result<Option<PageData<ApplicationWrapper>>> {
        let mut v = vec![];
        let b = self.best_number()?;
        for c in xassets::Chain::iterator() {
            let list: Result<Vec<xrecords::Application<AccountId, Balance, Timestamp>>> = self
                .client
                .runtime_api()
                .withdrawal_list_of(&b, *c)
                .map_err(|e| e.into());
            let list = list?;
            v.extend(list);
        }

        //        let mut handle = BTreeMap::<Chain, Vec<u32>>::new();
        //        // btc
        //        let key = xbitcoin::TxProposal::<Runtime>::key();
        //        let ids = match Self::pickout::<xbitcoin::CandidateTx>(&state, &key)? {
        //            Some(candidate_tx) => candidate_tx.outs,
        //            None => vec![],
        //        };
        //        handle.insert(Chain::Bitcoin, ids);

        let v = v.into_iter().filter(|r| r.applicant() == who).collect();

        let state = self.best_state()?;
        let applications = Self::get_applications_with_state(&state, v)?;

        into_pagedata(applications, page_index, page_size)
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
        let validators =
            Self::pickout::<Vec<AccountId>>(&state, &key)?.expect("Validators can't be empty");

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
                    //                    info.jackpot = vote_weight.jackpot;
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
                    info.on_line = pair.on_line;
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

    fn deposit_records(
        &self,
        who: AccountId,
        page_index: u32,
        page_size: u32,
    ) -> Result<Option<PageData<DepositInfo>>> {
        let state = self.best_state()?;
        let mut records = Vec::new();
        let key = <IrrBlock<Runtime>>::key();
        let irr_count = if let Some(irr) = Self::pickout::<u32>(&state, &key)? {
            irr
        } else {
            return into_pagedata(records, page_index, page_size);
        };
        let key = <AccountMap<Runtime>>::key_for(&who);
        let bind_list = if let Some(list) = Self::pickout::<Vec<Address>>(&state, &key)? {
            list
        } else {
            return into_pagedata(records, page_index, page_size);
        };
        let key = <TrusteeAddress<Runtime>>::key();
        let trustee = if let Some(a) = Self::pickout::<keys::Address>(&state, &key)? {
            a
        } else {
            return into_pagedata(records, page_index, page_size);
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
                            match get_deposit_info(&info.raw_tx, who, &info, &trustee, &bind_list) {
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
                                        remarks: dep.1,
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

    fn account(&self, btc_addr: String) -> Result<Option<AccountId>> {
        let state = self.best_state()?;
        let addr = match Address::from_str(btc_addr.as_str()) {
            Ok(a) => a,
            Err(_) => return Ok(None),
        };
        let key = <xbitcoin::AddressMap<Runtime>>::key_for(&addr);
        match Self::pickout::<AccountId>(&state, &key)? {
            Some(a) => Ok(Some(a)),
            None => Ok(None),
        }
    }

    fn address(&self, account: AccountId) -> Result<Option<Vec<String>>> {
        let state = self.best_state()?;
        let mut v = vec![];
        let key = <xbitcoin::AccountMap<Runtime>>::key_for(&account);
        match Self::pickout::<Vec<Address>>(&state, &key)? {
            Some(addrs) => {
                for a in addrs {
                    v.push(a.to_string());
                }
                Ok(Some(v))
            }
            None => Ok(Some(v)),
        }
    }
}

fn get_deposit_info(
    raw_tx: &BTCTransaction,
    who: AccountId,
    info: &TxInfo,
    trustee: &Address,
    bind_list: &[Address],
) -> Option<(Balance, String)> {
    let mut ops = String::new();
    let mut balance = 0;
    let mut flag = bind_list.iter().any(|a| a.hash == info.input_address.hash);
    for output in raw_tx.outputs.iter() {
        let script = &output.script_pubkey;
        let into_script: Script = script.clone().into();
        let s: Script = script.clone().into();
        if into_script.is_null_data_script() {
            let data = s.extract_rear(':');
            match from(data.to_vec()) {
                Ok(mut slice) => {
                    let account_id: H256 = Decode::decode(&mut slice[1..33].to_vec().as_slice())
                        .unwrap_or_else(|| [0; 32].into());
                    if account_id == who || flag {
                        flag = true;
                        ops = String::from_utf8(s[2..].to_vec()).unwrap_or_default();
                    }
                }
                Err(_) => {
                    if flag {
                        ops = String::from_utf8(s[2..].to_vec()).unwrap_or_default();
                    }
                }
            }

            continue;
        }

        // get deposit money
        let script_addresses = into_script.extract_destinations().unwrap_or_default();
        if script_addresses.len() == 1 {
            if (trustee.hash == script_addresses[0].hash) && (output.value > 0) && flag {
                balance += output.value;
            }
        }
    }

    if flag {
        Some((balance, ops))
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
