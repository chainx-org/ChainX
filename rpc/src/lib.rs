#![allow(unused)]

#[macro_use]
mod utils;

mod apis;
mod errors;
mod impls;
mod types;

use std::fmt;
use std::sync::Arc;

use sc_client_api::{backend::Backend, CallExecutor, StorageProvider};
use sc_rpc_api::DenyUnsafe;
use sc_service::client::Client;
use sp_api::ProvideRuntimeApi;
use sp_block_builder::BlockBuilder;
use sp_blockchain::{Error as BlockChainError, HeaderBackend, HeaderMetadata};
use sp_transaction_pool::TransactionPool;

use chainx_primitives::Block;
use chainx_runtime::{AccountId, Balance, BlockNumber, Hash, Index, UncheckedExtrinsic};

use apis::ChainXApi;
use impls::ChainXRpc;

/// Light client extra dependencies.
pub struct LightDeps<C, F, P> {
    /// The client instance to use.
    pub client: Arc<C>,
    /// Transaction pool instance.
    pub pool: Arc<P>,
    /// Remote access to the blockchain (async).
    pub remote_blockchain: Arc<dyn sc_client_api::light::RemoteBlockchain<Block>>,
    /// Fetcher instance.
    pub fetcher: Arc<F>,
}

/// Full client dependencies.
pub struct FullDeps<P, BE, E, RA> {
    /// The client instance to use.
    pub client: Arc<Client<BE, E, Block, RA>>,
    /// Transaction pool instance.
    pub pool: Arc<P>,
    /// Whether to deny unsafe calls
    pub deny_unsafe: DenyUnsafe,
}

/// Instantiate all Full RPC extensions.
pub fn create_full<P, M, BE, E, RA>(deps: FullDeps<P, BE, E, RA>) -> jsonrpc_core::IoHandler<M>
where
    BE: Backend<Block> + 'static,
    BE::State: sp_state_machine::backend::Backend<sp_runtime::traits::BlakeTwo256>,
    E: CallExecutor<Block>, //+ Clone + Send + Sync,
    RA: Send + Sync + 'static,
    Client<BE, E, Block, RA>: Send + Sync + 'static,
    Client<BE, E, Block, RA>: ProvideRuntimeApi<Block>,
    Client<BE, E, Block, RA>: HeaderBackend<Block>
        + HeaderMetadata<Block, Error = BlockChainError>
        + StorageProvider<Block, BE>
        + 'static,
    Client<BE, E, Block, RA>: Send + Sync + 'static,
    <Client<BE, E, Block, RA> as ProvideRuntimeApi<Block>>::Api: BlockBuilder<Block>,
    <Client<BE, E, Block, RA> as ProvideRuntimeApi<Block>>::Api:
        substrate_frame_rpc_system::AccountNonceApi<Block, AccountId, Index>,
    <Client<BE, E, Block, RA> as ProvideRuntimeApi<Block>>::Api:
        pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<
            Block,
            Balance,
            UncheckedExtrinsic,
        >,
    <Client<BE, E, Block, RA> as ProvideRuntimeApi<Block>>::Api:
        xpallet_assets_rpc_runtime_api::AssetsApi<Block, AccountId, Balance>,
    <Client<BE, E, Block, RA> as ProvideRuntimeApi<Block>>::Api:
        xpallet_mining_staking_rpc_runtime_api::XStakingApi<Block, AccountId, Balance, BlockNumber>,
    <Client<BE, E, Block, RA> as ProvideRuntimeApi<Block>>::Api:
        xpallet_dex_spot_rpc_runtime_api::XSpotApi<Block, AccountId, Balance, BlockNumber, Balance>,
    <Client<BE, E, Block, RA> as ProvideRuntimeApi<Block>>::Api:
        xpallet_gateway_records_rpc_runtime_api::XGatewayRecordsApi<
            Block,
            AccountId,
            Balance,
            BlockNumber,
        >,
    <Client<BE, E, Block, RA> as ProvideRuntimeApi<Block>>::Api:
        xpallet_gateway_common_rpc_runtime_api::XGatewayCommonApi<Block, AccountId, Balance>,
    <Client<BE, E, Block, RA> as ProvideRuntimeApi<Block>>::Api:
        xpallet_contracts_rpc::ContractsRuntimeApi<Block, AccountId, Balance, BlockNumber>,
    <<Client<BE, E, Block, RA> as ProvideRuntimeApi<Block>>::Api as sp_api::ApiErrorExt>::Error:
        fmt::Debug,
    P: TransactionPool + 'static,
    M: jsonrpc_core::Metadata + Default,
{
    use pallet_transaction_payment_rpc::{TransactionPayment, TransactionPaymentApi};
    use substrate_frame_rpc_system::{FullSystem, SystemApi};
    use xpallet_assets_rpc::{Assets, AssetsApi};
    use xpallet_contracts_rpc::{Contracts, ContractsApi};
    use xpallet_dex_spot_rpc::{XSpot, XSpotApi};
    use xpallet_gateway_common_rpc::{XGatewayCommon, XGatewayCommonApi};
    use xpallet_gateway_records_rpc::{XGatewayRecords, XGatewayRecordsApi};
    use xpallet_mining_staking_rpc::{XStaking, XStakingApi};

    let mut io = jsonrpc_core::IoHandler::default();
    let FullDeps {
        client,
        pool,
        deny_unsafe,
    } = deps;

    io.extend_with(SystemApi::to_delegate(FullSystem::new(
        client.clone(),
        pool,
        deny_unsafe,
    )));
    io.extend_with(TransactionPaymentApi::to_delegate(TransactionPayment::new(
        client.clone(),
    )));
    io.extend_with(AssetsApi::to_delegate(Assets::new(client.clone())));
    io.extend_with(ContractsApi::to_delegate(Contracts::new(client.clone())));
    io.extend_with(XStakingApi::to_delegate(XStaking::new(client.clone())));
    io.extend_with(XSpotApi::to_delegate(XSpot::new(client.clone())));
    io.extend_with(XGatewayRecordsApi::to_delegate(XGatewayRecords::new(
        client.clone(),
    )));
    io.extend_with(XGatewayCommonApi::to_delegate(XGatewayCommon::new(
        client.clone(),
    )));
    io
}

/// Instantiate all Light RPC extensions.
pub fn create_light<C, P, M, F>(deps: LightDeps<C, F, P>) -> jsonrpc_core::IoHandler<M>
where
    C: sc_client_api::blockchain::HeaderBackend<Block>,
    C: Send + Sync + 'static,
    F: sc_client_api::light::Fetcher<Block> + 'static,
    P: TransactionPool + 'static,
    M: jsonrpc_core::Metadata + Default,
{
    use substrate_frame_rpc_system::{LightSystem, SystemApi};

    let LightDeps {
        client,
        pool,
        remote_blockchain,
        fetcher,
    } = deps;
    let mut io = jsonrpc_core::IoHandler::default();
    io.extend_with(SystemApi::<Hash, AccountId, Index>::to_delegate(
        LightSystem::new(client, remote_blockchain, fetcher, pool),
    ));

    io
}
