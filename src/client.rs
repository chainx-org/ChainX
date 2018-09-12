// Copyright 2018 chainpool

use std::path::PathBuf;
use Arc;

use substrate_client;
use client_db;
use state_db;

pub use chainx_api::{TBackend, TExecutor, TClient, TClientBlockBuilder};
use state_machine::ExecutionStrategy;
use chainx_executor::NativeExecutor;
use cli::ChainSpec;

const FINALIZATION_WINDOW: u64 = 32;

pub fn build_client(db_path: &str, chainspec: ChainSpec) -> Arc<TClient> {
    let backend = Arc::new(
        TBackend::new(
            client_db::DatabaseSettings {
                cache_size: None,
                path: PathBuf::from(db_path),
                pruning: state_db::PruningMode::default(),
            },
            FINALIZATION_WINDOW,
        ).unwrap(),
    );

    let executor = substrate_client::LocalCallExecutor::new(backend.clone(), NativeExecutor::new());

    let genesis_config = super::genesis_config::testnet_genesis(chainspec);


    Arc::new(
        TClient::new(
            backend.clone(),
            executor,
            genesis_config,
            ExecutionStrategy::NativeWhenPossible,
        ).unwrap(),
    )
}
