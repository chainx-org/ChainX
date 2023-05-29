// Copyright 2019-2022 ChainX Project Authors. Licensed under GPL-3.0.

use std::sync::Arc;

use sc_client_api::AuxStore;
use sc_consensus_babe::Epoch;
use sc_consensus_babe_rpc::BabeRpcHandler;
use sc_finality_grandpa::{
    FinalityProofProvider, GrandpaJustificationStream, SharedAuthoritySet, SharedVoterState,
};
use sc_finality_grandpa_rpc::GrandpaRpcHandler;
use sc_rpc::SubscriptionTaskExecutor;
pub use sc_rpc_api::DenyUnsafe;
use sc_transaction_pool_api::TransactionPool;
use sp_api::ProvideRuntimeApi;
use sp_block_builder::BlockBuilder;
use sp_blockchain::{Error as BlockChainError, HeaderBackend, HeaderMetadata};
use sp_consensus::SelectChain;
use sp_consensus_babe::BabeApi;

use chainx_primitives::{AccountId, Balance, Block, BlockNumber, Hash, Index};

use xpallet_mining_asset_rpc_runtime_api::MiningWeight;
use xpallet_mining_staking_rpc_runtime_api::VoteWeight;

// EVM
use fc_rpc::{
    EthBlockDataCacheTask, OverrideHandle, RuntimeApiStorageOverride, SchemaV1Override,
    SchemaV2Override, SchemaV3Override, StorageOverride,
};
use fc_rpc_core::types::{FeeHistoryCache, FilterPool};
use fp_storage::EthereumStorageSchema;
use jsonrpc_pubsub::manager::SubscriptionManager;
use sc_client_api::{
    backend::{Backend, StateBackend, StorageProvider},
    client::BlockchainEvents,
};
use sc_network::NetworkService;
use sc_transaction_pool::{ChainApi, Pool};
use sp_runtime::traits::BlakeTwo256;
use std::collections::BTreeMap;
use xp_runtime::Never;

/// Extra dependencies for BABE.
pub struct BabeDeps {
    /// BABE protocol config.
    pub babe_config: sc_consensus_babe::Config,
    /// BABE pending epoch changes.
    pub shared_epoch_changes: sc_consensus_epochs::SharedEpochChanges<Block, Epoch>,
    /// The keystore that manages the keys of the node.
    pub keystore: sp_keystore::SyncCryptoStorePtr,
}

/// Extra dependencies for GRANDPA
pub struct GrandpaDeps<B> {
    /// Voting round info.
    pub shared_voter_state: SharedVoterState,
    /// Authority set info.
    pub shared_authority_set: SharedAuthoritySet<Hash, BlockNumber>,
    /// Receives notifications about justification events from Grandpa.
    pub justification_stream: GrandpaJustificationStream<Block>,
    /// Executor to drive the subscription manager in the Grandpa RPC handler.
    pub subscription_executor: SubscriptionTaskExecutor,
    /// Finality proof provider.
    pub finality_provider: Arc<FinalityProofProvider<B, Block>>,
}

/// Frontier client dependencies
pub struct FrontierDeps<A: sc_transaction_pool::ChainApi> {
    /// Graph pool instance.
    pub graph: Arc<Pool<A>>,
    /// The Node authority flag
    pub is_authority: bool,
    /// Network service
    pub network: Arc<NetworkService<Block, Hash>>,
    /// EthFilterApi pool.
    pub filter_pool: Option<FilterPool>,
    /// Backend.
    pub backend: Arc<fc_db::Backend<Block>>,
    /// Maximum number of logs in a query.
    pub max_past_logs: u32,
    /// Maximum fee history cache size.
    pub fee_history_limit: u64,
    /// Fee history cache.
    pub fee_history_cache: FeeHistoryCache,
    /// Ethereum data access overrides.
    pub overrides: Arc<OverrideHandle<Block>>,
    /// Cache for Ethereum block data.
    pub block_data_cache: Arc<EthBlockDataCacheTask<Block>>,
}

/// Full client dependencies.
pub struct FullDeps<C, P, SC, B, A: sc_transaction_pool::ChainApi> {
    /// The client instance to use.
    pub client: Arc<C>,
    /// Transaction pool instance.
    pub pool: Arc<P>,
    /// The SelectionChain Strategy.
    pub select_chain: SC,
    /// A copy of the chain spec.
    pub chain_spec: Box<dyn sc_chain_spec::ChainSpec>,
    /// Whether to deny unsafe calls
    pub deny_unsafe: DenyUnsafe,
    /// BABE specific dependencies.
    pub babe: BabeDeps,
    /// GRANDPA specific dependencies.
    pub grandpa: GrandpaDeps<B>,
    /// Frontier specific dependencies.
    pub frontier: FrontierDeps<A>,
}

