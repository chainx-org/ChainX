// Copyright 2018 chainpool.

//! chainx-specific network implementation.
//!
//! This manages gossip of consensus messages for BFT, communication between validators
//! and more.

extern crate substrate_bft as bft;
extern crate substrate_network;
extern crate substrate_primitives;

extern crate chainx_api;
extern crate chainx_consensus;
extern crate chainx_primitives;

extern crate futures;
extern crate rhododendron;
extern crate tokio;

#[macro_use]
extern crate log;

pub mod consensus;

use chainx_primitives::{Block, Hash, Header};
use substrate_network::consensus_gossip::ConsensusGossip;
use substrate_network::specialization::Specialization;
use substrate_network::StatusMessage as GenericFullStatus;
use substrate_network::{generic_message, message};
use substrate_network::{Context, NodeIndex, Severity};

type FullStatus = GenericFullStatus<Block>;

/// Specialization of the network service for the chainx protocol.
pub type NetworkService = ::substrate_network::Service<Block, ChainXProtocol, Hash>;

pub const CHAINX_PROTOCOL_ID: substrate_network::ProtocolId = *b"pcx";

/// ChainX protocol attachment for substrate.
pub struct ChainXProtocol {
    consensus_gossip: ConsensusGossip<Block>,
    live_consensus: Option<Hash>,
}

impl ChainXProtocol {
    /// Instantiate a chainx protocol handler.
    pub fn new() -> Self {
        ChainXProtocol {
            consensus_gossip: ConsensusGossip::new(),
            live_consensus: None,
        }
    }

    /// Note new consensus session.
    fn new_consensus(&mut self, parent_hash: Hash) {
        let old_consensus = self.live_consensus.take();
        self.live_consensus = Some(parent_hash);
        self.consensus_gossip
            .collect_garbage(|topic| old_consensus.as_ref().map_or(true, |h| topic != h));
    }
}

impl Specialization<Block> for ChainXProtocol {
    fn status(&self) -> Vec<u8> {
        Vec::new()
    }

    fn on_connect(&mut self, ctx: &mut Context<Block>, who: NodeIndex, status: FullStatus) {
        self.consensus_gossip.new_peer(ctx, who, status.roles);
    }

    fn on_disconnect(&mut self, ctx: &mut Context<Block>, who: NodeIndex) {
        self.consensus_gossip.peer_disconnected(ctx, who);
    }

    fn on_message(
        &mut self,
        ctx: &mut Context<Block>,
        who: NodeIndex,
        message: message::Message<Block>,
    ) {
        match message {
            generic_message::Message::BftMessage(msg) => {
                trace!(target: "p_net", "ChainX BFT message from {}: {:?}", who, msg);
                // TODO: check signature here? what if relevant block is unknown?
                self.consensus_gossip.on_bft_message(ctx, who, msg)
            }
            generic_message::Message::ChainSpecific(_) => {
                trace!(target: "p_net", "Bad message from {}", who);
                ctx.report_peer(who, Severity::Bad("Invalid CahinX protocol message format"));
            }
            _ => {}
        }
    }

    fn on_abort(&mut self) {
        self.consensus_gossip.abort();
    }

    fn maintain_peers(&mut self, _ctx: &mut Context<Block>) {
        self.consensus_gossip.collect_garbage(|_| true);
    }

    fn on_block_imported(&mut self, _ctx: &mut Context<Block>, _hash: Hash, _header: &Header) {}
}
