use std::str::FromStr;

use log::LevelFilter;

use crate::cli::Cli;

#[derive(Debug)]
struct Directive {
    name: Option<String>,
    level: LevelFilter,
}
/// and return a vector with log directives.
fn parse_spec(spec: &str) -> (Vec<Directive>, Option<LevelFilter>) {
    let mut dirs = Vec::new();

    let mut parts = spec.split('/');
    let mods = parts.next();
    let filter = parts.next().and_then(|s| FromStr::from_str(s).ok());
    if parts.next().is_some() {
        eprintln!(
            "warning: invalid logging spec '{}', ignoring it (too many '/'s)",
            spec
        );
        return (dirs, None);
    }
    mods.map(|m| {
        for s in m.split(',') {
            if s.len() == 0 {
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
                                "warning: invalid logging spec '{}', \
                                 ignoring it",
                                part1
                            );
                            continue;
                        }
                    },
                    _ => {
                        eprintln!(
                            "warning: invalid logging spec '{}', \
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
        if d.name == None {
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

pub fn init_logger_log4rs(spec: &str, params: &Cli) -> Result<(), String> {
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