pub fn overrides_handle<C, B>(client: Arc<C>) -> Arc<OverrideHandle<Block>>
where
    C: ProvideRuntimeApi<Block> + StorageProvider<Block, B> + AuxStore,
    C: HeaderBackend<Block> + HeaderMetadata<Block, Error = BlockChainError>,
    C: Send + Sync + 'static,
    C::Api: sp_api::ApiExt<Block>
        + fp_rpc::EthereumRuntimeRPCApi<Block>
        + fp_rpc::ConvertTransactionRuntimeApi<Block>,
    B: Backend<Block> + 'static,
    B::State: StateBackend<BlakeTwo256>,
{
    let mut overrides_map = BTreeMap::new();
    overrides_map.insert(
        EthereumStorageSchema::V1,
        Box::new(SchemaV1Override::new(client.clone()))
            as Box<dyn StorageOverride<_> + Send + Sync>,
    );
    overrides_map.insert(
        EthereumStorageSchema::V2,
        Box::new(SchemaV2Override::new(client.clone()))
            as Box<dyn StorageOverride<_> + Send + Sync>,
    );

    overrides_map.insert(
        EthereumStorageSchema::V3,
        Box::new(SchemaV3Override::new(client.clone()))
            as Box<dyn StorageOverride<_> + Send + Sync>,
    );

    Arc::new(OverrideHandle {
        schemas: overrides_map,
        fallback: Box::new(RuntimeApiStorageOverride::new(client)),
    })
}

/// A IO handler that uses all Full RPC extensions.
pub type RpcExtension = jsonrpc_core::IoHandler<sc_rpc::Metadata>;

