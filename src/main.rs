// Copyright 2018 Chainpool.

extern crate substrate_rpc_servers as rpc_server;
extern crate substrate_runtime_primitives;
extern crate substrate_bft as bft;
extern crate substrate_primitives;
extern crate substrate_client;
extern crate substrate_keyring as keyring;

extern crate chainx_consensus as consensus;
extern crate chainx_primitives;
extern crate chainx_runtime;
extern crate chainx_network;
extern crate chainx_rpc;
extern crate chainx_pool;
extern crate chainx_test;

extern crate rhododendron;
extern crate exit_future;
extern crate hex_literal;
extern crate env_logger;
extern crate ed25519;
extern crate tokio;
extern crate ctrlc;
extern crate clap;
#[macro_use]
extern crate log;

mod genesis_config;
mod network;
mod client;
mod cli;

use chainx_network::consensus::ConsensusNetwork;
use chainx_pool::{TransactionPool, PoolApi};
use chainx_primitives::{Block, Hash};
use cli::ChainSpec;

use tokio::runtime::Runtime;
use std::sync::Arc;

fn main() {
    let _ = env_logger::try_init();
    let matches = cli::build_cli().clone().get_matches();
    let chainspec = match matches.value_of("chainspec").unwrap_or("multi") {
        "dev" => { info!("Chainspec is dev mode"); ChainSpec::Dev },
        "local" => { info!("Chainspec is local mode"); ChainSpec::Local },
        "multi" | _ => { info!("Chainspec is multi mode"); ChainSpec::Multi },
    };
    let port = match matches.value_of("port") {
        Some(port) => {
            port.parse()
                .map_err(|_| "Invalid p2p port value specified.")
                .unwrap()
        }
        None => 20222,
    };
    let mut boot_nodes: Vec<String> = Vec::new();
    boot_nodes.extend(matches.values_of("bootnodes").map_or(
        Default::default(),
        |v| {
            v.map(|n| n.to_owned()).collect::<Vec<_>>()
        },
    ));

    let db_path = matches.value_of("db-path").unwrap_or("./.chainx");
    let client = client::build_client(db_path, chainspec);

    let (exit_send, exit) = exit_future::signal();
    let mut runtime = Runtime::new().expect("failed to start runtime on current thread");
    let task_executor = runtime.executor();

    let extrinsic_pool = Arc::new(TransactionPool::new(
        Default::default(),
        PoolApi::default(),
        client.clone(),
    ));

    let network = network::build_network(port, boot_nodes, client.clone(), extrinsic_pool.clone());

    let validator_mode = matches.subcommand_matches("validator").is_some();
    let _consensus = if validator_mode {
        let key = match matches.subcommand_matches("validator").unwrap().value_of("auth").unwrap_or("alice") {
               "alice" => { info!("Auth is alice"); ed25519::Pair::from_seed(b"Alice                           ") },
               "bob" => { info!("Auth is bob"); ed25519::Pair::from_seed(b"Bob                             ") },
               "gavin" => { info!("Auth is gavin"); ed25519::Pair::from_seed(b"Gavin                           ") },
               "satoshi" | _ => { info!("Auth is satoshi"); ed25519::Pair::from_seed(b"Satoshi                         ") },
        };

        let consensus_net = ConsensusNetwork::new(network, client.clone());
        Some(consensus::Service::new(
            client.clone(),
            client.clone(),
            consensus_net,
            extrinsic_pool.inner().clone(),
            task_executor.clone(),
            key,
        ))
    } else {
        None
    };

    let rpc_client = client.clone();
    let handler = || {
        let chain = rpc_server::apis::chain::Chain::new(rpc_client.clone(), task_executor.clone());
        let state = rpc_server::apis::state::State::new(rpc_client.clone(), task_executor.clone());
        let author = rpc_server::apis::author::Author::new(
            rpc_client.clone(),
            extrinsic_pool.inner().clone(),
            task_executor.clone(),
        );
        rpc_server::rpc_handler::<chainx_primitives::Block, chainx_primitives::Hash, _, _, _, _, _>(
            state,
            chain,
            author,
            chainx_rpc::default_rpc_config(),
            )
    };

    let rpc_interface: &str = "127.0.0.1";
    let ws_interface: &str = "127.0.0.1";
    let rpc_http = Some(
        cli::parse_address(&format!("{}:{}", rpc_interface, 8081), "rpc-port", &matches)
            .unwrap(),
    );
    let rpc_ws = Some(
        cli::parse_address(&format!("{}:{}", ws_interface, 8082), "ws-port", &matches)
            .unwrap(),
    );
    let _rpc_http = chainx_rpc::maybe_start_server(
        rpc_http,
        |address| rpc_server::start_http(address, handler()),
    );
    let _rpc_ws =
        chainx_rpc::maybe_start_server(
        rpc_ws,
        |address| rpc_server::start_ws(address, handler()));

    let _ = runtime.block_on(exit);
    exit_send.fire();
}
