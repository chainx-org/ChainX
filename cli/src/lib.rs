// Copyright 2018 Parity Technologies (UK) Ltd.
// Copyright 2018-2019 Chainpool.
// This file is part of Substrate.

// Substrate is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Substrate is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Substrate.  If not, see <http://www.gnu.org/licenses/>.

//! Substrate CLI library.

#![feature(custom_attribute)]

mod chain_spec;
mod genesis_config;
mod native_rpc;
mod params;
mod service;

use std::ops::Deref;

use log::{info, warn};
use tokio::runtime::{Builder as RuntimeBuilder, Runtime};

pub use cli::{error, IntoExit, NoCustom, VersionInfo};
use substrate_service::{Roles as ServiceRoles, ServiceFactory};

use self::params::ChainXParams;
use self::service::set_validator_name;

/// The chain specification option.
#[derive(Clone, Debug)]
pub enum ChainSpec {
    Development,
    Testnet,
    Mainnet,
}

/// Get a chain config from a spec setting.
impl ChainSpec {
    pub(crate) fn load(self) -> Result<chain_spec::ChainSpec, String> {
        Ok(match self {
            ChainSpec::Development => chain_spec::development_config(),
            ChainSpec::Testnet => chain_spec::testnet_config(),
            ChainSpec::Mainnet => chain_spec::mainnet_config(),
        })
    }

    pub(crate) fn from(s: &str) -> Option<Self> {
        match s {
            "mainnet" | "" => Some(ChainSpec::Mainnet),
            "testnet" => Some(ChainSpec::Testnet),
            "dev" => Some(ChainSpec::Development),
            _ => None,
        }
    }
}

fn load_spec(id: &str) -> Result<Option<chain_spec::ChainSpec>, String> {
    Ok(match ChainSpec::from(id) {
        Some(spec) => Some(spec.load()?),
        None => None,
    })
}

pub fn run<I, T, E>(args: I, exit: E, version: cli::VersionInfo) -> error::Result<()>
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
    E: IntoExit,
{
    cli::parse_and_execute::<service::Factory, NoCustom, ChainXParams, _, _, _, _, _, _>(
        load_spec,
        &version,
        "ChainX",
        args,
        exit,
        cli::init_logger,
        |exit, _cli_args, custom_args, config| {
            info!("{}", version.name);
            info!("  version {}", config.full_version());
            info!("  by ChainX, 2018-2019");
            info!("Chain specification: {}", config.chain_spec.name());
            if let Some(id) = config.chain_spec.protocol_id() {
                info!("Chain protocol_id: {:}", id);
            } else {
                warn!("Not set protocol_id! may receive other blockchain network msg");
            }
            info!("Chain properties: {:?}", config.chain_spec.properties());
            info!("Node name: {}", config.name);
            info!("Roles: {:?}", config.roles);
            let runtime = RuntimeBuilder::new()
                .name_prefix("main-tokio-")
                .build()
                .map_err(|e| format!("{:?}", e))?;
            let executor = runtime.executor();

            if config.roles == ServiceRoles::AUTHORITY {
                let name = custom_args
                    .validator_name
                    .expect("if in AUTHORITY mode, must point the validator name!");
                info!("Validator name: {:}", name);
                set_validator_name(name);
            }

            match config.roles {
                ServiceRoles::LIGHT => run_until_exit(
                    runtime,
                    service::Factory::new_light(config, executor)
                        .map_err(|e| format!("{:?}", e))?,
                    exit,
                ),
                _ => run_until_exit(
                    runtime,
                    service::Factory::new_full(config, executor).map_err(|e| format!("{:?}", e))?,
                    exit,
                ),
            }
            .map_err(|e| format!("{:?}", e))
        },
    )
    .map_err(Into::into)
    .map(|_| ())
}

fn run_until_exit<T, C, E>(mut runtime: Runtime, service: T, e: E) -> error::Result<()>
where
    T: Deref<Target = substrate_service::Service<C>> + native_rpc::Rpc,
    C: substrate_service::Components,
    E: IntoExit,
{
    let (exit_send, exit) = exit_future::signal();

    let executor = runtime.executor();
    let (_http, _ws) = service.start_rpc(executor.clone());
    cli::informant::start(&service, exit.clone(), executor.clone());

    let _ = runtime.block_on(e.into_exit());
    exit_send.fire();

    // we eagerly drop the service so that the internal exit future is fired,
    // but we need to keep holding a reference to the global telemetry guard
    let _telemetry = service.telemetry();
    drop(service);
    Ok(())
}