/// Instantiate all Full RPC extensions.
pub fn create_full<C, P, SC, B, A>(
    deps: FullDeps<C, P, SC, B, A>,
    subscription_task_executor: SubscriptionTaskExecutor,
) -> Result<RpcExtension, Box<dyn std::error::Error + Send + Sync>>
where
    C: ProvideRuntimeApi<Block>
        + AuxStore
        + HeaderBackend<Block>
        + HeaderMetadata<Block, Error = BlockChainError>
        + StorageProvider<Block, B>
        + BlockchainEvents<Block>
        + Send
        + Sync
        + 'static,
    C::Api: BlockBuilder<Block>,
    C::Api: BabeApi<Block>,
    C::Api: substrate_frame_rpc_system::AccountNonceApi<Block, AccountId, Index>,
    C::Api: pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance>,
    C::Api: xpallet_assets_rpc_runtime_api::XAssetsApi<Block, AccountId, Balance>,
    C::Api:
        xpallet_dex_spot_rpc_runtime_api::XSpotApi<Block, AccountId, Balance, BlockNumber, Balance>,
    C::Api: xpallet_gateway_bitcoin_rpc_runtime_api::XGatewayBitcoinApi<Block, AccountId>,
    C::Api: xpallet_gateway_common_rpc_runtime_api::XGatewayCommonApi<
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
    C::Api: xpallet_mining_staking_rpc_runtime_api::XStakingApi<
        Block,
        AccountId,
        Balance,
        VoteWeight,
        BlockNumber,
    >,
    C::Api: xpallet_mining_asset_rpc_runtime_api::XMiningAssetApi<
        Block,
        AccountId,
        Balance,
        MiningWeight,
        BlockNumber,
    >,
    C::Api: xpallet_btc_ledger_runtime_api::BtcLedgerApi<Block, AccountId, Balance>,
    C::Api: xpallet_transaction_fee_rpc_runtime_api::XTransactionFeeApi<Block, Balance>,
    C::Api: fp_rpc::EthereumRuntimeRPCApi<Block>,
    C::Api: fp_rpc::ConvertTransactionRuntimeApi<Block>,
    P: TransactionPool<Block = Block> + Sync + Send + 'static,
    SC: SelectChain<Block> + 'static,
    B: sc_client_api::Backend<Block> + Send + Sync + 'static,
    B::State: sc_client_api::backend::StateBackend<sp_runtime::traits::HashFor<Block>>,
    A: ChainApi<Block = Block> + 'static,
{
    use pallet_transaction_payment_rpc::{TransactionPayment, TransactionPaymentApi};
    use substrate_frame_rpc_system::{FullSystem, SystemApi};
    use xpallet_assets_rpc::{Assets, XAssetsApi};
    use xpallet_btc_ledger_rpc::{BtcLedger, BtcLedgerApi};
    use xpallet_dex_spot_rpc::{XSpot, XSpotApi};
    use xpallet_gateway_bitcoin_rpc::{XGatewayBitcoin, XGatewayBitcoinApi};
    use xpallet_gateway_common_rpc::{XGatewayCommon, XGatewayCommonApi};
    use xpallet_gateway_records_rpc::{XGatewayRecords, XGatewayRecordsApi};
    use xpallet_mining_asset_rpc::{XMiningAsset, XMiningAssetApi};
    use xpallet_mining_staking_rpc::{XStaking, XStakingApi};
    use xpallet_transaction_fee_rpc::{XTransactionFee, XTransactionFeeApi};

    let mut io = jsonrpc_core::IoHandler::default();
    let FullDeps {
        client,
        pool,
        select_chain,
        chain_spec,
        deny_unsafe,
        grandpa,
        babe,
        frontier,
    } = deps;

    let BabeDeps {
        keystore,
        babe_config,
        shared_epoch_changes,
    } = babe;
    let GrandpaDeps {
        shared_voter_state,
        shared_authority_set,
        justification_stream,
        subscription_executor,
        finality_provider,
    } = grandpa;

    io.extend_with(SystemApi::to_delegate(FullSystem::new(
        client.clone(),
        pool.clone(),
        deny_unsafe,
    )));
    io.extend_with(TransactionPaymentApi::to_delegate(TransactionPayment::new(
        client.clone(),
    )));
    io.extend_with(sc_consensus_babe_rpc::BabeApi::to_delegate(
        BabeRpcHandler::new(
            client.clone(),
            shared_epoch_changes.clone(),
            keystore,
            babe_config,
            select_chain,
            deny_unsafe,
        ),
    ));
    io.extend_with(sc_finality_grandpa_rpc::GrandpaApi::to_delegate(
        GrandpaRpcHandler::new(
            shared_authority_set.clone(),
            shared_voter_state,
            justification_stream,
            subscription_executor,
            finality_provider,
        ),
    ));
    io.extend_with(sc_sync_state_rpc::SyncStateRpcApi::to_delegate(
        sc_sync_state_rpc::SyncStateRpcHandler::new(
            chain_spec,
            client.clone(),
            shared_authority_set,
            shared_epoch_changes,
        )?,
    ));

    io.extend_with(XTransactionFeeApi::to_delegate(XTransactionFee::new(
        client.clone(),
    )));
    io.extend_with(XAssetsApi::to_delegate(Assets::new(client.clone())));
    io.extend_with(XStakingApi::to_delegate(XStaking::new(client.clone())));
    io.extend_with(XSpotApi::to_delegate(XSpot::new(client.clone())));
    io.extend_with(XMiningAssetApi::to_delegate(XMiningAsset::new(
        client.clone(),
    )));
    io.extend_with(XGatewayBitcoinApi::to_delegate(XGatewayBitcoin::new(
        client.clone(),
    )));
    io.extend_with(XGatewayRecordsApi::to_delegate(XGatewayRecords::new(
        client.clone(),
    )));
    io.extend_with(XGatewayCommonApi::to_delegate(XGatewayCommon::new(
        client.clone(),
    )));
    io.extend_with(BtcLedgerApi::to_delegate(BtcLedger::new(client.clone())));

    // EVM
    {
        use fc_rpc::{
            EthApi, EthApiServer, EthFilterApi, EthFilterApiServer, EthPubSubApi,
            EthPubSubApiServer, HexEncodedIdProvider, NetApi, NetApiServer, Web3Api, Web3ApiServer,
        };

        let FrontierDeps {
            graph,
            is_authority,
            network,
            filter_pool,
            backend,
            max_past_logs,
            fee_history_limit,
            fee_history_cache,
            overrides,
            block_data_cache,
        } = frontier;

        let convert_transaction: Option<Never> = None;

        io.extend_with(EthApiServer::to_delegate(EthApi::new(
            client.clone(),
            pool.clone(),
            graph,
            convert_transaction,
            network.clone(),
            Vec::new(),
            overrides.clone(),
            backend.clone(),
            is_authority,
            max_past_logs,
            block_data_cache.clone(),
            fc_rpc::format::Geth,
            fee_history_limit,
            fee_history_cache,
        )));

        if let Some(filter_pool) = filter_pool {
            io.extend_with(EthFilterApiServer::to_delegate(EthFilterApi::new(
                client.clone(),
                backend,
                filter_pool,
                500_usize, // max stored filters
                max_past_logs,
                block_data_cache,
            )));
        }

        io.extend_with(NetApiServer::to_delegate(NetApi::new(
            client.clone(),
            network.clone(),
            // Whether to format the `peer_count` response as Hex (default) or not.
            true,
        )));

        io.extend_with(Web3ApiServer::to_delegate(Web3Api::new(client.clone())));

        io.extend_with(EthPubSubApiServer::to_delegate(EthPubSubApi::new(
            pool,
            client,
            network,
            SubscriptionManager::<HexEncodedIdProvider>::with_id_provider(
                HexEncodedIdProvider::default(),
                Arc::new(subscription_task_executor),
            ),
            overrides,
        )));
    }

    Ok(io)
}
