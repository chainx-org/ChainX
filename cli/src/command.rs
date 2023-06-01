// Copyright 2019-2023 ChainX Project Authors. Licensed under GPL-3.0.
#![allow(clippy::borrowed_box)]

use std::net::SocketAddr;

use sc_cli::{
    ChainSpec, CliConfiguration, DefaultConfigurationValues, RuntimeVersion, SubstrateCli,
};
use sc_service::{
    config::{PrometheusConfig, TelemetryEndpoints},
    BasePath, TransactionPoolOptions,
};
use sp_core::crypto::{set_default_ss58_version, Ss58AddressFormat};

use chainx_service::{self as service, new_partial, IdentifyVariant};

use crate::chain_spec;
use crate::cli::{Cli, Subcommand};

impl DefaultConfigurationValues for Cli {
    fn p2p_listen_port() -> u16 {
        20222
    }

    fn rpc_ws_listen_port() -> u16 {
        8087
    }

    fn rpc_http_listen_port() -> u16 {
        8086
    }

    fn prometheus_listen_port() -> u16 {
        9615
    }
}

impl CliConfiguration<Self> for Cli {
    fn shared_params(&self) -> &sc_cli::SharedParams {
        self.run.base.shared_params()
    }

    fn import_params(&self) -> Option<&sc_cli::ImportParams> {
        self.run.base.import_params()
    }

    fn keystore_params(&self) -> Option<&sc_cli::KeystoreParams> {
        self.run.base.keystore_params()
    }

    fn network_params(&self) -> Option<&sc_cli::NetworkParams> {
        self.run.base.network_params()
    }

    fn offchain_worker_params(&self) -> Option<&sc_cli::OffchainWorkerParams> {
        self.run.base.offchain_worker_params()
    }

    fn base_path(&self) -> sc_cli::Result<Option<BasePath>> {
        self.run.base.base_path()
    }

    fn role(&self, is_dev: bool) -> sc_cli::Result<sc_cli::Role> {
        self.run.base.role(is_dev)
    }

    fn transaction_pool(&self) -> sc_cli::Result<TransactionPoolOptions> {
        self.run.base.transaction_pool()
    }

    fn node_name(&self) -> sc_cli::Result<String> {
        self.run.base.node_name()
    }

    fn rpc_http(&self, default_listen_port: u16) -> sc_cli::Result<Option<SocketAddr>> {
        self.run.base.rpc_http(default_listen_port)
    }

    fn rpc_ipc(&self) -> sc_cli::Result<Option<String>> {
        self.run.base.rpc_ipc()
    }

    fn rpc_ws(&self, default_listen_port: u16) -> sc_cli::Result<Option<SocketAddr>> {
        self.run.base.rpc_ws(default_listen_port)
    }

    fn rpc_methods(&self) -> sc_cli::Result<sc_service::config::RpcMethods> {
        self.run.base.rpc_methods()
    }

    fn rpc_ws_max_connections(&self) -> sc_cli::Result<Option<usize>> {
        self.run.base.rpc_ws_max_connections()
    }

    fn rpc_cors(&self, is_dev: bool) -> sc_cli::Result<Option<Vec<String>>> {
        self.run.base.rpc_cors(is_dev)
    }

    fn prometheus_config(
        &self,
        default_listen_port: u16,
        chain_spec: &Box<dyn ChainSpec>,
    ) -> sc_cli::Result<Option<PrometheusConfig>> {
        self.run
            .base
            .prometheus_config(default_listen_port, chain_spec)
    }

    fn telemetry_endpoints(
        &self,
        chain_spec: &Box<dyn sc_cli::ChainSpec>,
    ) -> sc_cli::Result<Option<TelemetryEndpoints>> {
        self.run.base.telemetry_endpoints(chain_spec)
    }

    fn force_authoring(&self) -> sc_cli::Result<bool> {
        self.run.base.force_authoring()
    }

    fn disable_grandpa(&self) -> sc_cli::Result<bool> {
        self.run.base.disable_grandpa()
    }

    fn dev_key_seed(&self, is_dev: bool) -> sc_cli::Result<Option<String>> {
        self.run.base.dev_key_seed(is_dev)
    }

    fn max_runtime_instances(&self) -> sc_cli::Result<Option<usize>> {
        self.run.base.max_runtime_instances()
    }
}

impl SubstrateCli for Cli {
    fn impl_name() -> String {
        "ChainX".into()
    }

    fn impl_version() -> String {
        env!("SUBSTRATE_CLI_IMPL_VERSION").into()
    }

    fn executable_name() -> String {
        "chainx".into()
    }

    fn description() -> String {
        env!("CARGO_PKG_DESCRIPTION").into()
    }

    fn author() -> String {
        env!("CARGO_PKG_AUTHORS").into()
    }

    fn support_url() -> String {
        "https://github.com/chainx-org/ChainX/issues/new".into()
    }

