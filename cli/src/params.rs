use std::path::PathBuf;
use structopt::{clap::App, StructOpt};

#[derive(Clone, StructOpt, Debug)]
pub struct ChainXParams {
    #[structopt(long = "validator-name", value_name = "NAME")]
    /// Registered validator name, when set `--validator` or `"validator": true`, must provide matching validator's unique name
    pub validator_name: Option<String>,

    // This option is actually unused and only for the auto generated help, which could be refined later.
    #[structopt(long = "config", value_name = "CONFIG_JSON_PATH", parse(from_os_str))]
    /// Pass [FLAGS] or [OPTIONS] via a JSON file, you can override them from the command line.
    pub config: Option<PathBuf>,

    #[structopt(long = "ws-max-connections", default_value = "100")]
    /// The maximum number of connections that this WebSocket will support, default is 100
    pub ws_max_connections: usize,

    #[structopt(long = "default-log")]
    /// Use `env_logger`, not `log4rs`. notice `env_logger` can't print into file directly
    pub default_log: bool,

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

impl cli::AugmentClap for ChainXParams {
    fn augment_clap<'a, 'b>(app: App<'a, 'b>) -> App<'a, 'b> {
        ChainXParams::augment_clap(app)
    }
}
