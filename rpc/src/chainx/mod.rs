//! ChainX API

use std::collections::BTreeMap;
use std::iter::FromIterator;
use std::sync::Arc;

use chrono::prelude::*;
use codec::{Decode, Encode};
use jsonrpc_macros::Trailing;

use serde::Serialize;

use client::{self, runtime_api::Metadata, Client};
use state_machine::Backend;

use primitives::storage::{StorageData, StorageKey};
use primitives::{Blake2Hasher, H256};
use runtime_primitives::generic::{BlockId, SignedBlock};
use runtime_primitives::traits::{As, Block as BlockT, Header, NumberFor, Zero};

use srml_support::storage::{StorageMap, StorageValue};

use chainx_primitives::{AccountId, Balance, BlockNumber, Timestamp};
use chainx_runtime::Runtime;

use xaccounts::{self, CertImmutableProps, IntentionImmutableProps, IntentionProps};
use xassets::{self, assetdef::ChainT, AssetType, Token};
use xspot::def::{OrderPair, OrderPairID, ID};
use xspot::{HandicapT, OrderT};
use xstaking::{self, IntentionProfs, NominationRecord};
use xsupport::storage::btree_map::CodecBTreeMap;
use xtokens::{self, DepositVoteWeight, PseduIntentionVoteWeight};

mod error;
mod types;

use self::error::Result;
use self::types::*;
use chainx::error::ErrorKind::{OrderPairIDErr, PageIndexErr, PageSizeErr, QuotationssPieceErr};

const MAX_PAGE_SIZE: u32 = 100;

build_rpc_trait! {
    /// ChainX API
    pub trait ChainXApi<Number, AccountId, Balance, BlockNumber> where SignedBlock: Serialize, {

        /// Returns the block of a storage entry at a block's Number.
        #[rpc(name = "chainx_getBlockByNumber")]
        fn block_info(&self, Trailing<Number>) -> Result<Option<SignedBlock>>;

        #[rpc(name = "chainx_getCertByAccount")]
        fn cert(&self, AccountId) -> Result<Option<Vec<CertInfo>>>;

        #[rpc(name = "chainx_getAssetsByAccount")]
        fn assets_of(&self, AccountId) -> Result<Option<Vec<AssetInfo>>>;

        #[rpc(name = "chainx_getNominationRecords")]
        fn nomination_records(&self, AccountId) -> Result<Option<Vec<(AccountId, NominationRecord<Balance, BlockNumber>)>>>;

        #[rpc(name = "chainx_getIntentions")]
        fn intentions(&self) -> Result<Option<Vec<(AccountId, DateTime<Utc>, IntentionInfo)>>>;

        #[rpc(name = "chainx_getPseduIntentions")]
        fn psedu_intentions(&self) -> Result<Option<Vec<PseduIntentionInfo>>>;

        #[rpc(name = "chainx_getPseduNominationRecords")]
        fn psedu_nomination_records(&self, AccountId) -> Result<Option<Vec<PseduNominationRecord>>>;

        #[rpc(name = "chainx_getOrderPairs")]
        fn order_pairs(&self) -> Result<Option<Vec<(PairInfo)>>>;

        #[rpc(name = "chainx_getQuotations")]
        fn quotationss(&self,OrderPairID,u32) -> Result<Option<QuotationsList>>;

        #[rpc(name = "chainx_getOrders")]
        fn orders(&self,AccountId,u32,u32) -> Result<Option<OrderList>>;

    }
}

/// ChainX API
pub struct ChainX<B, E, Block: BlockT, RA> {
    client: Arc<Client<B, E, Block, RA>>,
}

