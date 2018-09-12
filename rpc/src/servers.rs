//! chainx RPC servers.

use apis;
use http;
use pubsub;
use runtime_primitives;
use runtime_primitives::traits::{Block as BlockT, NumberFor};
use serde;
use std;
use std::io;
use ws;

use chainext;

type Metadata = apis::metadata::Metadata;
type RpcHandler = pubsub::PubSubHandler<Metadata>;
pub type HttpServer = http::Server;
pub type WsServer = ws::Server;

/// Construct rpc `IoHandler`
pub fn rpc_handler<Block: BlockT, ExHash, PendingExtrinsics, S, C, CE, A, Y>(
    state: S,
    chain: C,
    chainext: CE,
    author: A,
    system: Y,
) -> RpcHandler
where
    Block: BlockT + 'static,
    ExHash: Send
        + Sync
        + 'static
        + runtime_primitives::Serialize
        + runtime_primitives::DeserializeOwned,
    PendingExtrinsics: serde::Serialize + serde::de::DeserializeOwned + Send + Sync + 'static,
    S: apis::state::StateApi<Block::Hash, Metadata = Metadata>,
    C: apis::chain::ChainApi<
        Block::Hash,
        Block::Header,
        NumberFor<Block>,
        Block::Extrinsic,
        Metadata = Metadata,
    >,
    CE: chainext::ChainApiExt<Block::Hash, Block::Header, NumberFor<Block>, Block::Extrinsic>,
    A: apis::author::AuthorApi<ExHash, Block::Extrinsic, PendingExtrinsics, Metadata = Metadata>,
    Y: apis::system::SystemApi,
{
    let mut io = pubsub::PubSubHandler::default();
    io.extend_with(state.to_delegate());
    io.extend_with(chain.to_delegate());
    io.extend_with(chainext.to_delegate());
    io.extend_with(author.to_delegate());
    io.extend_with(system.to_delegate());
    io
}

/// Start HTTP server listening on given address.
pub fn start_http(addr: &std::net::SocketAddr, io: RpcHandler) -> io::Result<http::Server> {
    http::ServerBuilder::new(io)
        .threads(4)
        .rest_api(http::RestApi::Unsecure)
        .cors(http::DomainsValidation::Disabled)
        .start_http(addr)
}

/// Start WS server listening on given address.
pub fn start_ws(addr: &std::net::SocketAddr, io: RpcHandler) -> io::Result<ws::Server> {
    ws::ServerBuilder::with_meta_extractor(io, |context: &ws::RequestContext| {
        Metadata::new(context.sender())
    }).start(addr)
    .map_err(|err| match err {
        ws::Error(ws::ErrorKind::Io(io), _) => io,
        ws::Error(ws::ErrorKind::ConnectionClosed, _) => io::ErrorKind::BrokenPipe.into(),
        ws::Error(e, _) => {
            error!("{}", e);
            io::ErrorKind::Other.into()
        }
    })
}
