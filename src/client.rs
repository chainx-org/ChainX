// Copyright 2018 chainpool

use std::path::PathBuf;
use Arc;

use client_db;
use substrate_client;

pub use chainx_api::{TBackend, TClient, TClientBlockBuilder, TExecutor};
use chainx_executor::NativeExecutor;
use cli::ChainSpec;
use state_machine::ExecutionStrategy;
use state_db::PruningMode;

const FINALIZATION_WINDOW: u64 = 32;

pub fn build_client(db_path: &str, chainspec: ChainSpec, pruning: PruningMode) -> Arc<TClient> {
    let backend = Arc::new(
        TBackend::new(
            client_db::DatabaseSettings {
                cache_size: None,
                path: PathBuf::from(db_path),
                pruning: pruning,
            },
            FINALIZATION_WINDOW,
        )
        .unwrap(),
    );

    let executor = substrate_client::LocalCallExecutor::new(backend.clone(), NativeExecutor::new());

    let genesis_config = super::genesis_config::testnet_genesis(chainspec);

    Arc::new(
        TClient::new(
            backend.clone(),
            executor,
            genesis_config,
            ExecutionStrategy::NativeWhenPossible,
            ExecutionStrategy::NativeWhenPossible,
        )
        .unwrap(),
    )
}
