use std::collections::BTreeMap;
use std::sync::Arc;

use codec::Codec;
use jsonrpc_core::{Error, ErrorCode, Result};
use jsonrpc_derive::rpc;
use serde::{Deserialize, Serialize};

use sp_blockchain::HeaderBackend;
use sp_runtime::{generic::BlockId, traits::Block as BlockT};

use chainx_primitives::AssetId;
use xpallet_assets_rpc_runtime_api::{
    AssetRestrictions, AssetType, AssetsApi as AssetsRuntimeApi, Chain, Decimals,
};

pub struct Assets<C, B> {
    client: Arc<C>,
    _marker: std::marker::PhantomData<B>,
}

impl<C, B> Assets<C, B> {
    /// Create new `Contracts` with the given reference to the client.
    pub fn new(client: Arc<C>) -> Self {
        Assets {
            client,
            _marker: Default::default(),
        }
    }
}

#[rpc]
pub trait AssetsApi<BlockHash, AccountId, Balance> {
    /// Return all assets with AssetTypes for an account (exclude native token(PCX)). The returned map would not contains the assets which is not existed for this account but existed in valid assets list.
    #[rpc(name = "xassets_getAssetsByAccount")]
    fn assets_by_account(
        &self,
        who: AccountId,
        at: Option<BlockHash>,
    ) -> Result<BTreeMap<AssetId, BTreeMap<AssetType, String>>>;

    /// Return all valid assets balance with AssetTypes. (exclude native token(PCX))
    #[rpc(name = "xassets_getAssets")]
    fn assets(&self, at: Option<BlockHash>) -> Result<BTreeMap<AssetId, TotalAssetInfo>>;
}

impl<C, Block, AccountId, Balance> AssetsApi<<Block as BlockT>::Hash, AccountId, Balance>
    for Assets<C, Block>
where
    C: sp_api::ProvideRuntimeApi<Block>,
    C: HeaderBackend<Block>,
    C: Send + Sync + 'static,
    C::Api: AssetsRuntimeApi<Block, AccountId, Balance>,
    Block: BlockT,
    AccountId: Clone + std::fmt::Display + Codec,
    Balance: Clone + std::fmt::Display + Codec + ToString,
{
    fn assets_by_account(
        &self,
        who: AccountId,
        at: Option<<Block as BlockT>::Hash>,
    ) -> Result<BTreeMap<AssetId, BTreeMap<AssetType, String>>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));
        api.assets_for_account(&at, who)
            .map(|map| {
                map.into_iter()
                    .map(|(id, m)| {
                        // if balance not use u128, this part could be deleted
                        let mut r = BTreeMap::new();
                        AssetType::iterator().for_each(|type_| {
                            let balance = if let Some(b) = m.get(type_) {
                                (*b).to_string()
                            } else {
                                "0".to_string()
                            };
                            r.insert(*type_, balance);
                        });
                        (id, r)
                    })
                    .collect()
            })
            .map_err(runtime_error_into_rpc_err)
    }

    fn assets(
        &self,
        at: Option<<Block as BlockT>::Hash>,
    ) -> Result<BTreeMap<AssetId, TotalAssetInfo>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));
        api.assets(&at)
            .map(|map| {
                map.into_iter()
                    .map(|(id, info)| (id, info.into()))
                    .collect()
            })
            .map_err(runtime_error_into_rpc_err)
    }
}

#[derive(PartialEq, Eq, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetInfo {
    token: String,
    token_name: String,
    chain: Chain,
    decimals: Decimals,
    desc: String,
}

impl From<xpallet_assets_rpc_runtime_api::AssetInfo> for AssetInfo {
    fn from(info: xpallet_assets_rpc_runtime_api::AssetInfo) -> Self {
        AssetInfo {
            token: String::from_utf8_lossy(&info.token()).into_owned(),
            token_name: String::from_utf8_lossy(&info.token_name()).into_owned(),
            chain: info.chain(),
            decimals: info.decimals(),
            desc: String::from_utf8_lossy(&info.desc()).into_owned(),
        }
    }
}

#[derive(PartialEq, Eq, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TotalAssetInfo {
    pub info: AssetInfo,
    pub balance: BTreeMap<AssetType, String>,
    pub is_online: bool,
    pub restrictions: AssetRestrictions,
}

impl<Balance: ToString> From<xpallet_assets_rpc_runtime_api::TotalAssetInfo<Balance>>
    for TotalAssetInfo
{
    fn from(info: xpallet_assets_rpc_runtime_api::TotalAssetInfo<Balance>) -> Self {
        let mut r = BTreeMap::new();
        AssetType::iterator().for_each(|type_| {
            let balance = if let Some(b) = info.balance.get(type_) {
                (*b).to_string()
            } else {
                "0".to_string()
            };
            r.insert(*type_, balance);
        });
        TotalAssetInfo {
            info: info.info.into(),
            balance: r,
            is_online: info.is_online,
            restrictions: info.restrictions,
        }
    }
}

const RUNTIME_ERROR: i64 = 1;
fn runtime_error_into_rpc_err(err: impl std::fmt::Debug) -> Error {
    Error {
        code: ErrorCode::ServerError(RUNTIME_ERROR),
        message: "Runtime trapped".into(),
        data: Some(format!("{:?}", err).into()),
    }
}
