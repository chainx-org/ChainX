use sc_cli::{CliConfiguration, KeySubcommand, SignCmd, VanityCmd, VerifyCmd};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
pub struct Cli {
    #[structopt(subcommand)]
    pub subcommand: Option<Subcommand>,

    #[structopt(flatten)]
    pub run: RunCmd,
}

/// Possible subcommands of the main binary.
#[derive(Debug, StructOpt)]
pub enum Subcommand {
    /// A set of base subcommands handled by `sc_cli`.
    #[structopt(flatten)]
    Base(sc_cli::Subcommand),

    /// Key management cli utilities
    Key(KeySubcommand),

    /// The custom benchmark subcommmand benchmarking runtime pallets.
    #[structopt(name = "benchmark", about = "Benchmark runtime pallets.")]
    Benchmark(frame_benchmarking_cli::BenchmarkCmd),

    /// Verify a signature for a message, provided on STDIN, with a given (public or secret) key.
    Verify(VerifyCmd),

    /// Generate a seed that provides a vanity address.
    Vanity(VanityCmd),

    /// Sign a message, with a given (secret) key.
    Sign(SignCmd),
}

#[allow(missing_docs)]
#[derive(Debug, StructOpt)]
pub struct RunCmd {
    #[allow(missing_docs)]
    #[structopt(flatten)]
    pub base: sc_cli::RunCmd,

    /// Pass `foo` option starting with `--` via a JSON config file
    ///
    /// The key of JSON entry must be the form of `--KEY`, `-KEY` is invalid, e.g, you
    /// can use `base-path` in the config file but `d` is not allowed. For the options like
    /// `-d` you have to pass them from the command line. Any options in the config file
    /// can be overrided by the same one passed from the command line.
    #[structopt(long = "config", value_name = "PATH", parse(from_os_str))]
    pub config_file: Option<std::path::PathBuf>,

    #[structopt(flatten)]
    pub logger: crate::logger::LoggerParams,
}

impl Cli {
    pub fn try_init_logger(&self) -> sc_cli::Result<()> {
        if self.run.logger.log4rs {
            crate::logger::init(&self.run.base.log_filters()?, &self.run.logger)?;
        }
        Ok(())
    }
}
