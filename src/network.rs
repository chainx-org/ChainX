// Copyright 2018 chainpool

use clap;
use std::iter;
use std::net::Ipv4Addr;
use Arc;

use substrate_network;
use substrate_network::{Params, Roles, TransactionPool};
use substrate_network_libp2p;
use substrate_network_libp2p::Protocol;

use chainx_network::{ChainXProtocol, NetworkService, CHAINX_PROTOCOL_ID};

pub type NetworkParam = Params<super::Block, ChainXProtocol, super::Hash>;

pub fn build_network(
    port: u16,
    boot_nodes: Vec<String>,
    client: Arc<super::client::TClient>,
    tx_pool: Arc<TransactionPool<super::Hash, super::Block>>,
    multi_address: clap::Values<'_>,
    net_config_path: &str,
    is_validator: bool,
) -> Arc<NetworkService> {
    let mut net_conf = substrate_network_libp2p::NetworkConfiguration::new();
    net_conf.listen_addresses = vec![];
    for addr in multi_address {
        let addr = addr
            .parse()
            .map_err(|_| "Invalid listen multiaddress")
            .unwrap();
        net_conf.listen_addresses.push(addr);
    }
    if net_conf.listen_addresses.is_empty() {
        net_conf.listen_addresses = vec![iter::once(Protocol::Ip4(Ipv4Addr::new(0, 0, 0, 0)))
            .chain(iter::once(Protocol::Tcp(port)))
            .collect()];
    }
    net_conf.boot_nodes = boot_nodes;
    net_conf.net_config_path = Some(net_config_path.to_string());
    let mut config = substrate_network::ProtocolConfig::default();
    if is_validator {
        config.roles = Roles::AUTHORITY;
    }
    let param = NetworkParam {
        config,
        network_config: net_conf,
        chain: client,
        on_demand: None,
        transaction_pool: tx_pool,
        specialization: ChainXProtocol::new(),
    };
    NetworkService::new(param, CHAINX_PROTOCOL_ID).unwrap()
}
