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
use runtime_primitives::traits::{Block as BlockT, Header, NumberFor, Zero};

use srml_support::storage::{StorageMap, StorageValue};

use chainx_primitives::{AccountId, Balance, BlockNumber, Timestamp};
use chainx_runtime::Runtime;

use xaccounts::{self, CertImmutableProps, IntentionImmutableProps, IntentionProps};
use xassets::{self, assetdef::ChainT, AssetType, Token};
use xstaking::{self, IntentionProfs, NominationRecord};
use xsupport::storage::btree_map::CodecBTreeMap;

mod error;

use self::error::Result;

/// Cert info
#[derive(Debug, PartialEq, Serialize)]
pub struct CertInfo {
    /// name of cert
    pub name: String,
    /// when is the cert issued at
    pub issued_at: DateTime<Utc>,
    /// frozen duration of the shares cert owner holds
    pub frozen_duration: u32,
    /// remaining share of the cert
    pub remaining_shares: u32,
}

/// Intention info
#[derive(Debug, Default, PartialEq, Serialize)]
pub struct IntentionInfo {
    /// name of intention
    pub name: String,
    /// activator
    pub activator: String,
    /// initial shares
    pub initial_shares: u32,
    /// url
    pub url: String,
    /// is running for the validators
    pub is_active: bool,
    /// is validator
    pub is_validator: bool,
    /// how much has intention voted for itself
    pub self_vote: Balance,
    /// jackpot
    pub jackpot: Balance,
    /// total nomination from all nominators
    pub total_nomination: Balance,
    /// vote weight at last update
    pub last_total_vote_weight: u64,
    /// last update time of vote weight
    pub last_total_vote_weight_update: BlockNumber,
}

build_rpc_trait! {
    /// ChainX API
    pub trait ChainXApi<Number, AccountId, Balance, BlockNumber> where SignedBlock: Serialize, {

        /// Returns the block of a storage entry at a block's Number.
        #[rpc(name = "chainx_getBlockByNumber")]
        fn block_info(&self, Trailing<Number>) -> Result<Option<SignedBlock>>;

        #[rpc(name = "chainx_getCertByAccount")]
        fn cert(&self, AccountId) -> Result<Option<Vec<CertInfo>>>;

        #[rpc(name = "chainx_getAssetsByAccount")]
        fn assets_of(&self, AccountId) -> Result<Option<Vec<(String, CodecBTreeMap<AssetType, Balance>)>>>;

        #[rpc(name = "chainx_getNominationRecords")]
        fn nomination_records(&self, AccountId) -> Result<Option<Vec<(AccountId, NominationRecord<Balance, BlockNumber>)>>>;

        #[rpc(name = "chainx_getIntentions")]
        fn intentions(&self) -> Result<Option<Vec<(AccountId, DateTime<Utc>, IntentionInfo)>>>;
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

    fn assets_of(
        &self,
        who: AccountId,
    ) -> Result<Option<Vec<(String, CodecBTreeMap<AssetType, Balance>)>>> {
        let state = self.best_state()?;

        let chain = <xassets::Module<Runtime> as ChainT>::chain();

        // Native assets
        let key = <xassets::AssetList<Runtime>>::key_for(chain);
        let mut all_assets: Vec<Token> = Self::pickout(&state, &key)?.unwrap_or(Vec::new());

        // Crosschain assets
        let key = <xassets::CrossChainAssetsOf<Runtime>>::key_for(&who);
        if let Some(crosschain_assets) = Self::pickout::<Vec<Token>>(&state, &key)? {
            all_assets.extend_from_slice(&crosschain_assets);
        }

        let mut assets = Vec::new();
        for token in all_assets {
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

            assets.push((
                String::from_utf8_lossy(&token).into_owned(),
                CodecBTreeMap(bmap),
            ));
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
}

fn datetime(timestamp: u64) -> DateTime<Utc> {
    let naive_datetime = NaiveDateTime::from_timestamp(timestamp as i64, 0);
    DateTime::from_utc(naive_datetime, Utc)
}
