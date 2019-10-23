// Copyright 2018 Chainpool
use crate::service;
use log::warn;
use rpc;
use std::io;
use std::net::SocketAddr;
use substrate_service::{ComponentBlock, ComponentExHash, TaskExecutor};

fn maybe_start_server<T, F>(address: Option<SocketAddr>, start: F) -> Result<Option<T>, io::Error>
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

pub trait Rpc {
    fn start_rpc(
        &self,
        task_executor: TaskExecutor,
    ) -> (
        Result<Option<rpc::HttpServer>, io::Error>,
        Result<Option<rpc::WsServer>, io::Error>,
    );
}

impl Rpc for substrate_service::LightComponents<service::Factory> {
    fn start_rpc(
        &self,
        task_executor: TaskExecutor,
    ) -> (
        Result<Option<rpc::HttpServer>, io::Error>,
        Result<Option<rpc::WsServer>, io::Error>,
    ) {
        let config = &self.config;
        let system_info = rpc::apis::system::SystemInfo {
            chain_name: config.chain_spec.name().into(),
            impl_name: config.impl_name.into(),
            impl_version: config.impl_version.into(),
            properties: config.chain_spec.properties(),
        };

        let handler = || {
            let client = self.client.clone();
            let subscriptions = rpc::apis::Subscriptions::new(task_executor.clone());
            let state = rpc::apis::state::State::new(client.clone(), subscriptions.clone());
            let chain = rpc::apis::chain::Chain::new(client.clone(), subscriptions.clone());
            let author = rpc::apis::author::Author::new(
                client.clone(),
                self.transaction_pool.clone(),
                subscriptions,
            );
            let system = rpc::apis::system::System::new(
                system_info.clone(),
                self.network.clone().unwrap(),
                //should_have_peers,
                false,
            );

            let chainx = rpc::apis::chainx::ChainX::new(client.clone());
            rpc::rpc_handler::<ComponentBlock<Self>, ComponentExHash<Self>, _, _, _, _, _>(
                state, chain, author, system, chainx,
            )
        };
        let rpc_http: Result<Option<rpc::HttpServer>, io::Error> =
            maybe_start_server(config.rpc_http, |address| {
                rpc::start_http(address, config.rpc_cors.as_ref(), handler())
            });

        let rpc_ws: Result<Option<rpc::WsServer>, io::Error> =
            maybe_start_server(config.rpc_ws, |address| {
                rpc::start_ws(
                    address,
                    config.rpc_ws_max_connections,
                    config.rpc_cors.as_ref(),
                    handler(),
                )
            });
        (rpc_http, rpc_ws)
    }
}

impl Rpc for substrate_service::FullComponents<service::Factory> {
    fn start_rpc(
        &self,
        task_executor: TaskExecutor,
    ) -> (
        Result<Option<rpc::HttpServer>, io::Error>,
        Result<Option<rpc::WsServer>, io::Error>,
    ) {
        let config = &self.config;
        let system_info = rpc::apis::system::SystemInfo {
            chain_name: config.chain_spec.name().into(),
            impl_name: config.impl_name.into(),
            impl_version: config.impl_version.into(),
            properties: config.chain_spec.properties(),
        };

        let handler = || {
            let client = self.client.clone();
            let subscriptions = rpc::apis::Subscriptions::new(task_executor.clone());
            let state = rpc::apis::state::State::new(client.clone(), subscriptions.clone());
            let chain = rpc::apis::chain::Chain::new(client.clone(), subscriptions.clone());
            let author = rpc::apis::author::Author::new(
                client.clone(),
                self.transaction_pool.clone(),
                subscriptions,
            );
            let system = rpc::apis::system::System::new(
                system_info.clone(),
                self.network.clone().unwrap(),
                //should_have_peers,
                false,
            );

            let chainx = rpc::apis::chainx::ChainX::new(client.clone());
            rpc::rpc_handler::<ComponentBlock<Self>, ComponentExHash<Self>, _, _, _, _, _>(
                state, chain, author, system, chainx
            )
        };
        let rpc_http: Result<Option<rpc::HttpServer>, io::Error> =
            maybe_start_server(config.rpc_http, |address| {
                rpc::start_http(address, config.rpc_cors.as_ref(), handler())
            });

        let rpc_ws: Result<Option<rpc::WsServer>, io::Error> =
            maybe_start_server(config.rpc_ws, |address| {
                rpc::start_ws(
                    address,
                    config.rpc_ws_max_connections,
                    config.rpc_cors.as_ref(),
                    handler(),
                )
            });
        (rpc_http, rpc_ws)
    }
}