    fn copyright_start_year() -> i32 {
        2019
    }

    fn load_spec(&self, id: &str) -> Result<Box<dyn sc_service::ChainSpec>, String> {
        // this id is from `--chain=<id>`
        load_spec(id)
    }

    /// Create a runner for the command provided in argument. This will create a Configuration and
    /// a tokio runtime
    fn create_runner<T: CliConfiguration>(
        &self,
        command: &T,
    ) -> sc_cli::Result<sc_cli::Runner<Self>> {
        // Workaround for https://github.com/paritytech/substrate/issues/6856
        // Remove this once the cli config file is supported in Substrate.
        let raw_cli_args = std::env::args().collect::<Vec<_>>();
        let cli = Cli::from_iter(crate::config::preprocess_cli_args(raw_cli_args));

        let tokio_runtime = sc_cli::build_runtime()?;

        let config = if cli.subcommand.is_some() {
            command.create_configuration(self, tokio_runtime.handle().clone())?
        } else {
            CliConfiguration::create_configuration(&cli, self, tokio_runtime.handle().clone())?
        };

        // Try to enable the log rotation function if from config file.
        if cli.run.config_file.is_some() && !cli.run.logger.no_log_rotation {
            cli.try_init_logger()?;
        } else {
            command.init(
                &Self::support_url(),
                &Self::impl_version(),
                |_, _| {},
                &config,
            )?;
        }

        sc_cli::Runner::new(config, tokio_runtime)
    }

    fn native_runtime_version(chain_spec: &Box<dyn ChainSpec>) -> &'static RuntimeVersion {
        if chain_spec.is_malan() {
            &malan_runtime::VERSION
        } else if chain_spec.is_dev() {
            &dev_runtime::VERSION
        } else {
            &chainx_runtime::VERSION
        }
    }
}

fn load_spec(id: &str) -> Result<Box<dyn sc_service::ChainSpec>, String> {
    Ok(match id {
        "" | "mainnet" => Box::new(chain_spec::mainnet_config()?),
        "new-mainnet" => Box::new(chain_spec::new_mainnet_config()?),
        "dev" => Box::new(chain_spec::development_config()?),
        "malan" | "testnet" => Box::new(chain_spec::malan_config()?),
        "new-malan" => Box::new(chain_spec::new_malan_config()?),
        "local" => Box::new(chain_spec::local_testnet_config()?),
        "benchmarks" => {
            #[cfg(feature = "runtime-benchmarks")]
            {
                Box::new(chain_spec::benchmarks_config()?)
            }
            #[cfg(not(feature = "runtime-benchmarks"))]
            {
                return Err(
                    "benchmarks chain-config should compile with feature `runtime-benchmarks`"
                        .into(),
                );
            }
        }
        path if path.starts_with("dev") => Box::new(chain_spec::DevChainSpec::from_json_file(
            std::path::PathBuf::from(path),
        )?),
        path if path.starts_with("malan") => Box::new(chain_spec::MalanChainSpec::from_json_file(
            std::path::PathBuf::from(path),
        )?),
        path => {
            let p = std::path::PathBuf::from(path);
            if !p.exists() {
                return Err("invalid path or just use --chain={dev, local, testnet, mainnet, malan, benchmarks}".into());
            }
            Box::new(chain_spec::ChainXChainSpec::from_json_file(p)?)
        }
    })
}

macro_rules! construct_async_run {
     (|$components:ident, $cli:ident, $cmd:ident, $config:ident| $( $code:tt )* ) => {{
         let runner = $cli.create_runner($cmd)?;

         if runner.config().chain_spec.is_malan() {
            runner.async_run(|mut $config| {
                let $components = new_partial::<
                    malan_runtime::RuntimeApi,
                    chainx_executor::MalanExecutor
                >(
                    &mut $config,
                )?;
                let task_manager = $components.task_manager;
                { $( $code )* }.map(|v| (v, task_manager))
            })
        } else if runner.config().chain_spec.is_dev() {
            runner.async_run(|mut $config| {
                let $components = new_partial::<
                    dev_runtime::RuntimeApi,
                    chainx_executor::DevExecutor
                >(
                    &mut $config,
                )?;
                let task_manager = $components.task_manager;
                { $( $code )* }.map(|v| (v, task_manager))
            })
        } else {
            runner.async_run(|mut $config| {
                let $components = new_partial::<
                    chainx_runtime::RuntimeApi,
                    chainx_executor::ChainXExecutor,
                >(
                    &mut $config,
                )?;
                let task_manager = $components.task_manager;
                { $( $code )* }.map(|v| (v, task_manager))
            })
        }
    }}
 }

