// Copyright 2018 chainpool

extern crate substrate_network;
extern crate substrate_network_libp2p;

use self::substrate_network::{NodeIndex, Context, message,
                              Service, Params, TransactionPool};
use self::substrate_network_libp2p::AddrComponent;
use self::substrate_network::specialization::Specialization;
use self::substrate_network::StatusMessage as GenericFullStatus;

use std::net::Ipv4Addr;
use std::iter;
use super::Arc;

pub type NetworkService = Service<super::Block, Protocol, super::Hash>;
pub type NetworkParam = Params<super::Block, Protocol, super::Hash>;
type FullStatus = GenericFullStatus<super::Block>;

const CHAINX_PROTOCOL_ID: substrate_network::ProtocolId = *b"exc";
pub struct Protocol;

impl Protocol {
    pub fn new() -> Self {
        Protocol{}
    }
}

impl Specialization<super::Block> for Protocol {
    fn status(&self) -> Vec<u8> {
        println!("status");
        vec![2, 2]
    }

    fn on_connect(&mut self, _ctx: &mut Context<super::Block>, _who: NodeIndex, _status: FullStatus) {
        println!("on_connect");
    }

    fn on_disconnect(&mut self, _ctx: &mut Context<super::Block>, _who: NodeIndex) {
        println!("on_disconnect");
    }

    fn on_message(
        &mut self,
        _ctx: &mut Context<super::Block>,
        _who: NodeIndex,
        _message: message::Message<super::Block>,
        ) {
        println!("on_message");
    }

    fn on_abort(&mut self) {
        println!("on_abort!");
    }

    fn maintain_peers(&mut self, _ctx: &mut Context<super::Block>) {
        println!("maintain_peers!");
    }

    fn on_block_imported(&mut self, _ctx: &mut Context<super::Block>, _hash: super::Hash, _header: &super::Header) {
        println!("on_block_imported!");
    }
}


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
        specialization: Protocol::new(),
    };
    NetworkService::new(param, CHAINX_PROTOCOL_ID).unwrap()
} 
