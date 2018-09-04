// Copyright 2018 chainpool

extern crate substrate_client_db as client_db;
extern crate substrate_state_db as state_db;
extern crate substrate_state_machine as state_machine;
extern crate substrate_client;

extern crate chainx_executor;

use self::chainx_executor::NativeExecutor;
use self::state_machine::ExecutionStrategy;
use std::path::PathBuf;
use super::Arc;

const FINALIZATION_WINDOW: u64 = 32;
pub type TBackend = client_db::Backend<super::Block>;
pub type TExecutor = substrate_client::LocalCallExecutor<TBackend, NativeExecutor<chainx_executor::Executor>>;
pub type TClient = substrate_client::Client<TBackend, TExecutor, super::Block>;

pub fn build_client(db_path: &str) -> Arc<TClient> {
    let backend = Arc::new(
        TBackend::new(
            client_db::DatabaseSettings{
                cache_size: None,
                path: PathBuf::from(db_path),
                pruning:state_db::PruningMode::default(),},
                FINALIZATION_WINDOW
        ).unwrap());

    let executor = substrate_client::LocalCallExecutor::new(
        backend.clone(),
        NativeExecutor::<chainx_executor::Executor>::with_heap_pages(8));
    let genesis_config = super::genesis_config::testnet_genesis();

    Arc::new(
        TClient::new(
            backend.clone(),
            executor,
            genesis_config,
            ExecutionStrategy::NativeWhenPossible
        ).unwrap())
}
