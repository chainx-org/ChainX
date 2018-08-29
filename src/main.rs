// Copyright 2018 Chainpool.

extern crate substrate_network;
extern crate substrate_network_libp2p;
extern crate substrate_runtime_primitives;
extern crate substrate_primitives;
extern crate substrate_client as client;
extern crate substrate_bft as bft;
extern crate substrate_rpc_servers as rpc_server;

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

use substrate_network_libp2p::AddrComponent;
use substrate_network::specialization::Specialization;
use substrate_network::{NodeIndex, Context, message};
use substrate_network::StatusMessage as GenericFullStatus;
use chainx_primitives::{Block, Header, Hash};
use chainx_runtime::{GenesisConfig, ConsensusConfig, CouncilConfig, DemocracyConfig,
                     SessionConfig, StakingConfig, TimestampConfig};

use futures::{Future, Stream};
use tokio::runtime::Runtime;
use tokio::timer::Interval;
use clap::{Arg, App, SubCommand};
use std::sync::Arc;
use std::iter;
use std::net::{Ipv4Addr, IpAddr, SocketAddr};
use std::time::{Duration, Instant};

use chainx_pool::pool::{TransactionPool, PoolApi};


pub struct Protocol;

const TIMER_INTERVAL_MS: u64 = 5000;

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

const DOT_PROTOCOL_ID: substrate_network::ProtocolId = *b"exc";

fn genesis_config() -> GenesisConfig {
    let god_key = hex!("3d866ec8a9190c8343c2fc593d21d8a6d0c5c4763aaab2349de3a6111d64d124");
    let genesis_config = GenesisConfig {
        consensus: Some(ConsensusConfig {
            code: include_bytes!(
                "../runtime/wasm/target/wasm32-unknown-unknown/release/chainx_runtime.compact.wasm"
            ).to_vec(),
            authorities: vec![ed25519::Pair::from_seed(&god_key).public().into()],
        }),
        system: None,
        session: Some(SessionConfig {
            validators: vec![god_key.clone().into()],
            session_length: 720, // that's 1 hour per session.
        }),
        staking: Some(StakingConfig {
            current_era: 0,
            intentions: vec![],
            transaction_base_fee: 100,
            transaction_byte_fee: 1,
            transfer_fee: 0,
            creation_fee: 0,
            reclaim_rebate: 0,
            existential_deposit: 500,
            balances: vec![(god_key.clone().into(), 1u128 << 63)]
                .into_iter()
                .collect(),
            validator_count: 12,
            minimum_validator_count: 0,
            sessions_per_era: 24, // 24 hours per era.
            bonding_duration: 90, // 90 days per bond.
            early_era_slash: 10000,
            session_reward: 100,
            offline_slash_grace: 0,
        }),
        democracy: Some(DemocracyConfig {
            launch_period: 120 * 24 * 14, // 2 weeks per public referendum
            voting_period: 120 * 24 * 28, // 4 weeks to discuss & vote on an active referendum
            minimum_deposit: 1000, // 1000 as the minimum deposit for a referendum
        }),
        council: Some(CouncilConfig {
            active_council: vec![],
            candidacy_bond: 1000, // 1000 to become a council candidate
            voter_bond: 100, // 100 down to vote for a candidate
            present_slash_per_voter: 1, // slash by 1 per voter for an invalid presentation.
            carry_count: 24, // carry over the 24 runners-up to the next council election
            presentation_duration: 120 * 24, // one day for presenting winners.
            // one week period between possible council elections.
            approval_voting_period: 7 * 120 * 24,
            term_duration: 180 * 120 * 24, // 180 day term duration for the council.
            // start with no council: we'll raise this once the stake has been dispersed a bit.
            desired_seats: 0,
            // one addition vote should go by before an inactive voter can be reaped.
            inactive_grace_period: 1,
            // 90 day cooling off period if council member vetoes a proposal.
            cooloff_period: 90 * 120 * 24,
            voting_period: 7 * 120 * 24, // 7 day voting period for council members.
        }),
        timestamp: Some(TimestampConfig {
                    period: 5,                  // 5 second block time.
                }),
    };
    genesis_config
}

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
    let matches = App::new("chainx")
        .version("0.1.0")
        .arg(
            Arg::with_name("port")
                .long("port")
                .value_name("PORT")
                .help("Specify p2p protocol TCP port")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("bootnodes")
                .long("bootnodes")
                .value_name("URL")
                .help("Specify a list of bootnodes")
                .takes_value(true)
                .multiple(true),
        )
        .subcommand(SubCommand::with_name("validator").help(
            "Enable validator mode",
        ))
        .get_matches();
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

    let executor = chainx_executor::NativeExecutor::with_heap_pages(8);
    let client = Arc::new(
        client::new_in_mem::<
            chainx_executor::NativeExecutor<chainx_executor::Executor>,
            Block,
            _,
        >(executor, genesis_config()).unwrap(),
    );


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
    let work = interval
        .map_err(|e| debug!("Timer error: {:?}", e))
        .for_each(move |_| {
            let best_header = client.best_block_header().unwrap();
            println!("Best block: #{}", best_header.number);
            if let Some(_) = matches.subcommand_matches("validator") {
                let builder = client.new_block().unwrap();
                let block = builder.bake().unwrap();
                let block_header = block.header.clone();
                let hash = block_header.hash();
                let justification = fake_justify(&block.header);
                let justified = client
                    .check_justification(block.header, justification)
                    .unwrap();
                client
                    .import_block(
                        client::BlockOrigin::NetworkBroadcast,
                        justified,
                        Some(block.extrinsics),
                    )
                    .unwrap();
                network.on_block_imported(hash, &block_header);
            }
            Ok(())
        });

    let rpc_http = Some(SocketAddr::new(
        IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
        8081,
    ));
    let rpc_ws = Some(SocketAddr::new(
        IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
        8082,
    ));
    chainx_rpc::maybe_start_server(
        rpc_http,
        |address| rpc_server::start_http(address, handler()),
    ).unwrap();
    chainx_rpc::maybe_start_server(rpc_ws, |address| rpc_server::start_ws(address, handler()))
        .unwrap();

    let _ = runtime.block_on(exit.until(work).map(|_| ()));
    exit_send.fire();
}
