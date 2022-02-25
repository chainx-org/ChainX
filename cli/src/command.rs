// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use std::net::SocketAddr;

use sc_cli::{
    ChainSpec, CliConfiguration, DefaultConfigurationValues, ImportParams, KeystoreParams,
    NetworkParams, Role, RuntimeVersion, SharedParams, SubstrateCli,
};
use sc_service::config::PrometheusConfig;

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
    fn shared_params(&self) -> &SharedParams {
        self.run.base.shared_params()
    }

    fn import_params(&self) -> Option<&ImportParams> {
        self.run.base.import_params()
    }

    fn keystore_params(&self) -> Option<&KeystoreParams> {
        self.run.base.keystore_params()
    }

    fn network_params(&self) -> Option<&NetworkParams> {
        self.run.base.network_params()
    }

    fn role(&self, is_dev: bool) -> sc_cli::Result<sc_service::Role> {
        self.run.base.role(is_dev)
    }

    fn transaction_pool(&self) -> sc_cli::Result<sc_service::config::TransactionPoolOptions> {
        self.run.base.transaction_pool()
    }

    fn state_cache_child_ratio(&self) -> sc_cli::Result<Option<usize>> {
        self.run.base.state_cache_child_ratio()
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

    fn default_heap_pages(&self) -> sc_cli::Result<Option<u64>> {
        self.run.base.default_heap_pages()
    }

    fn force_authoring(&self) -> sc_cli::Result<bool> {
        self.run.base.force_authoring()
    }

    fn disable_grandpa(&self) -> sc_cli::Result<bool> {
        self.run.base.disable_grandpa()
    }

    fn max_runtime_instances(&self) -> sc_cli::Result<Option<usize>> {
        self.run.base.max_runtime_instances()
    }

    fn announce_block(&self) -> sc_cli::Result<bool> {
        self.run.base.announce_block()
    }

    fn init<C: SubstrateCli>(&self) -> sc_cli::Result<()> {
        unreachable!("ChainX is never initialized; qed");
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
    fn create_runner<T: CliConfiguration>(&self, command: &T) -> sc_cli::Result<sc_cli::Runner<Self>> {
        // Workaround for https://github.com/paritytech/substrate/issues/6856
        // Remove this once the cli config file is supported in Substrate.
        let raw_cli_args = std::env::args().collect::<Vec<_>>();
        let cli = Cli::from_iter(crate::config::preprocess_cli_args(raw_cli_args));

        // Try to enable the log rotation function if from config file.
        if cli.run.config_file.is_some() && !cli.run.logger.no_log_rotation {
            cli.try_init_logger()?;
        } else {
            command.init::<Self>()?;
        }

        sc_cli::Runner::new(self, command)
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

macro_rules! construct_async_run {
    (|$components:ident, $cli:ident, $cmd:ident, $config:ident| $( $code:tt )* ) => {{
        let runner = $cli.create_runner($cmd)?;
        set_default_ss58_version(&runner.config().chain_spec);

        if runner.config().chain_spec.is_malan() {
            runner.async_run(|$config| {
                let $components = new_partial::<
                    malan_runtime::RuntimeApi,
                    chainx_executor::MalanExecutor
                >(
                    &$config,
                )?;
                let task_manager = $components.task_manager;
                { $( $code )* }.map(|v| (v, task_manager))
            })
        } else if runner.config().chain_spec.is_dev() {
            runner.async_run(|$config| {
                let $components = new_partial::<
                    dev_runtime::RuntimeApi,
                    chainx_executor::DevExecutor
                >(
                    &$config,
                )?;
                let task_manager = $components.task_manager;
                { $( $code )* }.map(|v| (v, task_manager))
            })
        } else {
            runner.async_run(|$config| {
                let $components = new_partial::<
                    chainx_runtime::RuntimeApi,
                    chainx_executor::ChainXExecutor,
                >(
                    &$config,
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
    let cli = Cli::from_iter(crate::config::preprocess_cli_args(raw_cli_args));

    match &cli.subcommand {
        None => {
            let runner = cli.create_runner(&cli.run.base)?;
            let chain_spec = &runner.config().chain_spec;
            set_default_ss58_version(chain_spec);

            runner
                .run_node_until_exit(|config| async move {
                    match config.role {
                        Role::Light => service::build_light(config),
                        _ => service::build_full(config),
                    }
                })
                .map_err(sc_cli::Error::Service)
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
        Some(Subcommand::Key(cmd)) => cmd.run(&cli),
        Some(Subcommand::Sign(cmd)) => cmd.run(),
        Some(Subcommand::Verify(cmd)) => cmd.run(),
        Some(Subcommand::Vanity(cmd)) => cmd.run(),
        Some(Subcommand::BuildSpec(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            set_default_ss58_version(&runner.config().chain_spec);

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
            set_default_ss58_version(&runner.config().chain_spec);

            runner.sync_run(|config| cmd.run(config.database))
        }
        Some(Subcommand::Revert(cmd)) => {
            construct_async_run!(|components, cli, cmd, config| {
                Ok(cmd.run(components.client, components.backend))
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
