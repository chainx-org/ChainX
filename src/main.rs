// Copyright 2018 Chainpool.

extern crate substrate_network;
extern crate substrate_network_libp2p;
extern crate substrate_runtime_primitives;
extern crate substrate_primitives;
extern crate substrate_client as client;
extern crate substrate_bft as bft;
extern crate substrate_rpc_servers as rpc_server;
extern crate substrate_client_db as client_db;
extern crate substrate_state_machine as state_machine;
extern crate substrate_state_db as state_db;

extern crate chainx_primitives;
extern crate chainx_executor;
extern crate chainx_runtime;
extern crate chainx_rpc;
extern crate chainx_pool;

extern crate futures;
extern crate tokio;
extern crate exit_future;
extern crate ctrlc;
extern crate rhododendron;
#[macro_use]
extern crate hex_literal;
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate clap;
extern crate ed25519;

use std::sync::Arc;
use std::iter;
use std::net::Ipv4Addr;
use std::time::{Duration, Instant};
use std::path::PathBuf;

use substrate_network_libp2p::AddrComponent;
use substrate_network::specialization::Specialization;
use substrate_network::{NodeIndex, Context, message};
use substrate_network::StatusMessage as GenericFullStatus;
use chainx_primitives::{Block, Header, Hash};
use chainx_executor::NativeExecutor;

use futures::{Future, Stream};
use tokio::runtime::Runtime;
use tokio::timer::Interval;
use chainx_pool::pool::{TransactionPool, PoolApi};
use state_machine::ExecutionStrategy;

mod genesis_config;
mod cli;

pub struct Protocol;

const TIMER_INTERVAL_MS: u64 = 5000;
const FINALIZATION_WINDOW: u64 = 32;

type FullStatus = GenericFullStatus<Block>;

impl Protocol {
    pub fn new() -> Self {
        Protocol{}
    }
}

impl Specialization<Block> for Protocol {
    fn status(&self) -> Vec<u8> {
        println!("status");
        vec![2, 2]
    }

    fn on_connect(&mut self, _ctx: &mut Context<Block>, _who: NodeIndex, _status: FullStatus) {
        println!("on_connect");
    }

    fn on_disconnect(&mut self, _ctx: &mut Context<Block>, _who: NodeIndex) {
        println!("on_disconnect");
    }

    fn on_message(
        &mut self,
        _ctx: &mut Context<Block>,
        _who: NodeIndex,
        _message: message::Message<Block>,
        ) {
        println!("on_message");
    }

    fn on_abort(&mut self) {
        println!("on_abort!");
    }

    fn maintain_peers(&mut self, _ctx: &mut Context<Block>) {
        println!("maintain_peers!");
    }

    fn on_block_imported(&mut self, _ctx: &mut Context<Block>, _hash: Hash, _header: &Header) {
        println!("on_block_imported!");
    }
}

pub type NetworkService = substrate_network::Service<Block, Protocol, Hash>;

pub type NetworkParam = substrate_network::Params<Block, Protocol, Hash>;

pub type TBackend = client_db::Backend<Block>;
pub type TExecutor = client::LocalCallExecutor<TBackend, NativeExecutor<chainx_executor::Executor>>;

const DOT_PROTOCOL_ID: substrate_network::ProtocolId = *b"exc";

pub fn fake_justify(header: &Header) -> bft::UncheckedJustification<Hash> {
    let hash = header.hash();
    let authorities_keys = vec![
        ed25519::Pair::from_seed(&hex!(
                "3d866ec8a9190c8343c2fc593d21d8a6d0c5c4763aaab2349de3a6111d64d124"
        )),
    ];

    bft::UncheckedJustification::new(
        hash,
        authorities_keys
        .iter()
        .map(|key| {
            let msg = bft::sign_message::<Block>(
                ::rhododendron::Vote::Commit(1, hash).into(),
                key,
                header.parent_hash,
                );

            match msg {
                ::rhododendron::LocalizedMessage::Vote(vote) => vote.signature,
                _ => panic!("signing vote leads to signed vote"),
            }
        })
        .collect(),
        1,
        )
}

