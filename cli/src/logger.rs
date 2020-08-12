use std::str::FromStr;

use log::LevelFilter;
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

use crate::cli::Cli;

#[derive(Debug)]
struct Directive {
    name: Option<String>,
    level: LevelFilter,
}

/// Parses the log filters and returns a vector of `Directive`.
///
/// The log filters should be a list of comma-separated values.
/// Example: `foo=trace,bar=debug,baz=info`
fn parse_log_filters(log_filters: &str) -> (Vec<Directive>, Option<LevelFilter>) {
    let mut dirs = Vec::new();

    let mut parts = log_filters.split('/');
    let mods = parts.next();
    let filter = parts.next().and_then(|s| FromStr::from_str(s).ok());
    if parts.next().is_some() {
        eprintln!(
            "warning: invalid logging log_filters '{}', ignoring it (too many '/'s)",
            log_filters
        );
        return (dirs, None);
    }

    mods.map(|m| {
        for s in m.split(',') {
            if s.is_empty() {
                continue;
            }
            let mut parts = s.split('=');
            let (log_level, name) =
                match (parts.next(), parts.next().map(|s| s.trim()), parts.next()) {
                    (Some(part0), None, None) => {
                        // if the single argument is a log-level string or number,
                        // treat that as a global fallback
                        match part0.parse() {
                            Ok(num) => (num, None),
                            Err(_) => (LevelFilter::max(), Some(part0)),
                        }
                    }
                    (Some(part0), Some(""), None) => (LevelFilter::max(), Some(part0)),
                    (Some(part0), Some(part1), None) => match part1.parse() {
                        Ok(num) => (num, Some(part0)),
                        _ => {
                            eprintln!(
                                "warning: invalid logging log_filters '{}', \
                                 ignoring it",
                                part1
                            );
                            continue;
                        }
                    },
                    _ => {
                        eprintln!(
                            "warning: invalid logging log_filters '{}', \
                             ignoring it",
                            s
                        );
                        continue;
                    }
                };
            dirs.push(Directive {
                name: name.map(|s| s.to_string()),
                level: log_level,
            });
        }
    });

    let mut tmp_filter = LevelFilter::Off;
    for d in dirs.iter() {
        if d.name.is_none() {
            if d.level > tmp_filter {
                tmp_filter = d.level;
            }
        }
    }

    let filter = if let Some(f) = filter {
        if f > tmp_filter {
            Some(f)
        } else {
            Some(tmp_filter)
        }
    } else {
        if tmp_filter == LevelFilter::Off {
            None
        } else {
            Some(tmp_filter)
        }
    };

    return (dirs, filter);
}

/// Initialize the log4rs related configuration.
pub fn init(log_filters: &str, params: &Cli) -> Result<(), String> {
    if params.log_size == 0 {
        return Err("the `--log-size` can't be 0".to_string());
    }

    let (directives, filter) = parse_log_filters(log_filters);
    let filter = filter.unwrap_or(LevelFilter::Info);

    let (console_pattern, log_file_pattern) = if filter > LevelFilter::Info {
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

    let log_file = if params.log_compression {
        full_log_filename.clone() + ".gz"
    } else {
        full_log_filename.clone()
    };

    let roller = roll::fixed_window::FixedWindowRoller::builder()
        .build(&format!("{}.{{}}", log_file), params.log_roll_count)
        .map_err(|e| format!("log rotate file:{:?}", e))?;

    let policy = policy::compound::CompoundPolicy::new(
        Box::new(SizeTrigger::new(1024 * 1024 * params.log_size)), // log_size MB
        Box::new(roller),
    );

    let roll_file = RollingFileAppender::builder()
        .encoder(Box::new(PatternEncoder::new(log_file_pattern)))
        .build(full_log_filename, Box::new(policy))
        .map_err(|e| format!("{}", e))?;

    let mut config_builder = if params.log_console {
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

    let root = if params.log_console {
        config::Root::builder()
            .appender("roll")
            .appender("console")
            .build(filter)
    } else {
        config::Root::builder().appender("roll").build(filter)
    };

    let log_config = config_builder
        .build(root)
        .expect("Construct log config failure");

    log4rs::init_config(log_config).expect("The log4rs config initialization shouldn't fail; qed");

    Ok(())
}
