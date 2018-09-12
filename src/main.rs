// Copyright 2018 Chainpool.

extern crate substrate_bft as bft;
extern crate substrate_client;
extern crate substrate_client_db as client_db;
extern crate substrate_keyring as keyring;
extern crate substrate_network;
extern crate substrate_network_libp2p;
extern crate substrate_primitives;
extern crate substrate_rpc_servers as rpc_server;
extern crate substrate_runtime_primitives;
extern crate substrate_state_db as state_db;
extern crate substrate_state_machine as state_machine;

extern crate chainx_api;
extern crate chainx_consensus as consensus;
extern crate chainx_executor;
extern crate chainx_network;
extern crate chainx_pool;
extern crate chainx_primitives;
extern crate chainx_rpc;
extern crate chainx_runtime;
extern crate chainx_test;

extern crate clap;
extern crate ctrlc;
extern crate ed25519;
extern crate env_logger;
extern crate exit_future;
extern crate hex_literal;
extern crate jsonrpc_http_server;
extern crate jsonrpc_ws_server;
extern crate rhododendron;
extern crate tokio;
#[macro_use]
extern crate log;

mod cli;
mod client;
mod genesis_config;
mod network;
mod rpc;

use substrate_client::BlockchainEvents;

use chainx_network::consensus::ConsensusNetwork;
use chainx_pool::{PoolApi, TransactionPool};
use chainx_primitives::{Block, Hash};
use cli::ChainSpec;

use std::sync::Arc;
use tokio::prelude::Future;
use tokio::prelude::Stream;
use tokio::runtime::Runtime;

fn main() {
    let _ = env_logger::try_init();
    let matches = cli::build_cli().clone().get_matches();
    let chainspec = match matches.value_of("chainspec").unwrap_or("multi") {
        "dev" => {
            info!("Chainspec is dev mode");
            ChainSpec::Dev
        }
        "local" => {
            info!("Chainspec is local mode");
            ChainSpec::Local
        }
        "multi" | _ => {
            info!("Chainspec is multi mode");
            ChainSpec::Multi
        }
    };
    let port = match matches.value_of("port") {
        Some(port) => port
            .parse()
            .map_err(|_| "Invalid p2p port value specified.")
            .unwrap(),
        None => 20222,
    };
    let mut boot_nodes: Vec<String> = Vec::new();
    boot_nodes.extend(
        matches
            .values_of("bootnodes")
            .map_or(Default::default(), |v| {
                v.map(|n| n.to_owned()).collect::<Vec<_>>()
            }),
    );

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

    let validator_mode = matches.subcommand_matches("validator").is_some();
    let network = network::build_network(
        port,
        boot_nodes,
        client.clone(),
        extrinsic_pool.clone(),
        validator_mode,
    );

    {
        // block notifications
        let network = network.clone();
        let events = client
            .import_notification_stream()
            .for_each(move |notification| {
                network.on_block_imported(notification.hash, &notification.header);
                Ok(())
            }).select(exit.clone())
            .then(|_| Ok(()));
        task_executor.spawn(events);
    }

    let _consensus = if validator_mode {
        let key = match matches
            .subcommand_matches("validator")
            .unwrap()
            .value_of("auth")
            .unwrap_or("alice")
        {
            "alice" => {
                info!("Auth is alice");
                ed25519::Pair::from_seed(b"Alice                           ")
            }
            "bob" => {
                info!("Auth is bob");
                ed25519::Pair::from_seed(b"Bob                             ")
            }
            "gavin" => {
                info!("Auth is gavin");
                ed25519::Pair::from_seed(b"Gavin                           ")
            }
            "satoshi" | _ => {
                info!("Auth is satoshi");
                ed25519::Pair::from_seed(b"Satoshi                         ")
            }
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

    let (_rpc_http, _rpc_ws) = rpc::start(&client, &task_executor, &matches, &extrinsic_pool);

    let _ = runtime.block_on(exit);
    exit_send.fire();
}
