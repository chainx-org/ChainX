use sc_cli::{ChainSpec, CliConfiguration, Role, RuntimeVersion, SubstrateCli};
use sc_service::ServiceParams;

use crate::chain_spec;
use crate::cli::Cli;
use crate::service::{self, new_full_params};

impl SubstrateCli for Cli {
    fn impl_name() -> String {
        "ChainX".into()
    }

    fn impl_version() -> String {
        env!("SUBSTRATE_CLI_IMPL_VERSION").into()
    }

    fn executable_name() -> String {
        env!("CARGO_PKG_NAME").into()
    }

    fn description() -> String {
        env!("CARGO_PKG_DESCRIPTION").into()
    }

    fn author() -> String {
        env!("CARGO_PKG_AUTHORS").into()
    }

    fn support_url() -> String {
        "https://github.com/chainx-org/ChainX".into()
    }

    fn copyright_start_year() -> i32 {
        2020
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
        "dev" => Box::new(chain_spec::development_config()?),
        "" | "local" => Box::new(chain_spec::local_testnet_config()?),
        path => {
            let p = std::path::PathBuf::from(path);
            if !p.exists() {
                // TODO more better hint
                return Err(format!(
                    "not a valid path or just allow [\"dev\", \"local\"]"
                ));
            }
            Box::new(chain_spec::ChainSpec::from_json_file(p)?)
        }
    })
}

/// Parse and run command line arguments
pub fn run() -> sc_cli::Result<()> {
    // Workaround for https://github.com/paritytech/substrate/issues/6856
    // Remove this once the cli config file is supported in Substrate.
    let raw_cli_args = std::env::args().collect::<Vec<_>>();
    let cli = Cli::from_iter(crate::config::preprocess_cli_args(raw_cli_args));

    if cli.log4rs {
        crate::logger::init(&cli.run.log_filters()?, &cli)?;
    }

    match &cli.subcommand {
        Some(subcommand) => {
            let runner = cli.create_runner(subcommand)?;
            let chain_spec = &runner.config().chain_spec;
            set_default_ss58_version(chain_spec);

            runner.run_subcommand(subcommand, |config| {
                let (
                    ServiceParams {
                        client,
                        backend,
                        task_manager,
                        import_queue,
                        ..
                    },
                    ..,
                ) = new_full_params(config)?;
                Ok((client, backend, import_queue, task_manager))
            })
        }
        None => {
            let runner = cli.create_runner(&cli.run)?;
            let chain_spec = &runner.config().chain_spec;
            set_default_ss58_version(chain_spec);

            runner.run_node_until_exit(|config| match config.role {
                Role::Light => service::new_light(config),
                _ => service::new_full(config),
            })
        }
    }
}

fn set_default_ss58_version(spec: &Box<dyn sc_service::ChainSpec>) {
    use sp_core::crypto::Ss58AddressFormat;
    // this `id()` is from `ChainSpec::from_genesis()` second parameter
    // todo may use a better way
    let version: Ss58AddressFormat = if spec.id().contains("mainnet") {
        Ss58AddressFormat::ChainXAccount
    } else {
        Ss58AddressFormat::SubstrateAccount
    };

    sp_core::crypto::set_default_ss58_version(version);
}
