// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use sc_cli::{CliConfiguration, KeySubcommand, SignCmd, VanityCmd, VerifyCmd};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
pub struct Cli {
    #[structopt(subcommand)]
    pub subcommand: Option<Subcommand>,

    #[structopt(flatten)]
    pub run: RunCmd,
}

#[derive(Debug, StructOpt)]
pub enum Subcommand {
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

    /// Build a chain specification.
    BuildSpec(sc_cli::BuildSpecCmd),

    /// Validate blocks.
    CheckBlock(sc_cli::CheckBlockCmd),

    /// Export blocks.
    ExportBlocks(sc_cli::ExportBlocksCmd),

    /// Export the state of a given block into a chain spec.
    ExportState(sc_cli::ExportStateCmd),

    /// Import blocks.
    ImportBlocks(sc_cli::ImportBlocksCmd),

    /// Remove the whole chain.
    PurgeChain(sc_cli::PurgeChainCmd),

    /// Revert the chain to a previous state.
    Revert(sc_cli::RevertCmd),
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
        if !self.run.logger.no_log_rotation {
            crate::logger::init(&self.run.base.log_filters()?, &self.run.logger)?;
        }
        Ok(())
    }
}
