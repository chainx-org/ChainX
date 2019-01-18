use super::*;

impl<B, E, Block, RA>
    ChainXApi<NumberFor<Block>, AccountId, Balance, BlockNumber, SignedBlock<Block>>
    for ChainX<B, E, Block, RA>
where
    B: client::backend::Backend<Block, Blake2Hasher> + Send + Sync + 'static,
    E: client::CallExecutor<Block, Blake2Hasher> + Clone + Send + Sync + 'static,
    Block: BlockT<Hash = H256> + 'static,
    RA: Metadata<Block> + XAssetsApi<Block> + XMiningApi<Block>,
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

    fn cert(&self, owner: AccountId) -> Result<Option<Vec<CertInfo>>> {
        let state = self.best_state()?;

        let key = <xaccounts::CertNamesOf<Runtime>>::key_for(&owner);
        let names: Vec<Vec<u8>> = if let Some(names) = Self::pickout(&state, &key)? {
            names
        } else {
            return Ok(None);
        };

        let mut certs = Vec::new();
        for name in names.iter() {
            let key = <xaccounts::CertImmutablePropertiesOf<Runtime>>::key_for(name);
            let props: CertImmutableProps<BlockNumber, Timestamp> =
                if let Some(props) = Self::pickout(&state, &key)? {
                    props
                } else {
                    return Ok(None);
                };

            let key = <xaccounts::RemainingSharesOf<Runtime>>::key_for(name);
            let shares: u32 = if let Some(shares) = Self::pickout(&state, &key)? {
                shares
            } else {
                return Ok(None);
            };

            certs.push(CertInfo {
                name: String::from_utf8_lossy(name).into_owned(),
                issued_at: props.issued_at.1,
                frozen_duration: props.frozen_duration,
                remaining_shares: shares,
            });
        }

        Ok(Some(certs))
    }

    fn assets_of(
        &self,
        who: AccountId,
        page_index: u32,
        page_size: u32,
    ) -> Result<Option<PageData<AssetInfo>>> {
        let state = self.best_state()?;
        let chain = <xassets::Module<Runtime> as ChainT>::chain();
        let mut assets = Vec::new();

        // Native assets
        let key = <xassets::AssetList<Runtime>>::key_for(chain);
        let native_assets: Vec<Token> = Self::pickout(&state, &key)?.unwrap_or(Vec::new());

        for token in native_assets {
            let mut bmap = BTreeMap::<AssetType, Balance>::from_iter(
                xassets::AssetType::iterator().map(|t| (*t, Zero::zero())),
            );

            let key = <xassets::AssetBalance<Runtime>>::key_for(&(who.clone(), token.clone()));
            if let Some(info) = Self::pickout::<CodecBTreeMap<AssetType, Balance>>(&state, &key)? {
                bmap.extend(info.0.iter());
            }

            // PCX free balance
            if token.as_slice() == xassets::Module::<Runtime>::TOKEN {
                let free = <balances::FreeBalance<Runtime>>::key_for(&who);
                let free_balance = Self::pickout::<Balance>(&state, &free)?.unwrap_or(Zero::zero());

                bmap.insert(xassets::AssetType::Free, free_balance);
            }

            assets.push(AssetInfo {
                name: String::from_utf8_lossy(&token).into_owned(),
                is_native: true,
                details: CodecBTreeMap(bmap),
            });
        }

        // Crosschain assets
        let key = <xassets::CrossChainAssetsOf<Runtime>>::key_for(&who);
        if let Some(crosschain_assets) = Self::pickout::<Vec<Token>>(&state, &key)? {
            for token in crosschain_assets {
                let mut bmap = BTreeMap::<AssetType, Balance>::from_iter(
                    xassets::AssetType::iterator().map(|t| (*t, Zero::zero())),
                );
                let key = <xassets::AssetBalance<Runtime>>::key_for(&(who.clone(), token.clone()));
                if let Some(info) =
                    Self::pickout::<CodecBTreeMap<AssetType, Balance>>(&state, &key)?
                {
                    bmap.extend(info.0.iter());
                }

                assets.push(AssetInfo {
                    name: String::from_utf8_lossy(&token).into_owned(),
                    is_native: false,
                    details: CodecBTreeMap(bmap),
                });
            }
        }

        into_pagedata(assets, page_index, page_size)
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
            //            let (is_native , asset) = match asset {
            //                Some(info) => match info.chain() {
            //                    Chain::ChainX => (true, info),
            //                    _ => (false, info),
            //                },
            //                None => unreachable!("should not reach this branch, the token info must be exists"),
            //            };

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

            assets.push(TotalAssetInfo::new(asset, CodecBTreeMap(bmap)));
        }

        into_pagedata(assets, page_index, page_size)
    }

    fn verify_addr(
        &self,
        token: xassets::Token,
        addr: xrecords::AddrStr,
        memo: xassets::Memo,
    ) -> Result<Option<Vec<u8>>> {
        let b = self.best_number()?;
        self.client
            .runtime_api()
            .verify_address(&b, &token, &addr, &memo)
            .and_then(|r| match r {
                Ok(()) => Ok(None),
                Err(s) => Ok(Some(s)),
            })
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
                .withdrawal_list_of(&best_number, c)
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
                .withdrawal_list_of(&b, c)
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
    ) -> Result<Option<Vec<(AccountId, NominationRecord<Balance, BlockNumber>)>>> {
        let state = self.best_state()?;

        let mut records = Vec::new();

        let key = <xstaking::Intentions<Runtime>>::key();
        if let Some(intentions) = Self::pickout::<Vec<AccountId>>(&state, &key)? {
            for intention in intentions {
                let key = <xstaking::NominationRecords<Runtime>>::key_for(&(who, intention));
                if let Some(record) =
                    Self::pickout::<NominationRecord<Balance, BlockNumber>>(&state, &key)?
                {
                    records.push((intention, record));
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

        let key = <xstaking::Intentions<Runtime>>::key();

        if let Some(intentions) = Self::pickout::<Vec<AccountId>>(&state, &key)? {
            let jackpot_addr_list: Result<Vec<H256>> = self
                .client
                .runtime_api()
                .multi_jackpot_accountid_for(&self.best_number()?, &intentions)
                .map_err(|e| e.into());
            let jackpot_addr_list: Vec<H256> = jackpot_addr_list?;

            for (intention, jackpot_addr) in intentions.into_iter().zip(jackpot_addr_list) {
                let mut info = IntentionInfo::default();

                let key = <xaccounts::IntentionImmutablePropertiesOf<Runtime>>::key_for(&intention);
                if let Some(props) =
                    Self::pickout::<IntentionImmutableProps<Timestamp>>(&state, &key)?
                {
                    info.name = String::from_utf8_lossy(&props.name).into_owned();
                    info.activator = String::from_utf8_lossy(&props.activator).into_owned();
                    info.initial_shares = props.initial_shares;
                    info.registered_at = props.registered_at;
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
                    Self::pickout::<NominationRecord<Balance, BlockNumber>>(&state, &key)?
                {
                    info.self_vote = record.nomination;
                }

                info.is_validator = validators.iter().find(|i| **i == intention).is_some();
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
                .multi_token_jackpot_accountid_for(&self.best_number()?, &tokens)
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

                let key = <xassets::PCXPriceFor<Runtime>>::key_for(&token);
                if let Some(price) = Self::pickout::<Balance>(&state, &key)? {
                    info.price = price;
                }

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
                    info.used = pair.used;

                    let price_key=<xspot::OrderPairPriceOf<Runtime>>::key_for(&i);
                    if let Some(price)=Self::pickout::<(Balance,Balance,BlockNumber)>(&state,&price_key)?{
                        info.last_price=price.0;
                        info.aver_price=price.1;
                        info.update_height=price.2;
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
                    handicap.sell * ((100_u64 + price_volatility as Balance) / 100_u64);
                let min_price: Balance =
                    handicap.sell * ((100_u64 - price_volatility as Balance) / 100_u64);
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
                        for i in 0..list.len() {
                            let order_key = <xspot::AccountOrder<Runtime>>::key_for(&list[i]);
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
                    };

                    opponent_price = match opponent_price
                        .checked_sub(As::sa(10_u64.pow(pair.precision.as_())))
                    {
                        Some(v) => v,
                        None => Default::default(),
                    };
                    n = n + 1;
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
                        for i in 0..list.len() {
                            let order_key = <xspot::AccountOrder<Runtime>>::key_for(&list[i]);
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
                    };

                    opponent_price = match opponent_price
                        .checked_add(As::sa(10_u64.pow(pair.precision.as_())))
                    {
                        Some(v) => v,
                        None => Default::default(),
                    };
                    n = n + 1;
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
                    total = total + 1;
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

    fn deposit_records(&self, who: AccountId) -> Result<Option<Vec<DepositInfo>>> {
        let state = self.best_state()?;
        let mut records = Vec::new();
        let key = <IrrBlock<Runtime>>::key();
        let irr_count = if let Some(irr) = Self::pickout::<u32>(&state, &key)? {
            irr
        } else {
            return Ok(None);
        };
        let key = <AccountMap<Runtime>>::key_for(&who);
        let bind_list = if let Some(list) = Self::pickout::<Vec<Address>>(&state, &key)? {
            list
        } else {
            return Ok(None);
        };
        let key = <TrusteeAddress<Runtime>>::key();
        let trustee = if let Some(a) = Self::pickout::<keys::Address>(&state, &key)? {
            a
        } else {
            return Ok(None);
        };
        let key = <BestIndex<Runtime>>::key();
        if let Some(best_hash) = Self::pickout::<btc_chain::hash::H256>(&state, &key)? {
            let mut block_hash = best_hash;
            for _i in 0..irr_count {
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
                                        time: header_info.header.time as u64,
                                        txid: tx_hash,
                                        height: header_info.height as u64,
                                        address: btc_address,
                                        balance: dep.0,
                                        op_return: dep.1,
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
        if records.is_empty() {
            return Ok(None);
        }
        Ok(Some(records))
    }
}

fn get_deposit_info(
    raw_tx: &BTCTransaction,
    who: AccountId,
    info: &TxInfo,
    trustee: &Address,
    bind_list: &Vec<Address>,
) -> Option<(Balance, String)> {
    let mut ops = String::new();
    let mut balance = 0;
    let mut flag = bind_list
        .into_iter()
        .any(|a| a.hash == info.input_address.hash);
    for output in raw_tx.outputs.iter() {
        let script = &output.script_pubkey;
        let into_script: Script = script.clone().into();
        let s: Script = script.clone().into();
        if into_script.is_null_data_script() {
            let data = s.extract_rear(':');
            match from(data.to_vec()) {
                Ok(mut slice) => {
                    let account_id: H256 = Decode::decode(&mut slice[1..33].to_vec().as_slice())
                        .unwrap_or(H256::from(0));
                    if account_id == who || flag {
                        flag = true;
                        ops = String::from_utf8(s[2..].to_vec()).unwrap_or(String::new());
                    }
                }
                Err(_) => {
                    if flag {
                        ops = String::from_utf8(s[2..].to_vec()).unwrap_or(String::new());
                    }
                }
            }

            continue;
        }

        // get deposit money
        let script_addresses = into_script.extract_destinations().unwrap_or(Vec::new());
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
