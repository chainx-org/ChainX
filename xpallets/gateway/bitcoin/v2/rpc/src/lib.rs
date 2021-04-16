// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use std::default::Default;
use std::fmt::Display;
use std::str::FromStr;
use std::sync::Arc;

use codec::Codec;
use jsonrpc_derive::rpc;

use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{
    generic::BlockId,
    traits::{Block as BlockT, Header, Zero},
};

use xp_rpc::{runtime_error_into_rpc_err, Result};

use xpallet_gateway_bitcoin_v2_rpc_runtime_api::XGatewayBitcoinV2Api as XGatewayBitcoinV2RuntimeApi;

pub struct GatewayBitcoinV2<C, B> {
    client: Arc<C>,
    _marker: std::marker::PhantomData<B>,
}

impl<C, B> GatewayBitcoinV2<C, B> {
    pub fn new(client: Arc<C>) -> Self {
        Self {
            client,
            _marker: Default::default(),
        }
    }
}

#[rpc]
pub trait XGatewayBitcoinV2Api<BlockHash, AccountId, Balance, BlockNumber>
where
    Balance: Display + FromStr,
{
    #[rpc(name = "xgatewayBitcoinV2_getFirstMatchedVault")]
    fn get_first_matched_vault(
        &self,
        xbtc_amount: Balance,
        at: Option<BlockHash>,
    ) -> Result<Option<(AccountId, String)>>;
}

impl<C, Block, AccountId, Balance>
    XGatewayBitcoinV2Api<
        <Block as BlockT>::Hash,
        AccountId,
        Balance,
        <<Block as BlockT>::Header as Header>::Number,
    > for GatewayBitcoinV2<C, Block>
where
    C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
    C::Api: XGatewayBitcoinV2RuntimeApi<
        Block,
        AccountId,
        <<Block as BlockT>::Header as Header>::Number,
        Balance,
    >,
    Block: BlockT,
    AccountId: Clone + Display + Codec,
    Balance: Clone + Copy + Display + FromStr + Codec + Zero,
{
    fn get_first_matched_vault(
        &self,
        xbtc_amount: Balance,
        at: Option<<Block as BlockT>::Hash>,
    ) -> Result<Option<(AccountId, String)>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));
        let runtime_result = api
            .get_first_matched_vault(&at, xbtc_amount)
            .map_err(runtime_error_into_rpc_err)?;
        match runtime_result {
            Some((account, encoded_str)) => {
                let btc_address = String::from_utf8(encoded_str).map_err(|_| xp_rpc::Error {
                    code: xp_rpc::ErrorCode::ParseError,
                    message: "Invalid btc address".into(),
                    data: None,
                })?;
                Ok(Some((account, btc_address)))
            }
            None => Ok(None),
        }
    }
}
