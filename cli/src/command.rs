// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use std::net::SocketAddr;

use sc_cli::{
    ChainSpec, CliConfiguration, DefaultConfigurationValues, Role, RuntimeVersion, SubstrateCli,
};
use sc_service::{
    config::{PrometheusConfig, TelemetryEndpoints},
    BasePath, PartialComponents, TransactionPoolOptions,
};

use chainx_service::{self as service, new_partial};

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

impl<DCV> CliConfiguration<DCV> for Cli
where
    DCV: DefaultConfigurationValues,
{
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
    ) -> sc_cli::Result<Option<PrometheusConfig>> {
        self.run.base.prometheus_config(default_listen_port)
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

    fn native_runtime_version(_: &Box<dyn ChainSpec>) -> &'static RuntimeVersion {
        &chainx_runtime::VERSION
    }
}

fn load_spec(id: &str) -> Result<Box<dyn sc_service::ChainSpec>, String> {
    Ok(match id {
        "" | "mainnet" => Box::new(chain_spec::mainnet_config()?),
        "dev" => Box::new(chain_spec::development_config()?),
        "malan" | "testnet" => Box::new(chain_spec::malan_config()?),
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
        path => {
            let p = std::path::PathBuf::from(path);
            if !p.exists() {
                // TODO more better hint
                return Err("invalid path or just use --chain={dev, local, testnet, mainnet, malan, benchmarks}".into());
            }
            Box::new(chain_spec::ChainXChainSpec::from_json_file(p)?)
        }
    })
}

/// Parse and run command line arguments
pub fn run() -> sc_cli::Result<()> {
    // Workaround for https://github.com/paritytech/substrate/issues/6856
    // Remove this once the cli config file is supported in Substrate.
    let raw_cli_args = std::env::args().collect::<Vec<_>>();
    let cli = <Cli as SubstrateCli>::from_iter(crate::config::preprocess_cli_args(raw_cli_args));

    // Try to enable the log rotation function if not a dev chain.
    if !cli.run.base.shared_params.dev {
        cli.try_init_logger()?;
    }

    match &cli.subcommand {
        None => {
            let runner = cli.create_runner(&cli)?;
            let chain_spec = &runner.config().chain_spec;
            set_default_ss58_version(chain_spec);

            runner.run_node_until_exit(|config| async move {
                let config = SubstrateCli::create_configuration::<Cli, Cli>(
                    &cli,
                    &cli,
                    config.task_executor.clone(),
                )
                .map_err(|err| format!("chain argument error: {:?}", err))?;

                match config.role {
                    Role::Light => service::build_light(config),
                    _ => service::build_full(config),
                }
            })
        }
        Some(Subcommand::Benchmark(cmd)) => {
            if cfg!(feature = "runtime-benchmarks") {
                let runner = cli.create_runner(cmd)?;
                let chain_spec = &runner.config().chain_spec;

                set_default_ss58_version(chain_spec);

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
        Some(Subcommand::Key(cmd)) => cmd.run(),
        Some(Subcommand::Sign(cmd)) => cmd.run(),
        Some(Subcommand::Verify(cmd)) => cmd.run(),
        Some(Subcommand::Vanity(cmd)) => cmd.run(),
        Some(Subcommand::BuildSpec(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            set_default_ss58_version(&runner.config().chain_spec);

            runner.sync_run(|config| cmd.run(config.chain_spec, config.network))
        }
        Some(Subcommand::CheckBlock(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            set_default_ss58_version(&runner.config().chain_spec);

            runner.async_run(|config| {
                let PartialComponents {
                    client,
                    task_manager,
                    import_queue,
                    ..
                } = new_partial::<chainx_runtime::RuntimeApi, chainx_executor::ChainXExecutor>(
                    &config,
                )?;
                Ok((cmd.run(client, import_queue), task_manager))
            })
        }
        Some(Subcommand::ExportBlocks(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            set_default_ss58_version(&runner.config().chain_spec);

            runner.async_run(|config| {
                let PartialComponents {
                    client,
                    task_manager,
                    ..
                } = new_partial::<chainx_runtime::RuntimeApi, chainx_executor::ChainXExecutor>(
                    &config,
                )?;
                Ok((cmd.run(client, config.database), task_manager))
            })
        }
        Some(Subcommand::ExportState(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            set_default_ss58_version(&runner.config().chain_spec);

            runner.async_run(|config| {
                let PartialComponents {
                    client,
                    task_manager,
                    ..
                } = new_partial::<chainx_runtime::RuntimeApi, chainx_executor::ChainXExecutor>(
                    &config,
                )?;
                Ok((cmd.run(client, config.chain_spec), task_manager))
            })
        }
        Some(Subcommand::ImportBlocks(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            set_default_ss58_version(&runner.config().chain_spec);

            runner.async_run(|config| {
                let PartialComponents {
                    client,
                    task_manager,
                    import_queue,
                    ..
                } = new_partial::<chainx_runtime::RuntimeApi, chainx_executor::ChainXExecutor>(
                    &config,
                )?;
                Ok((cmd.run(client, import_queue), task_manager))
            })
        }
        Some(Subcommand::PurgeChain(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            set_default_ss58_version(&runner.config().chain_spec);

            runner.sync_run(|config| cmd.run(config.database))
        }
        Some(Subcommand::Revert(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            set_default_ss58_version(&runner.config().chain_spec);

            runner.async_run(|config| {
                let PartialComponents {
                    client,
                    task_manager,
                    backend,
                    ..
                } = new_partial::<chainx_runtime::RuntimeApi, chainx_executor::ChainXExecutor>(
                    &config,
                )?;
                Ok((cmd.run(client, backend), task_manager))
            })
        }
    }
}

fn set_default_ss58_version(spec: &Box<dyn sc_service::ChainSpec>) {
    use sp_core::crypto::Ss58AddressFormat;
    // this `id()` is from `ChainSpec::from_genesis()` second parameter
    // todo may use a better way
    let version: Ss58AddressFormat = if spec.id() == "chainx" {
        Ss58AddressFormat::ChainXAccount
    } else {
        Ss58AddressFormat::SubstrateAccount
    };

    sp_core::crypto::set_default_ss58_version(version);
}