impl<B, E, Block: BlockT, RA> ChainX<B, E, Block, RA>
where
    Block: BlockT<Hash = H256> + 'static,
    B: client::backend::Backend<Block, Blake2Hasher> + Send + Sync + 'static,
    E: client::CallExecutor<Block, Blake2Hasher> + Send + Sync + 'static,
    RA: Metadata<Block>,
{
    /// Create new ChainX API RPC handler.
    pub fn new(client: Arc<Client<B, E, Block, RA>>) -> Self {
        Self { client }
    }

    fn to_storage_key(key: &[u8]) -> StorageKey {
        let hashed = primitives::twox_128(key).to_vec();
        StorageKey(hashed)
    }

    /// Get best state of the chain.
    fn best_state(
        &self,
    ) -> std::result::Result<
        <B as client::backend::Backend<Block, Blake2Hasher>>::State,
        client::error::Error,
    > {
        let best_hash = self.client.info()?.chain.best_hash;
        let state = self.client.state_at(&BlockId::Hash(best_hash))?;
        Ok(state)
    }

    fn timestamp(&self, number: BlockNumber) -> std::result::Result<Timestamp, error::Error> {
        let number = number.encode();
        let number: NumberFor<Block> = Decode::decode(&mut number.as_slice()).unwrap();

        let state = self.client.state_at(&BlockId::Number(number))?;

        let key = <timestamp::Now<Runtime>>::key();

        Ok(Self::pickout::<Timestamp>(&state, &key)?.unwrap())
    }

    /// Pick out specified data from storage given the state and key.
    fn pickout<ReturnValue: Decode>(
        state: &<B as client::backend::Backend<Block, Blake2Hasher>>::State,
        key: &[u8],
    ) -> std::result::Result<Option<ReturnValue>, error::Error> {
        Ok(state
            .storage(&Self::to_storage_key(key).0)
            .map_err(|e| error::Error::from_state(Box::new(e)))?
            .map(StorageData)
            .map(|s| Decode::decode(&mut s.0.as_slice()))
            .unwrap_or(None))
    }
}

impl<B, E, Block, RA>
    ChainXApi<NumberFor<Block>, AccountId, Balance, BlockNumber, SignedBlock<Block>>
    for ChainX<B, E, Block, RA>
