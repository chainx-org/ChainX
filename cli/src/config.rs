use serde_json::value::Value;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;

fn read_config_file(path: &Path) -> Result<HashMap<String, Value>, Box<dyn std::error::Error>> {
    let mut bytes = Vec::new();
    File::open(path)?.read_to_end(&mut bytes)?;

    Ok(serde_json::from_slice(&bytes).expect(&format!(
        "JSON was not well-formatted, please ensure {} is a valid JSON file.",
        path.display()
    )))
}

/// Extends the origin cli arg list with the options from the config file.
///
/// Only the options that do not appear in the command line will be appended.
fn extend_cli_args(
    cli_args: Vec<String>,
    path: &std::path::Path,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    // Gather all the command line FLAG/OPTION
    let cli_opts = cli_args
        .iter()
        .filter(|i| i.starts_with("--"))
        .filter_map(|i| i.split('=').next())
        .collect::<Vec<_>>();

    let mut config_opts = Vec::new();

    for (key, value) in read_config_file(path)?.into_iter() {
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

    let mut args = cli_args;
    args.extend(config_opts);
    Ok(args)
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
            config_path = arg.split('=').skip(1).next().map(|s| s.to_string());
            assert!(config_path.is_some(), "missing value in --config=[value]");
        }
    }

    if let Some(config) = config_path {
        match extend_cli_args(cli_args, Path::new(&config)) {
            Ok(args) => args,
            Err(e) => panic!(e.to_string()),
        }
    } else {
        cli_args
    }
}
