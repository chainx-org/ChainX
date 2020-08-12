// Copyright 2018-2020 Chainpool.

use super::*;
use log4rs::{
    append::{
        console::ConsoleAppender,
        rolling_file::{
            policy::{
                self,
                compound::{roll, trigger},
            },
            RollingFileAppender,
        },
    },
    config,
    encode::pattern::PatternEncoder,
};

pub fn init(spec: &str, params: ChainXParams) -> Result<(), String> {
    let (directives, filter) = parse_spec(spec);
    let filter = filter.unwrap_or(LevelFilter::Info);

    let (pattern1, pattern2) = if filter > LevelFilter::Info {
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

    let console = ConsoleAppender::builder()
        .encoder(Box::new(PatternEncoder::new(pattern1)))
        .build();

    let log = params.log_dir.clone() + "/" + &params.log_name;
    let log_file = if params.log_compression {
        log.clone() + ".gz"
    } else {
        log.clone()
    };

    if params.log_size == 0 {
        return Err("the `--log-size` can't be 0".to_string());
    }

    let trigger = trigger::size::SizeTrigger::new(1024 * 1024 * params.log_size);
    let roll_pattern = format!("{}.{{}}", log_file);
    let roll = roll::fixed_window::FixedWindowRoller::builder()
        .build(roll_pattern.as_str(), params.log_roll_count)
        .map_err(|e| format!("log rotate file:{:?}", e))?;

    let policy = policy::compound::CompoundPolicy::new(Box::new(trigger), Box::new(roll));
    let roll_file = RollingFileAppender::builder()
        .encoder(Box::new(PatternEncoder::new(pattern2)))
        .build(log, Box::new(policy))
        .map_err(|e| format!("{}", e))?;

    let mut tmp_builder = if params.log_console {
        config::Config::builder()
            .appender(config::Appender::builder().build("console", Box::new(console)))
            .appender(config::Appender::builder().build("roll", Box::new(roll_file)))
    } else {
        config::Config::builder()
            .appender(config::Appender::builder().build("roll", Box::new(roll_file)))
    };

    for d in directives {
        if let Some(name) = d.name {
            tmp_builder = tmp_builder.logger(config::Logger::builder().build(name, d.level));
        }
    }

    let root = if params.log_console {
        config::Root::builder()
            .appender("roll")
            .appender("console")
            .build(filter)
    } else {
        config::Root::builder().appender("roll").build(filter)
    };

    let log_config = tmp_builder
        .build(root)
        .expect("Construct log config failure");

    log4rs::init_config(log_config).expect("Initializing log config shouldn't be fail");

    Ok(())
}
