// Copyright 2018 chainpool

use chainx_api::TClient;
use chainx_pool::TransactionPool;
use chainx_primitives;
use chainx_rpc;
use chainx_rpc::chainext::ChainExt;
use clap;
use cli;
use jsonrpc_http_server::Server as HttpServer;
use jsonrpc_ws_server::Server as WsServer;
use rpc_server;
use rpc_server::apis::chain::Chain;
use std::io;
use tokio::runtime::TaskExecutor;
use Arc;

pub fn start(
    client: &Arc<TClient>,
    task_executor: &TaskExecutor,
    matches: &clap::ArgMatches,
    extrinsic_pool: &Arc<TransactionPool>,
) -> (
    Result<Option<HttpServer>, io::Error>,
    Result<Option<WsServer>, io::Error>,
) {
    let handler = || {
        let chain = Chain::new(client.clone(), task_executor.clone());
        let chain_ext = ChainExt::new(client.clone(), task_executor.clone());
        let state = rpc_server::apis::state::State::new(client.clone(), task_executor.clone());
        let author = rpc_server::apis::author::Author::new(
            client.clone(),
            extrinsic_pool.inner().clone(),
            task_executor.clone(),
        );
        chainx_rpc::servers::rpc_handler::<
            chainx_primitives::Block,
            chainx_primitives::Hash,
            _,
            _,
            _,
            _,
            _,
            _,
        >(
            state,
            chain,
            chain_ext,
            author,
            chainx_rpc::default_rpc_config(),
        )
    };
    let rpc_interface: &str = "127.0.0.1";
    let ws_interface: &str = "127.0.0.1";
    let rpc_http_addr = Some(
        cli::parse_address(&format!("{}:{}", rpc_interface, 8081), "rpc-port", &matches).unwrap(),
    );
    let rpc_ws_addr = Some(
        cli::parse_address(&format!("{}:{}", ws_interface, 8082), "ws-port", &matches).unwrap(),
    );

    let rpc_http: Result<Option<HttpServer>, io::Error> =
        chainx_rpc::maybe_start_server(rpc_http_addr, |address| {
            chainx_rpc::servers::start_http(address, handler())
        });

    let rpc_ws: Result<Option<WsServer>, io::Error> =
        chainx_rpc::maybe_start_server(rpc_ws_addr, |address| {
            chainx_rpc::servers::start_ws(address, handler())
        });

    (rpc_http, rpc_ws)
}
