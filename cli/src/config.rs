// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use serde_json::value::Value;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;

fn read_config_file(path: &Path) -> Result<HashMap<String, Value>, Box<dyn std::error::Error>> {
    let mut bytes = Vec::new();
    File::open(path)?.read_to_end(&mut bytes)?;

    Ok(serde_json::from_slice(&bytes).unwrap_or_else(|_| {
        panic!(
            "JSON was not well-formatted, please ensure {} is a valid JSON file.",
            path.display()
        )
    }))
}

const SUB_COMMANDS: [&str; 13] = [
    "benchmark",
    "build-spec",
    "check-block",
    "export-blocks",
    "export-state",
    "help",
    "import-blocks",
    "key",
    "purge-chain",
    "revert",
    "sign",
    "vanity",
    "verify",
];

/// Extends the origin cli arg list with the options from the config file.
///
/// Only the options that do not appear in the command line will be appended.
fn extend_cli_args(
    cli_args: Vec<String>,
    path: Option<&Path>,
    default_opts: HashMap<String, String>,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    // Gather all the FLAGS/OPTIONS passed from the command line.
    let cli_opts = cli_args
        .iter()
        .filter(|i| i.starts_with("--"))
        .filter_map(|i| i.split('=').next())
        .collect::<Vec<_>>();

    let mut config_opts = Vec::new();
    let mut default_opts = default_opts
        .into_iter()
        .filter(|(k, _)| !cli_opts.contains(&format!("--{}", k).as_ref()))
        .collect::<HashMap<_, _>>();

    if let Some(path) = path {
        for (key, value) in read_config_file(path)?.into_iter() {
            // Remove the option that has been configured in the config file.
            if default_opts.contains_key(key.as_str()) {
                default_opts.remove(key.as_str());
            }

            let opt = format!("--{}", key);
            match value {
                Value::Bool(b) => {
                    if !cli_opts.contains(&opt.as_ref()) && b {
                        config_opts.push(opt.to_string());
                    }
                }
                Value::Number(n) => {
                    if !cli_opts.contains(&opt.as_ref()) {
                        config_opts.push(format!("{}={}", opt, n));
                    }
                }
                Value::String(s) => {
                    if !s.is_empty() && !cli_opts.contains(&opt.as_ref()) {
                        config_opts.push(format!("{}={}", opt, s));
                    }
                }
                Value::Array(arr) => {
                    config_opts.extend(arr.into_iter().map(|a| {
                        format!(
                            "{}={}",
                            opt,
                            a.as_str().expect("Array item can always be a String; qed")
                        )
                    }));
                }
                Value::Null => {}
                Value::Object(_) => {
                    panic!("The nested configuration in the config file is unsupported.")
                }
            }
        }
    }

    if let Some(sub_command) = cli_args.get(1) {
        // Injecting `default_opts()` only makes sense in the context of no specified subcommands.
        if !SUB_COMMANDS.contains(&sub_command.as_str()) {
            for (key, value) in default_opts {
                config_opts.push(format!("--{}={}", key, value));
            }
        }
    }

    let mut args = cli_args;
    args.extend(config_opts);
    Ok(args)
}

fn default_opts() -> HashMap<String, String> {
    let mut map = HashMap::new();
    map.insert("port".into(), "20222".to_string());
    map.insert("rpc-port".into(), "8086".to_string());
    map.insert("ws-port".into(), "8087".to_string());
    map
}

/// Try to inject the options from the config file.
pub fn preprocess_cli_args(cli_args: Vec<String>) -> Vec<String> {
    let mut config_path: Option<String> = None;

    // Find the last --config option.
    let mut cli_args_iter = cli_args.iter();
    while let Some(arg) = cli_args_iter.next() {
        if arg == "--config" {
            let path = cli_args_iter
                .next()
                .expect("The argument '--config <PATH>' requires a value but none was supplied");
            config_path = Some(path.to_string());
        } else if arg.starts_with("--config=") {
            config_path = arg.split('=').nth(1).map(|s| s.to_string());
            assert!(config_path.is_some(), "missing PATH in --config=<PATH>");
        }
    }

    let path: Option<&Path> = config_path.as_ref().map(Path::new);
    match extend_cli_args(cli_args, path, default_opts()) {
        Ok(args) => args,
        Err(e) => panic!("{}", e.to_string()),
    }
}
