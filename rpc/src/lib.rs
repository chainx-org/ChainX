// Copyright 2018 Chainpool.

extern crate substrate_rpc as rpc;
#[macro_use]
extern crate log;

extern crate chainx_api;
extern crate jsonrpc_core;
extern crate jsonrpc_http_server as http;
extern crate jsonrpc_pubsub as pubsub;
extern crate jsonrpc_ws_server as ws;
extern crate serde;
extern crate sr_primitives as runtime_primitives;
extern crate substrate_client as client;
extern crate substrate_primitives as primitives;
extern crate substrate_rpc;
pub extern crate substrate_rpc as apis;
extern crate substrate_rpc_servers as rpc_server;
extern crate tokio;
#[macro_use]
extern crate error_chain;

#[macro_use]
extern crate jsonrpc_macros;

pub mod chainext;
pub mod servers;

use std::io;
use std::net::SocketAddr;

const CHAIN_NAME: &'static str = "ChainX POC-3";
const IMPL_NAME: &'static str = "bud";
const IMPL_VERSION: &'static str = "v0.3.0";

#[derive(Clone)]
pub struct RpcConfig {
    chain_name: String,
    impl_name: &'static str,
    impl_version: &'static str,
}

impl rpc::system::SystemApi for RpcConfig {
    fn system_name(&self) -> rpc::system::error::Result<String> {
        Ok(self.impl_name.into())
    }

    fn system_version(&self) -> rpc::system::error::Result<String> {
        Ok(self.impl_version.into())
    }

    fn system_chain(&self) -> rpc::system::error::Result<String> {
        Ok(self.chain_name.clone())
    }
}

pub fn default_rpc_config() -> RpcConfig {
    RpcConfig {
        chain_name: CHAIN_NAME.to_string(),
        impl_name: IMPL_NAME,
        impl_version: IMPL_VERSION,
    }
}

pub fn maybe_start_server<T, F>(
    address: Option<SocketAddr>,
    start: F,
) -> Result<Option<T>, io::Error>
where
    F: Fn(&SocketAddr) -> Result<T, io::Error>,
{
    Ok(match address {
        Some(mut address) => Some(start(&address).or_else(|e| match e.kind() {
            io::ErrorKind::AddrInUse | io::ErrorKind::PermissionDenied => {
                warn!("Unable to bind server to {}. Trying random port.", address);
                address.set_port(0);
                start(&address)
            }
            _ => Err(e),
        })?),
        None => None,
    })
}
