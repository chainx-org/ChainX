use sc_cli::{RunCmd, Subcommand};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
pub struct Cli {
    #[structopt(subcommand)]
    pub subcommand: Option<Subcommand>,

    #[structopt(flatten)]
    pub run: RunCmd,

    #[structopt(long = "log4rs")]
    /// Use `env_logger`, not `log4rs`. notice `env_logger` can't print into file directly
    pub log4rs: bool,

    #[structopt(long = "log-dir", default_value = "./log")]
    /// When use `log4rs`, assign the path of log dir, notice this would create the dir directly. Default dir is `./log`
    pub log_dir: String,
    #[structopt(long = "log-name", default_value = "chainx.log")]
    /// When use `log4rs`, assign the log file name. Default dir is `chainx.log`, thus when use default config, the log would create in ./log/chainx.log
    pub log_name: String,
    #[structopt(long = "log-size", default_value = "300")]
    /// When use `log4rs`, the default log size in log rotation. The unit is MB, default is 300 MB
    pub log_size: u64,
    #[structopt(long = "log-roll-count", default_value = "10")]
    /// When use `log4rs`, the max log rotation. Default is 10. If the log is more then the number, would delete old file.
    /// The log would like `chainx.log.0`, `chainx.log.1` ... `chainx.log.10`
    pub log_roll_count: u32,
    #[structopt(long = "log-console")]
    /// When use `log4rs`, print log into console. Default is false
    pub log_console: bool,
    #[structopt(long = "log-compression")]
    /// When use `log4rs`, compress the old log to save space. the log would like `chainx.log.gz.0`
    pub log_compression: bool,
}
