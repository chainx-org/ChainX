use sc_cli::{CliConfiguration, SubstrateCli};

use crate::chain_spec;
use crate::cli::Cli;
use crate::service;

impl SubstrateCli for Cli {
    fn impl_name() -> &'static str {
        "ChainX"
    }

    fn impl_version() -> &'static str {
        env!("SUBSTRATE_CLI_IMPL_VERSION")
    }

    fn description() -> &'static str {
        env!("CARGO_PKG_DESCRIPTION")
    }

    fn author() -> &'static str {
        env!("CARGO_PKG_AUTHORS")
    }

    fn support_url() -> &'static str {
        "https://github.com/chainx-org/ChainX"
    }

    fn copyright_start_year() -> i32 {
        2020
    }

    fn executable_name() -> &'static str {
        env!("CARGO_PKG_NAME")
    }

    fn load_spec(&self, id: &str) -> Result<Box<dyn sc_service::ChainSpec>, String> {
        // this id is from `--chain=<id>`
        load_spec(id)
    }
}

fn load_spec(id: &str) -> Result<Box<dyn sc_service::ChainSpec>, String> {
    Ok(match id {
        "dev" => Box::new(chain_spec::development_config()),
        "" | "local" => Box::new(chain_spec::local_testnet_config()),
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
    let cli = Cli::from_args();

    if cli.log4rs {
        crate::logger::init(&cli.run.log_filters()?, &cli)?;
    }

    match &cli.subcommand {
        Some(subcommand) => {
            let runner = cli.create_runner(subcommand)?;
            let chain_spec = &runner.config().chain_spec;
            set_default_ss58_version(chain_spec);

            runner.run_subcommand(subcommand, |config| Ok(new_full_start!(config).0))
        }
        None => {
            let runner = cli.create_runner(&cli.run)?;
            let chain_spec = &runner.config().chain_spec;
            set_default_ss58_version(chain_spec);

            runner.run_node(
                service::new_light,
                service::new_full,
                chainx_runtime::VERSION,
            )
        }
    }
}

fn set_default_ss58_version(spec: &Box<dyn sc_service::ChainSpec>) {
    use chainx_runtime::NetworkType;
    use sp_core::crypto::Ss58AddressFormat;
    // this `id()` is from `ChainSpec::from_genesis()` second parameter
    // todo may use a better way
    let type_: NetworkType = if spec.id().contains("mainnet") {
        NetworkType::Mainnet
    } else {
        NetworkType::Testnet
    };
    let ss58_version = Ss58AddressFormat::Custom(type_.addr_version());
    sp_core::crypto::set_default_ss58_version(ss58_version);
}
