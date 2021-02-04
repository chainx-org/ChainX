use super::*;

/// A set of APIs that chainx-like runtimes must implement.
pub trait RuntimeApiCollection:
    sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block>
    + sp_api::ApiExt<Block, Error = sp_blockchain::Error>
    + sp_consensus_babe::BabeApi<Block>
    + sp_finality_grandpa::GrandpaApi<Block>
    + sp_block_builder::BlockBuilder<Block>
    + frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce>
    + pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance>
    + sp_api::Metadata<Block>
    + sp_offchain::OffchainWorkerApi<Block>
    + sp_session::SessionKeys<Block>
    + sp_authority_discovery::AuthorityDiscoveryApi<Block>
    + xpallet_assets_rpc_runtime_api::XAssetsApi<Block, AccountId, Balance>
    + xpallet_dex_spot_rpc_runtime_api::XSpotApi<Block, AccountId, Balance, BlockNumber, Balance>
    + xpallet_gateway_common_rpc_runtime_api::XGatewayCommonApi<Block, AccountId, Balance>
    + xpallet_gateway_records_rpc_runtime_api::XGatewayRecordsApi<
        Block,
        AccountId,
        Balance,
        BlockNumber,
    > + xpallet_mining_staking_rpc_runtime_api::XStakingApi<
        Block,
        AccountId,
        Balance,
        VoteWeight,
        BlockNumber,
    > + xpallet_mining_asset_rpc_runtime_api::XMiningAssetApi<
        Block,
        AccountId,
        Balance,
        MiningWeight,
        BlockNumber,
    > + xpallet_transaction_fee_rpc_runtime_api::XTransactionFeeApi<Block, Balance>
where
    <Self as sp_api::ApiExt<Block>>::StateBackend: sp_api::StateBackend<BlakeTwo256>,
{
}

impl<Api> RuntimeApiCollection for Api
where
    Api: sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block>
        + sp_api::ApiExt<Block, Error = sp_blockchain::Error>
        + sp_consensus_babe::BabeApi<Block>
        + sp_finality_grandpa::GrandpaApi<Block>
        + sp_block_builder::BlockBuilder<Block>
        + frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce>
        + pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance>
        + sp_api::Metadata<Block>
        + sp_offchain::OffchainWorkerApi<Block>
        + sp_session::SessionKeys<Block>
        + sp_authority_discovery::AuthorityDiscoveryApi<Block>
        + xpallet_assets_rpc_runtime_api::XAssetsApi<Block, AccountId, Balance>
        + xpallet_dex_spot_rpc_runtime_api::XSpotApi<Block, AccountId, Balance, BlockNumber, Balance>
        + xpallet_gateway_common_rpc_runtime_api::XGatewayCommonApi<Block, AccountId, Balance>
        + xpallet_gateway_records_rpc_runtime_api::XGatewayRecordsApi<
            Block,
            AccountId,
            Balance,
            BlockNumber,
        > + xpallet_mining_staking_rpc_runtime_api::XStakingApi<
            Block,
            AccountId,
            Balance,
            VoteWeight,
            BlockNumber,
        > + xpallet_mining_asset_rpc_runtime_api::XMiningAssetApi<
            Block,
            AccountId,
            Balance,
            MiningWeight,
            BlockNumber,
        > + xpallet_transaction_fee_rpc_runtime_api::XTransactionFeeApi<Block, Balance>,
    <Self as sp_api::ApiExt<Block>>::StateBackend: sp_api::StateBackend<BlakeTwo256>,
{
}
