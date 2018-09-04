// Copyright 2018 chainpool

extern crate substrate_network_libp2p;
extern crate substrate_network;
extern crate chainx_network;

use self::chainx_network::{ChainXProtocol, NetworkService, CHAINX_PROTOCOL_ID};

use self::substrate_network::{Params, TransactionPool};
use self::substrate_network_libp2p::AddrComponent;

use std::net::Ipv4Addr;
use std::iter;
use super::Arc;

pub type NetworkParam = Params<super::Block, ChainXProtocol, super::Hash>;

pub fn build_network(port: u16, boot_nodes: Vec<String>,
                     client: Arc<super::client::TClient>,
                     tx_pool: Arc<TransactionPool<super::Hash, super::Block>>)
      -> Arc<NetworkService> {
    let mut net_conf = substrate_network_libp2p::NetworkConfiguration::new();
    net_conf.listen_addresses = vec![iter::once(AddrComponent::IP4(Ipv4Addr::new(127, 0, 0, 1)))
        .chain(iter::once(AddrComponent::TCP(port)))
        .collect()];
    net_conf.boot_nodes = boot_nodes;
    let param = NetworkParam {
        config: substrate_network::ProtocolConfig::default(),
        network_config: net_conf,
        chain: client,
        on_demand: None,
        transaction_pool: tx_pool,
        specialization: ChainXProtocol::new(),
    };
    NetworkService::new(param, CHAINX_PROTOCOL_ID).unwrap()
} 
