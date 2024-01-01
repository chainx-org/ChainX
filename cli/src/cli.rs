// Copyright 2019-2023 ChainX Project Authors. Licensed under GPL-3.0.

use sc_cli::{
    CliConfiguration, KeySubcommand, PruningParams, Result, SharedParams, SignCmd, SubstrateCli,
    VanityCmd, VerifyCmd,
};
use sc_client_api::AuxStore;

use chainx_service::new_partial;

#[derive(Debug, clap::Parser)]
pub struct Cli {
    #[clap(subcommand)]
    pub subcommand: Option<Subcommand>,

    #[clap(flatten)]
    pub run: RunCmd,
}

#[derive(Debug, clap::Subcommand)]
pub enum Subcommand {
    /// Key management cli utilities
    #[clap(subcommand)]
    Key(KeySubcommand),

    /// The custom benchmark subcommmand benchmarking runtime pallets.
    #[clap(name = "benchmark", about = "Benchmark runtime pallets.")]
    Benchmark(frame_benchmarking_cli::BenchmarkCmd),

    /// Try some command against runtime state.
    #[cfg(feature = "try-runtime")]
    TryRuntime(try_runtime_cli::TryRuntimeCmd),

    /// Try some command against runtime state. Note: `try-runtime` feature must be enabled.
    #[cfg(not(feature = "try-runtime"))]
    TryRuntime,

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

    #[clap(subcommand)]
    FixBabeEpoch(FixEpochSubCommand),
}

#[derive(Debug, clap::Subcommand)]
pub enum FixEpochSubCommand {
    Dump(FixEpochDumpCommand),
    Override(FixEpochOverrideommand),
}

impl FixEpochSubCommand {
    /// Run the revert command
    pub fn run<C: SubstrateCli>(&self, cli: &C) -> Result<()> {
        match self {
            FixEpochSubCommand::Dump(cmd) => cmd.run(cli),
            FixEpochSubCommand::Override(cmd) => cmd.run(cli),
        }
    }
}

#[derive(Debug, clap::Parser)]
pub struct FixEpochDumpCommand {
    #[allow(missing_docs)]
    #[clap(flatten)]
    pub shared_params: SharedParams,

    #[allow(missing_docs)]
    #[clap(flatten)]
    pub pruning_params: PruningParams,
}
const BABE_EPOCH_CHANGES_KEY: &[u8] = b"babe_epoch_changes";
use codec::{Decode, Encode};
use sc_consensus_babe::Epoch;
use sc_consensus_epochs::EpochChangesFor;

impl FixEpochDumpCommand {
    pub fn run<C: SubstrateCli>(&self, cli: &C) -> Result<()> {
        let runner = cli.create_runner(self)?;
        runner.sync_run(|mut config| {
            let components = new_partial::<
                chainx_runtime::RuntimeApi,
                chainx_executor::ChainXExecutor,
            >(&mut config)?;
            let client = components.client;
            let bytes = client
                .get_aux(BABE_EPOCH_CHANGES_KEY)
                .expect("Access DB should success")
                .expect("value must exist");
            let hex_str = hex::encode(&bytes);
            println!("{}", hex_str);
            let epoch: EpochChangesFor<chainx_runtime::Block, Epoch> =
                codec::Decode::decode(&mut &bytes[..]).expect("Decode must success");
            println!("epoch: {:?}", epoch);
            Ok(())
        })
    }
}

#[derive(Debug, clap::Parser)]
pub struct FixEpochOverrideommand {
    #[allow(missing_docs)]
    #[clap(flatten)]
    pub shared_params: SharedParams,

    #[allow(missing_docs)]
    #[clap(flatten)]
    pub pruning_params: PruningParams,

    #[clap(long)]
    pub bytes: String,
}

impl FixEpochOverrideommand {
    pub fn run<C: SubstrateCli>(&self, cli: &C) -> Result<()> {
        let runner = cli.create_runner(self)?;
        runner.sync_run(|mut config| {
            let components = new_partial::<
                chainx_runtime::RuntimeApi,
                chainx_executor::ChainXExecutor,
            >(&mut config)?;
            let client = components.client;

            let bytes = hex::decode(&self.bytes).expect("require hex string without 0x");
            let epoch: EpochChangesFor<chainx_runtime::Block, Epoch> =
                Decode::decode(&mut &bytes[..]).expect("Decode must success");
            println!("epoch: {:?}", epoch);

            client
                .insert_aux(&[(BABE_EPOCH_CHANGES_KEY, &epoch.encode()[..])], &[])
                .expect("Insert to db must success");
            Ok(())
        })
    }
}

impl CliConfiguration for FixEpochDumpCommand {
    fn shared_params(&self) -> &SharedParams {
        &self.shared_params
    }

    fn pruning_params(&self) -> Option<&PruningParams> {
        Some(&self.pruning_params)
    }
}

impl CliConfiguration for FixEpochOverrideommand {
    fn shared_params(&self) -> &SharedParams {
        &self.shared_params
    }

    fn pruning_params(&self) -> Option<&PruningParams> {
        Some(&self.pruning_params)
    }
}

#[allow(missing_docs)]
#[derive(Debug, clap::Parser)]
pub struct RunCmd {
    #[allow(missing_docs)]
    #[clap(flatten)]
    pub base: sc_cli::RunCmd,

    /// Pass `foo` option starting with `--` via a JSON config file
    ///
    /// The key of JSON entry must be the form of `--KEY`, `-KEY` is invalid, e.g, you
    /// can use `base-path` in the config file but `d` is not allowed. For the options like
    /// `-d` you have to pass them from the command line. Any options in the config file
    /// can be overrided by the same one passed from the command line.
    #[clap(long = "config", value_name = "PATH", parse(from_os_str))]
    pub config_file: Option<std::path::PathBuf>,

    #[clap(flatten)]
    pub logger: crate::logger::LoggerParams,
}

impl Cli {
    pub fn try_init_logger(&self) -> sc_cli::Result<()> {
        crate::logger::init(&self.run.base.log_filters()?, &self.run.logger)?;

        Ok(())
    }
}
