use sc_cli::CliConfiguration;
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

    /// The custom benchmark subcommmand benchmarking runtime pallets.
    #[structopt(name = "benchmark", about = "Benchmark runtime pallets.")]
    Benchmark(frame_benchmarking_cli::BenchmarkCmd),
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
    pub logger: LoggerParams,
}

impl Cli {
    pub fn try_init_logger(&self) -> sc_cli::Result<()> {
        if self.run.logger.log4rs {
            crate::logger::init(&self.run.base.log_filters()?, &self.run.logger)?;
        }
        Ok(())
    }
}

#[derive(Debug, StructOpt)]
pub struct LoggerParams {
    /// Enable the log4rs feature, including the log rotation function.
    //  Use `log4rs` as `env_logger` can't print the message into file directly.
    #[structopt(long = "log4rs")]
    pub log4rs: bool,

    /// Specify the path of directory which will contain the log files.
    ///
    /// The directory will be created if it does not exist.
    #[structopt(long = "log-dir", default_value = "./log")]
    pub log_dir: String,

    /// Specify the name of log file.
    ///
    /// The latest log file would be created at ./log/chainx.log.
    #[structopt(long = "log-filename", default_value = "chainx.log")]
    pub log_filename: String,

    /// Rotate the log file when it exceeds this size (in MB).
    #[structopt(long = "log-size", default_value = "300")]
    pub log_size: u64,

    /// The maximum number of log rorations.
    ///
    /// By default the generated log files would be like `chainx.log.0`,
    /// `chainx.log.1`, ... `chainx.log.10`. Once the number of generated log files
    /// are larger than this variable, the oldest one will be removed.
    #[structopt(long = "log-roll-count", default_value = "10")]
    pub log_roll_count: u32,

    /// Print the log message to the console aside from the log file.
    #[structopt(long = "log-console")]
    pub log_console: bool,

    /// Compress the old log file to save some disk space.
    ///
    /// The compressed log file would be like `chainx.log.gz.0` by default.
    #[structopt(long = "log-compression")]
    pub log_compression: bool,
}
