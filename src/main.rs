// Copyright 2018 Chainpool.

extern crate substrate_bft as bft;
extern crate substrate_client;
extern crate substrate_client_db as client_db;
extern crate substrate_keyring as keyring;
extern crate substrate_network;
extern crate substrate_network_libp2p;
extern crate substrate_primitives;
extern crate substrate_rpc_servers as rpc_server;
#[macro_use]
extern crate substrate_telemetry as tel;
extern crate sr_primitives;
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

extern crate ansi_term;
extern crate clap;
extern crate ctrlc;
extern crate env_logger;
extern crate exit_future;
extern crate hex_literal;
extern crate jsonrpc_http_server;
extern crate jsonrpc_ws_server;
extern crate names;
extern crate parity_codec as codec;
extern crate rhododendron;
extern crate sysinfo;
extern crate tokio;
#[macro_use]
extern crate slog;
#[macro_use]
extern crate log;

mod cli;
mod client;
mod genesis_config;
mod network;
mod rpc;
mod telemetry;

use substrate_client::BlockchainEvents;
use substrate_primitives::{ed25519, ed25519::Pair, storage::StorageKey, twox_128};

use chainx_api::TClient;
use chainx_network::consensus::ConsensusNetwork;
use chainx_pool::{Pool, PoolApi, TransactionPool};
use chainx_primitives::{Block, BlockId, Hash, Timestamp};
use chainx_runtime::{BlockPeriod, Runtime as ChainXRuntime, StorageValue};
use cli::ChainSpec;

use codec::Decode;
use names::{Generator, Name};
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
    let pruning = match matches.value_of("pruning").unwrap_or("constrained") {
         "archiveall" => {
             info!("Pruning is ArchiveAll mode");
             state_db::PruningMode::ArchiveAll
         }
         "archivecanonical" => {
             info!("Pruning is ArchiveCanonical mode");
             state_db::PruningMode::ArchiveCanonical
         }
         "constrained" | _ => {
             info!("Pruning is Constrained mode");
             state_db::PruningMode::default()
         }
     };
    let port = match matches.value_of("port") {
        Some(port) => port
            .parse()
            .map_err(|_| "invalid p2p port value specified.")
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
    let client = client::build_client(db_path, chainspec, pruning);
    
    let (exit_send, exit) = exit_future::signal();
    let mut runtime = Runtime::new().expect("failed to start runtime on current thread");
    let task_executor = runtime.executor();

    let extrinsic_pool = Arc::new(TransactionPool::new(
        Default::default(),
        PoolApi::new(client.clone() as Arc<TClient>),
        client.clone(),
    ));

    let validator_mode = matches.subcommand_matches("validator").is_some();
    let multi_address = matches.values_of("listen-addr").unwrap_or_default();
    let network = network::build_network(
        port,
        boot_nodes,
        client.clone(),
        extrinsic_pool.clone(),
        multi_address,
        db_path,
        validator_mode,
    );

    {
        // block notifications
        let network = network.clone();
        let txpool = extrinsic_pool.clone();

        let events = client
            .import_notification_stream()
            .for_each(move |notification| {
                network.on_block_imported(notification.hash, &notification.header);
                txpool
                    .inner()
                    .cull(&BlockId::hash(notification.hash))
                    .map_err(|e| warn!("Error removing extrinsics: {:?}", e))?;
                Ok(())
            })
            .select(exit.clone())
            .then(|_| Ok(()));
        task_executor.spawn(events);
    }

    {
        // extrinsic notifications
        let network = network.clone();
        let txpool = extrinsic_pool.clone();
        let events = txpool
            .inner()
            .import_notification_stream()
            // TODO [ToDr] Consider throttling?
            .for_each(move |_| {
                network.trigger_repropagate();
                Ok(())
            })
            .select(exit.clone())
            .then(|_| Ok(()));

        task_executor.spawn(events);
    }

    let _consensus = if validator_mode {
        let mut key = match matches
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
            "charlie" => {
                info!("Auth is charlie");
                ed25519::Pair::from_seed(b"Charlie                         ")
            }
            "dave" => {
                info!("Auth is dave");
                ed25519::Pair::from_seed(b"Dave                            ")
            }
            "eve" => {
                info!("Auth is eve");
                ed25519::Pair::from_seed(b"Eve                             ")
            }
            "ferdie" => {
                info!("Auth is ferdie");
                ed25519::Pair::from_seed(b"Ferdie                          ")
            }
            "satoshi" | _ => {
                info!("Auth is satoshi");
                ed25519::Pair::from_seed(b"Satoshi                         ")
            }
        };

        if let Some(seed) = matches.value_of("key") {
            let generate_from_seed = |seed: &str| -> Pair {
                let mut s: [u8; 32] = [' ' as u8; 32];
                let len = ::std::cmp::min(32, seed.len());
                &mut s[..len].copy_from_slice(&seed.as_bytes()[..len]);
                Pair::from_seed(&s)
            };
            key = generate_from_seed(seed);
        }

        let block_id = BlockId::number(client.info().unwrap().chain.best_number);
        // TODO: this needs to be dynamically adjustable
        let block_delay = client
            .storage(
                &block_id,
                &StorageKey(twox_128(BlockPeriod::<ChainXRuntime>::key()).to_vec()),
            )
            .unwrap()
            .and_then(|data| Timestamp::decode(&mut data.0.as_slice()))
            .unwrap_or_else(|| {
                warn!("Block period is missing in the storage.");
                3
            });

        let consensus_net = ConsensusNetwork::new(network.clone(), client.clone());
        Some(consensus::Service::new(
            client.clone(),
            client.clone(),
            consensus_net,
            extrinsic_pool.inner().clone(),
            task_executor.clone(),
            key,
            block_delay,
        ))
    } else {
        None
    };

    let (_rpc_http, _rpc_ws) = rpc::start(&client, &task_executor, &matches, &extrinsic_pool);

    if matches.is_present("telemetry") {
        let telemetry_url = match matches.value_of("telemetry-url") {
            Some(url) => Some(url.to_owned()),
            None => Some("wss://telemetry.polkadot.io/submit/".to_owned()),
        };
        let name = match matches.value_of("name") {
            None => Generator::with_naming(Name::Numbered).next().unwrap(),
            Some(name) => name.into(),
        };
        let _telemetry = telemetry::build_telemetry(telemetry_url, validator_mode, name);
        telemetry::run_telemetry(network, client, extrinsic_pool.inner(), task_executor);
        let _ = runtime.block_on(exit.clone());
    } else {
        let _ = runtime.block_on(exit);
    }

    exit_send.fire();
}