/// Parse and run command line arguments
pub fn run() -> sc_cli::Result<()> {
    // Workaround for https://github.com/paritytech/substrate/issues/6856
    // Remove this once the cli config file is supported in Substrate.
    let raw_cli_args = std::env::args().collect::<Vec<_>>();
    let cli = <Cli as SubstrateCli>::from_iter(crate::config::preprocess_cli_args(raw_cli_args));

    // Set ChainX account
    set_default_ss58_version(Ss58AddressFormat::from(44u16));

    match &cli.subcommand {
        None => {
            let runner = cli.create_runner(&cli.run.base)?;

            runner.run_node_until_exit(|config| async move {
                service::build_full(config).map_err(sc_cli::Error::Service)
            })
        }
        Some(Subcommand::Benchmark(cmd)) => {
            if cfg!(feature = "runtime-benchmarks") {
                let runner = cli.create_runner(cmd)?;

                runner.sync_run(|config| {
                    cmd.run::<chainx_runtime::Block, chainx_executor::ChainXExecutor>(config)
                })
            } else {
                println!(
                    "Benchmarking wasn't enabled when building the node. \
                     You can enable it with `--features runtime-benchmarks`."
                );
                Ok(())
            }
        }
        Some(Subcommand::Key(cmd)) => cmd.run(&cli),
        Some(Subcommand::Sign(cmd)) => cmd.run(),
        Some(Subcommand::Verify(cmd)) => cmd.run(),
        Some(Subcommand::Vanity(cmd)) => cmd.run(),
        Some(Subcommand::BuildSpec(cmd)) => {
            let runner = cli.create_runner(cmd)?;

            runner.sync_run(|config| cmd.run(config.chain_spec, config.network))
        }
        Some(Subcommand::CheckBlock(cmd)) => {
            construct_async_run!(|components, cli, cmd, config| {
                Ok(cmd.run(components.client, components.import_queue))
            })
        }
        Some(Subcommand::ExportBlocks(cmd)) => {
            construct_async_run!(|components, cli, cmd, config| {
                Ok(cmd.run(components.client, config.database))
            })
        }
        Some(Subcommand::ExportState(cmd)) => {
            construct_async_run!(|components, cli, cmd, config| {
                Ok(cmd.run(components.client, config.chain_spec))
            })
        }
        Some(Subcommand::ImportBlocks(cmd)) => {
            construct_async_run!(|components, cli, cmd, config| {
                Ok(cmd.run(components.client, components.import_queue))
            })
        }
        Some(Subcommand::PurgeChain(cmd)) => {
            let runner = cli.create_runner(cmd)?;

            runner.sync_run(|config| {
                // Remove Frontier offchain db
                let frontier_database_config = sc_service::DatabaseSource::RocksDb {
                    path: service::frontier_database_dir(&config),
                    cache_size: 0,
                };
                cmd.run(frontier_database_config)?;
                cmd.run(config.database)
            })
        }
        Some(Subcommand::Revert(cmd)) => {
            construct_async_run!(|components, cli, cmd, config| {
                Ok(cmd.run(components.client, components.backend))
            })
        }
        #[cfg(feature = "try-runtime")]
        Some(Subcommand::TryRuntime(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            let chain_spec = &runner.config().chain_spec;

            if chain_spec.is_malan() {
                return runner.async_run(|config| {
                    let registry = config.prometheus_config.as_ref().map(|cfg| &cfg.registry);
                    let task_manager =
                        sc_service::TaskManager::new(config.tokio_handle.clone(), registry)
                            .map_err(|e| {
                                sc_cli::Error::Service(sc_service::Error::Prometheus(e))
                            })?;
                    Ok((
                        cmd.run::<malan_runtime::Block, chainx_executor::MalanExecutor>(config),
                        task_manager,
                    ))
                });
            } else if chain_spec.is_dev() {
                return runner.async_run(|config| {
                    let registry = config.prometheus_config.as_ref().map(|cfg| &cfg.registry);
                    let task_manager =
                        sc_service::TaskManager::new(config.tokio_handle.clone(), registry)
                            .map_err(|e| {
                                sc_cli::Error::Service(sc_service::Error::Prometheus(e))
                            })?;
                    Ok((
                        cmd.run::<dev_runtime::Block, chainx_executor::DevExecutor>(config),
                        task_manager,
                    ))
                });
            } else {
                return runner.async_run(|config| {
                    let registry = config.prometheus_config.as_ref().map(|cfg| &cfg.registry);
                    let task_manager =
                        sc_service::TaskManager::new(config.tokio_handle.clone(), registry)
                            .map_err(|e| {
                                sc_cli::Error::Service(sc_service::Error::Prometheus(e))
                            })?;
                    Ok((
                        cmd.run::<chainx_runtime::Block, chainx_executor::ChainXExecutor>(config),
                        task_manager,
                    ))
                });
            }
        }
        #[cfg(not(feature = "try-runtime"))]
        Some(Subcommand::TryRuntime) => Err("TryRuntime wasn't enabled when building the node. \
             You can enable it with `--features try-runtime`."
            .into()),
    }
}
