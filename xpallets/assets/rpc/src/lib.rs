// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use std::collections::BTreeMap;
use std::str::FromStr;
use std::sync::Arc;

use codec::Codec;
use jsonrpc_core::{Error, ErrorCode, Result};
use jsonrpc_derive::rpc;
use serde::{Deserialize, Serialize};

use sp_blockchain::HeaderBackend;
use sp_runtime::{
    generic::BlockId,
    traits::{Block as BlockT, Zero},
};

use xpallet_assets_rpc_runtime_api::{
    AssetId, AssetRestrictions, AssetType, Chain, Decimals, XAssetsApi as XAssetsRuntimeApi,
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
pub trait XAssetsApi<BlockHash, AccountId, Balance>
where
    Balance: ToString + FromStr,
{
    /// Return all assets with AssetTypes for an account (exclude native token(PCX)). The returned map would not contains the assets which is not existed for this account but existed in valid assets list.
    #[rpc(name = "xassets_getAssetsByAccount")]
    fn assets_by_account(
        &self,
        who: AccountId,
        at: Option<BlockHash>,
    ) -> Result<BTreeMap<AssetId, BTreeMap<AssetType, Balance>>>;

    /// Return all valid assets balance with AssetTypes. (exclude native token(PCX))
    #[rpc(name = "xassets_getAssets")]
    fn assets(&self, at: Option<BlockHash>) -> Result<BTreeMap<AssetId, TotalAssetInfo<Balance>>>;
}

impl<C, Block, AccountId, Balance> XAssetsApi<<Block as BlockT>::Hash, AccountId, Balance>
    for Assets<C, Block>
where
    C: sp_api::ProvideRuntimeApi<Block>,
    C: HeaderBackend<Block>,
    C: Send + Sync + 'static,
    C::Api: XAssetsRuntimeApi<Block, AccountId, Balance>,
    Block: BlockT,
    AccountId: Clone + std::fmt::Display + Codec,
    Balance: Clone + Copy + std::fmt::Display + Codec + ToString + FromStr + Zero,
{
    fn assets_by_account(
        &self,
        who: AccountId,
        at: Option<<Block as BlockT>::Hash>,
    ) -> Result<BTreeMap<AssetId, BTreeMap<AssetType, Balance>>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));
        api.assets_for_account(&at, who)
            .map(|map| {
                map.into_iter()
                    .map(|(id, m)| {
                        // if balance not use u128, this part could be deleted
                        let mut r = BTreeMap::new();
                        AssetType::iter().for_each(|type_| {
                            let balance = if let Some(b) = m.get(type_) {
                                (*b).into()
                            } else {
                                Balance::zero()
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
    ) -> Result<BTreeMap<AssetId, TotalAssetInfo<Balance>>> {
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
pub struct TotalAssetInfo<Balance: ToString + FromStr> {
    pub info: AssetInfo,
    pub balance: BTreeMap<AssetType, Balance>,
    pub is_online: bool,
    pub restrictions: AssetRestrictions,
}

impl<Balance: Copy + Clone + ToString + FromStr + Zero>
    From<xpallet_assets_rpc_runtime_api::TotalAssetInfo<Balance>> for TotalAssetInfo<Balance>
{
    fn from(info: xpallet_assets_rpc_runtime_api::TotalAssetInfo<Balance>) -> Self {
        let mut r = BTreeMap::new();
        AssetType::iter().for_each(|type_| {
            let balance = if let Some(b) = info.balance.get(type_) {
                (*b).into()
            } else {
                Balance::zero().into()
            };
            r.insert(*type_, balance);
        });
        TotalAssetInfo::<Balance> {
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
