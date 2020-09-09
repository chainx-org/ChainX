use std::fmt;
use std::sync::Arc;

use jsonrpc_pubsub::manager::SubscriptionManager;

use sc_finality_grandpa::{GrandpaJustificationStream, SharedAuthoritySet, SharedVoterState};
use sc_finality_grandpa_rpc::GrandpaRpcHandler;
pub use sc_rpc_api::DenyUnsafe;
use sp_api::ProvideRuntimeApi;
use sp_block_builder::BlockBuilder;
use sp_blockchain::{Error as BlockChainError, HeaderBackend, HeaderMetadata};
use sp_transaction_pool::TransactionPool;

use chainx_primitives::Block;
use chainx_runtime::{AccountId, Balance, BlockNumber, Hash, Index};

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

/// Extra dependencies for GRANDPA
pub struct GrandpaDeps {
    /// Voting round info.
    pub shared_voter_state: SharedVoterState,
    /// Authority set info.
    pub shared_authority_set: SharedAuthoritySet<Hash, BlockNumber>,
    /// Receives notifications about justification events from Grandpa.
    pub justification_stream: GrandpaJustificationStream<Block>,
    /// Subscription manager to keep track of pubsub subscribers.
    pub subscriptions: SubscriptionManager,
}

/// Full client dependencies.
pub struct FullDeps<C, P> {
    /// The client instance to use.
    pub client: Arc<C>,
    /// Transaction pool instance.
    pub pool: Arc<P>,
    /// Whether to deny unsafe calls
    pub deny_unsafe: DenyUnsafe,
    /// GRANDPA specific dependencies.
    pub grandpa: GrandpaDeps,
}

/// A IO handler that uses all Full RPC extensions.
pub type IoHandler = jsonrpc_core::IoHandler<sc_rpc::Metadata>;

/// Instantiate all Full RPC extensions.
pub fn create_full<C, P>(deps: FullDeps<C, P>) -> jsonrpc_core::IoHandler<sc_rpc_api::Metadata>
where
    C: ProvideRuntimeApi<Block>,
    C: HeaderBackend<Block> + HeaderMetadata<Block, Error = BlockChainError> + 'static,
    C: Send + Sync + 'static,
    C::Api: BlockBuilder<Block>,
    C::Api: substrate_frame_rpc_system::AccountNonceApi<Block, AccountId, Index>,
    C::Api: pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance>,
    C::Api: xpallet_assets_rpc_runtime_api::AssetsApi<Block, AccountId, Balance>,
    C::Api:
        xpallet_mining_staking_rpc_runtime_api::XStakingApi<Block, AccountId, Balance, BlockNumber>,
    C::Api:
        xpallet_dex_spot_rpc_runtime_api::XSpotApi<Block, AccountId, Balance, BlockNumber, Balance>,
    C::Api: xpallet_mining_asset_rpc_runtime_api::XMiningAssetApi<
        Block,
        AccountId,
        Balance,
        BlockNumber,
    >,
    C::Api: xpallet_gateway_records_rpc_runtime_api::XGatewayRecordsApi<
        Block,
        AccountId,
        Balance,
        BlockNumber,
    >,
    C::Api: xpallet_gateway_common_rpc_runtime_api::XGatewayCommonApi<Block, AccountId, Balance>,
    C::Api: xpallet_contracts_rpc::ContractsRuntimeApi<Block, AccountId, Balance, BlockNumber>,
    <C::Api as sp_api::ApiErrorExt>::Error: fmt::Debug,
    P: TransactionPool + 'static,
{
    use pallet_transaction_payment_rpc::{TransactionPayment, TransactionPaymentApi};
    use substrate_frame_rpc_system::{FullSystem, SystemApi};
    use xpallet_assets_rpc::{Assets, AssetsApi};
    use xpallet_contracts_rpc::{Contracts, ContractsApi};
    use xpallet_dex_spot_rpc::{XSpot, XSpotApi};
    use xpallet_gateway_common_rpc::{XGatewayCommon, XGatewayCommonApi};
    use xpallet_gateway_records_rpc::{XGatewayRecords, XGatewayRecordsApi};
    use xpallet_mining_asset_rpc::{XMiningAsset, XMiningAssetApi};
    use xpallet_mining_staking_rpc::{XStaking, XStakingApi};

    let mut io = jsonrpc_core::IoHandler::default();
    let FullDeps {
        client,
        pool,
        deny_unsafe,
        grandpa,
    } = deps;
    let GrandpaDeps {
        shared_voter_state,
        shared_authority_set,
        justification_stream,
        subscriptions,
    } = grandpa;

    io.extend_with(SystemApi::to_delegate(FullSystem::new(
        client.clone(),
        pool,
        deny_unsafe,
    )));
    io.extend_with(TransactionPaymentApi::to_delegate(TransactionPayment::new(
        client.clone(),
    )));

    io.extend_with(sc_finality_grandpa_rpc::GrandpaApi::to_delegate(
        GrandpaRpcHandler::new(
            shared_authority_set,
            shared_voter_state,
            justification_stream,
            subscriptions,
        ),
    ));

    io.extend_with(AssetsApi::to_delegate(Assets::new(client.clone())));
    io.extend_with(ContractsApi::to_delegate(Contracts::new(client.clone())));
    io.extend_with(XStakingApi::to_delegate(XStaking::new(client.clone())));
    io.extend_with(XSpotApi::to_delegate(XSpot::new(client.clone())));
    io.extend_with(XMiningAssetApi::to_delegate(XMiningAsset::new(
        client.clone(),
    )));
    io.extend_with(XGatewayRecordsApi::to_delegate(XGatewayRecords::new(
        client.clone(),
    )));
    io.extend_with(XGatewayCommonApi::to_delegate(XGatewayCommon::new(client)));
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