//#[warn(unused_must_use)]
fn main() {
    let matches = cli::build_cli().clone().get_matches();
    let port = match matches.value_of("port") {
        Some(port) => {
            port.parse()
                .map_err(|_| "Invalid p2p port value specified.")
                .unwrap()
        }
        None => 20222,
    };

    let mut net_conf = substrate_network_libp2p::NetworkConfiguration::new();
    net_conf.listen_addresses = vec![iter::once(AddrComponent::IP4(Ipv4Addr::new(127, 0, 0, 1)))
        .chain(iter::once(AddrComponent::TCP(port)))
        .collect()];
    net_conf.boot_nodes.extend(
        matches.values_of("bootnodes").map_or(
            Default::default(),
            |v| v.map(|n| n.to_owned()).collect::<Vec<_>>(),
            ),
            );

    let _ = env_logger::try_init();

    let db_path = matches.value_of("db-path").unwrap_or("./.chainx");
    let backend = Arc::new(
        TBackend::new(
            client_db::DatabaseSettings{
                cache_size: None,
                path: PathBuf::from(db_path),
                pruning:state_db::PruningMode::default(),},
                FINALIZATION_WINDOW
        ).unwrap());

    let executor = client::LocalCallExecutor::new(
        backend.clone(),
        NativeExecutor::<chainx_executor::Executor>::with_heap_pages(8));
    let genesis_config = genesis_config::testnet_genesis();
    let client = Arc::new(
        client::Client::new(
            backend.clone(),
            executor,
            genesis_config,
            ExecutionStrategy::NativeWhenPossible
        ).unwrap());

    let (exit_send, exit) = exit_future::signal();
    let mut runtime = Runtime::new().expect("failed to start runtime on current thread");
    let task_executor = runtime.executor();

    let extrinsic_pool = Arc::new(TransactionPool::new(
            Default::default(),
            PoolApi::default(),
            client.clone(),
            ));

    let rpc_client = client.clone();
    let handler = || {
        let chain = rpc_server::apis::chain::Chain::new(rpc_client.clone(), task_executor.clone());
        let state = rpc_server::apis::state::State::new(rpc_client.clone(), task_executor.clone());
        let author = rpc_server::apis::author::Author::new(
            rpc_client.clone(),
            extrinsic_pool.inner.clone(),
            task_executor.clone(),
            );
        rpc_server::rpc_handler::<chainx_primitives::Block, chainx_primitives::Hash, _, _, _, _, _>(
            state,
            chain,
            author,
            chainx_rpc::default_rpc_config(),
            )
    };

    let param = NetworkParam {
        config: substrate_network::ProtocolConfig::default(),
        network_config: net_conf,
        chain: client.clone(),
        on_demand: None,
        transaction_pool: extrinsic_pool.clone(),
        specialization: Protocol::new(),
    };
    let network = NetworkService::new(param, DOT_PROTOCOL_ID).unwrap();

    let interval = Interval::new(Instant::now(), Duration::from_millis(TIMER_INTERVAL_MS));
    let validator_mode = matches.subcommand_matches("validator").is_some();
    let work = interval
        .map_err(|e| debug!("Timer error: {:?}", e))
        .for_each(move |_| {
            let best_header = client.best_block_header().unwrap();
            println!("Best block: #{}", best_header.number);
            if validator_mode {
                let builder = client.new_block().unwrap();
                let block = builder.bake().unwrap();
                let block_header = block.header.clone();
                let hash = block_header.hash();
                let justification = fake_justify(&block.header);
                let justified = client.check_justification(
                    block.header,
                    justification).unwrap();

                client.import_block(
                    client::BlockOrigin::NetworkBroadcast,
                    justified,
                    Some(block.extrinsics),
                    ).unwrap();

                network.on_block_imported(hash, &block_header);
            }
            Ok(())
        });

    let rpc_interface: &str = "127.0.0.1";
    let ws_interface: &str = "127.0.0.1";

    let rpc_http = Some(cli::parse_address(&format!("{}:{}", rpc_interface, 8081), "rpc-port", &matches).unwrap());
    let rpc_ws = Some(cli::parse_address(&format!("{}:{}", ws_interface, 8082), "ws-port", &matches).unwrap());

    let _rpc_http = chainx_rpc::maybe_start_server(rpc_http,
                                                   |address| rpc_server::start_http(address, handler()));
    let _rpc_ws = chainx_rpc::maybe_start_server(rpc_ws,
                                                 |address| rpc_server::start_ws(address, handler()));

    let _ = runtime.block_on(exit.until(work).map(|_| ()));
    exit_send.fire();
}
