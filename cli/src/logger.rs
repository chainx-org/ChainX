// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use std::str::FromStr;

use log::{LevelFilter, ParseLevelError};
use log4rs::{
    append::{
        console::ConsoleAppender,
        rolling_file::{
            policy::{
                self,
                compound::{roll, trigger::size::SizeTrigger},
            },
            RollingFileAppender,
        },
    },
    config,
    encode::pattern::PatternEncoder,
};

#[derive(Debug, structopt::StructOpt)]
pub struct LoggerParams {
    /// Disable the log rotation.
    //  Use `log4rs` as `env_logger` can't print the message into file directly.
    #[structopt(long)]
    pub no_log_rotation: bool,

    /// Print the log message to the console aside from the log file.
    #[structopt(long)]
    pub enable_console_log: bool,

    /// Specify the path of directory which will contain the log files.
    ///
    /// The directory will be created if it does not exist.
    #[structopt(long, default_value = "./log")]
    pub log_dir: String,

    /// Specify the name of log file.
    ///
    /// The latest log file would be created at ./log/chainx.log.
    #[structopt(long, default_value = "chainx.log")]
    pub log_filename: String,

    /// Rotate the log file when it exceeds this size (in MB).
    #[structopt(long, default_value = "300")]
    pub log_size: u64,

    /// The maximum number of log rorations.
    ///
    /// By default the generated log files would be like `chainx.log.0`,
    /// `chainx.log.1`, ... `chainx.log.10`. Once the number of generated log files
    /// are larger than this variable, the oldest one will be removed.
    #[structopt(long, default_value = "10")]
    pub log_roll_count: u32,

    /// Compress the old log file to save some disk space.
    ///
    /// The compressed log file would be like `chainx.log.gz.0` by default.
    #[structopt(long)]
    pub log_compression: bool,
}

#[derive(Debug, Eq, PartialEq)]
struct Directive {
    name: Option<String>,
    level: LevelFilter,
}

impl FromStr for Directive {
    type Err = ParseLevelError;
    fn from_str(from: &str) -> Result<Self, Self::Err> {
        // `info` or `runtime=debug`
        let v: Vec<&str> = from.split('=').collect();
        assert!(v.len() == 1 || v.len() == 2);
        if v.len() == 1 {
            v[0].parse::<LevelFilter>()
                .map(|level| Self { name: None, level })
        } else {
            v[1].parse::<LevelFilter>().map(|level| Self {
                name: Some(v[0].into()),
                level,
            })
        }
    }
}

fn parse_directives(dirs: impl AsRef<str>) -> Vec<Directive> {
    dirs.as_ref()
        .split(',')
        .filter_map(|s| s.trim().parse().ok())
        .collect()
}

fn global_level(dirs: &[Directive]) -> LevelFilter {
    dirs.iter()
        .filter(|d| d.name.is_none())
        .map(|d| d.level)
        .max()
        .unwrap_or(LevelFilter::Info)
}

/// Parses the log filters and returns a vector of `Directive`.
///
/// The log filters should be a list of comma-separated values.
/// Example: `foo=trace,bar=debug,baz=info`
fn parse_log_filters(pattern: &str) -> (Vec<Directive>, LevelFilter) {
    let dirs = parse_directives(pattern);
    let global_level = global_level(&dirs);
    (dirs, global_level)
}

/// Initialize the global logger using log4rs.
///
/// The Substrate Logger will not registered if this one succeeds.
pub fn init(log_filters: &str, params: &LoggerParams) -> Result<(), String> {
    if params.log_size == 0 {
        return Err("the `--log-size` can't be 0".to_string());
    }

    let (directives, global_level) = parse_log_filters(log_filters);
    let (console_pattern, log_file_pattern) = if global_level >= LevelFilter::Info {
        (
            "{d(%Y-%m-%d %H:%M:%S:%3f)} {T} {h({l})} {t}  {m}\n",
            "{d(%Y-%m-%d %H:%M:%S:%3f)} {T} {l} {t}  {m}\n", // remove color
        )
    } else {
        (
            "{d(%Y-%m-%d %H:%M:%S:%3f)} {h({l})} {m}\n",
            "{d(%Y-%m-%d %H:%M:%S:%3f)} {l} {m}\n", // remove color
        )
    };

    let full_log_filename = format!(
        "{}{}{}",
        params.log_dir,
        std::path::MAIN_SEPARATOR,
        &params.log_filename
    );

    let roller_pattern = if params.log_compression {
        full_log_filename.clone() + ".gz"
    } else {
        full_log_filename.clone()
    };

    let roller = roll::fixed_window::FixedWindowRoller::builder()
        .build(&format!("{}.{{}}", roller_pattern), params.log_roll_count)
        .map_err(|e| format!("log rotate file:{:?}", e))?;

    let policy = policy::compound::CompoundPolicy::new(
        Box::new(SizeTrigger::new(params.log_size * 1024 * 1024)), // log_size MB
        Box::new(roller),
    );

    let roll_file = RollingFileAppender::builder()
        .encoder(Box::new(PatternEncoder::new(log_file_pattern)))
        .build(full_log_filename, Box::new(policy))
        .map_err(|e| format!("{}", e))?;

    let mut config_builder = if params.enable_console_log {
        let console = ConsoleAppender::builder()
            .encoder(Box::new(PatternEncoder::new(console_pattern)))
            .build();
        config::Config::builder()
            .appender(config::Appender::builder().build("console", Box::new(console)))
            .appender(config::Appender::builder().build("roll", Box::new(roll_file)))
    } else {
        config::Config::builder()
            .appender(config::Appender::builder().build("roll", Box::new(roll_file)))
    };

    for d in directives {
        if let Some(name) = d.name {
            config_builder = config_builder.logger(config::Logger::builder().build(name, d.level));
        }
    }

    let root = if params.enable_console_log {
        config::Root::builder()
            .appender("roll")
            .appender("console")
            .build(global_level)
    } else {
        config::Root::builder().appender("roll").build(global_level)
    };

    let log_config = config_builder
        .build(root)
        .expect("Construct log config failure");

    if let Err(e) = log4rs::init_config(log_config) {
        log::warn!("Registering ChainX Logger failed: {:?}", e);
    }

    Ok(())
}

#[test]
fn test_directive() {
    assert_eq!(
        parse_log_filters("info,runtime=debug,debug"),
        (
            vec![
                Directive {
                    name: None,
                    level: LevelFilter::Info
                },
                Directive {
                    name: Some("runtime".into()),
                    level: LevelFilter::Debug
                },
                Directive {
                    name: None,
                    level: LevelFilter::Debug
                },
            ],
            LevelFilter::Debug
        )
    );
}