where
    Block: BlockT<Hash = H256> + 'static,
    B: client::backend::Backend<Block, Blake2Hasher> + Send + Sync + 'static,
    E: client::CallExecutor<Block, Blake2Hasher> + Send + Sync + 'static,
    RA: Metadata<Block>,
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
                issued_at: datetime(props.issued_at.1),
                frozen_duration: props.frozen_duration,
                remaining_shares: shares,
            });
        }

        Ok(Some(certs))
    }

    fn assets_of(&self, who: AccountId) -> Result<Option<Vec<AssetInfo>>> {
        let state = self.best_state()?;

        let chain = <xassets::Module<Runtime> as ChainT>::chain();

        let mut assets = Vec::new();

        // Native assets
        let key = <xassets::AssetList<Runtime>>::key_for(chain);
        let mut all_assets: Vec<Token> = Self::pickout(&state, &key)?.unwrap_or(Vec::new());

        for token in all_assets {
            let mut asset = AssetInfo::default();

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

            asset.name = String::from_utf8_lossy(&token).into_owned();
            asset.is_native = true;
            asset.details = CodecBTreeMap(bmap);

            assets.push(asset);
        }

        // Crosschain assets
        let key = <xassets::CrossChainAssetsOf<Runtime>>::key_for(&who);
        if let Some(crosschain_assets) = Self::pickout::<Vec<Token>>(&state, &key)? {
            for token in crosschain_assets {
                let mut asset = AssetInfo::default();

                let mut bmap = BTreeMap::<AssetType, Balance>::from_iter(
                    xassets::AssetType::iterator().map(|t| (*t, Zero::zero())),
                );
                let key = <xassets::AssetBalance<Runtime>>::key_for(&(who.clone(), token.clone()));
                if let Some(info) =
                    Self::pickout::<CodecBTreeMap<AssetType, Balance>>(&state, &key)?
                {
                    bmap.extend(info.0.iter());
                }

                asset.name = String::from_utf8_lossy(&token).into_owned();
                asset.is_native = false;
                asset.details = CodecBTreeMap(bmap);

                assets.push(asset);
            }
        }

        Ok(Some(assets))
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

    fn intentions(&self) -> Result<Option<Vec<(AccountId, DateTime<Utc>, IntentionInfo)>>> {
        let state = self.best_state()?;
        let mut intention_info = Vec::new();

        let key = <session::Validators<Runtime>>::key();
        let validators =
            Self::pickout::<Vec<AccountId>>(&state, &key)?.expect("Validators can't be empty");

        let key = <xstaking::Intentions<Runtime>>::key();

        if let Some(intentions) = Self::pickout::<Vec<AccountId>>(&state, &key)? {
            for intention in intentions {
                let mut info = IntentionInfo::default();
                let mut registered_at = Utc::now();

                let key = <xaccounts::IntentionImmutablePropertiesOf<Runtime>>::key_for(&intention);
                if let Some(props) =
                    Self::pickout::<IntentionImmutableProps<Timestamp>>(&state, &key)?
                {
                    info.name = String::from_utf8_lossy(&props.name).into_owned();
                    info.activator = String::from_utf8_lossy(&props.activator).into_owned();
                    info.initial_shares = props.initial_shares;
                    registered_at = datetime(props.registered_at);
                }

                let key = <xaccounts::IntentionPropertiesOf<Runtime>>::key_for(&intention);
                if let Some(props) = Self::pickout::<IntentionProps>(&state, &key)? {
                    info.url = String::from_utf8_lossy(&props.url).into_owned();
                    info.is_active = props.is_active;
                }

                let key = <xstaking::IntentionProfiles<Runtime>>::key_for(&intention);
                if let Some(profs) =
                    Self::pickout::<IntentionProfs<Balance, BlockNumber>>(&state, &key)?
                {
                    info.jackpot = profs.jackpot;
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
                intention_info.push((intention, registered_at, info));
            }
        }

        Ok(Some(intention_info))
    }

    fn psedu_intentions(&self) -> Result<Option<Vec<PseduIntentionInfo>>> {
        let state = self.best_state()?;
        let mut psedu_intentions = Vec::new();

        let key = <xtokens::PseduIntentions<Runtime>>::key();

        if let Some(tokens) = Self::pickout::<Vec<Token>>(&state, &key)? {
            for token in tokens {
                let mut info = PseduIntentionInfo::default();

                let key = <xtokens::PseduIntentionProfiles<Runtime>>::key_for(&token);
                let vote_weight =
                    Self::pickout::<PseduIntentionVoteWeight<Balance, BlockNumber>>(&state, &key)?
                        .expect("Fail to decode PseduIntentionVoteWeight");
                info.jackpot = vote_weight.jackpot;
                info.last_total_deposit_weight = vote_weight.last_total_deposit_weight;
                info.last_total_deposit_weight_update =
                    vote_weight.last_total_deposit_weight_update;

                let key = <xassets::PCXPriceFor<Runtime>>::key_for(&token);
                let price =
                    Self::pickout::<Balance>(&state, &key)?.expect("Fail to decode PCXPriceFor");
                info.price = price;

                let key = <xassets::TotalAssetBalance<Runtime>>::key_for(&token);
                let total_asset_balance =
                    Self::pickout::<CodecBTreeMap<AssetType, Balance>>(&state, &key)?
                        .expect("Fail to decode TotalAssetBalance");
                info.circulation = total_asset_balance
                    .0
                    .iter()
                    .fold(Zero::zero(), |acc, (_, v)| acc + *v);

                info.id = token;
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

                record.id = token;

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

    fn orders(&self, who: AccountId, page_index: u32, page_size: u32) -> Result<Option<OrderList>> {
        if page_size > MAX_PAGE_SIZE || page_size < 1 {
            return Err(PageSizeErr.into());
        }

        let mut list = OrderList::default();
        let mut orders = Vec::new();
        list.page_index = page_index;
        list.page_size = page_size;
        list.page_total = 0;

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

            list.page_total = total_page;

            if page_index >= total_page && total_page > 0 {
                return Err(PageIndexErr.into());
            }
        }
        list.data = orders;

        Ok(Some(list))
    }
}

fn datetime(timestamp: u64) -> DateTime<Utc> {
    let naive_datetime = NaiveDateTime::from_timestamp(timestamp as i64, 0);
    DateTime::from_utc(naive_datetime, Utc)
}
