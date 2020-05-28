// Copyright 2018-2019 Chainpool.

use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;

use chainx_cli::VersionInfo;
use futures::{future, sync::oneshot, Future};
use serde_json::value::Value;

// handles ctrl-c
struct Exit;
impl chainx_cli::IntoExit for Exit {
    type Exit = future::MapErr<oneshot::Receiver<()>, fn(oneshot::Canceled) -> ()>;
    fn into_exit(self) -> Self::Exit {
        // can't use signal directly here because CtrlC takes only `Fn`.
        let (exit_send, exit) = oneshot::channel();

        let exit_send_cell = RefCell::new(Some(exit_send));
        ctrlc::set_handler(move || {
            if let Some(exit_send) = exit_send_cell
                .try_borrow_mut()
                .expect("signal handler not reentrant; qed")
                .take()
            {
                exit_send.send(()).expect("Error sending exit notification");
            }
        })
        .expect("Error setting Ctrl-C handler");

        exit.map_err(drop)
    }
}

fn combine_conf(
    cmd_args: Vec<String>,
    path: &std::path::Path,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let opts_from_cmd = cmd_args
        .iter()
        .filter(|i| i.starts_with("--"))
        .map(|i| i.split('=').collect::<Vec<&str>>()[0])
        .collect::<Vec<&str>>();

    let mut bytes = Vec::new();
    File::open(path)?.read_to_end(&mut bytes)?;

    let json: HashMap<String, Value> =
        serde_json::from_slice(&bytes).expect("JSON was not well-formatted");

    let mut opts: Vec<String> = Vec::new();

    for (opt, v) in json.clone().into_iter() {
        let opt = format!("--{}", opt);

        match v {
            Value::Bool(b) => {
                if !opts_from_cmd.contains(&&opt.as_ref()) && b {
                    opts.push(opt.to_string());
                }
            }
            Value::Number(b) => {
                if !opts_from_cmd.contains(&&opt.as_ref()) {
                    opts.push(format!("{}={}", opt, b));
                }
            }
            Value::String(v) => {
                if !v.is_empty() && !opts_from_cmd.contains(&&opt.as_ref()) {
                    opts.push(format!("{}={}", opt, v));
                }
            }
            Value::Array(arr) => {
                let arr = arr
                    .iter()
                    .map(|a| format!("{}={}", opt, a.as_str().unwrap()))
                    .collect::<Vec<String>>();
                opts.extend(arr);
            }
            Value::Null => {}
            Value::Object(_) => panic!("Unsupported nested configuration"),
        }
    }

    let mut args = cmd_args;
    args.extend(opts);
    Ok(args)
}

fn try_combine_options_config(cmd_args: Vec<String>) -> Vec<String> {
    let mut options_conf: Option<String> = None;
    let mut args_iter = cmd_args.iter();
    while let Some(arg) = args_iter.next() {
        if arg == "--config" {
            let conf = args_iter.next().expect(
                "The argument '--config <CONFIG_JSON_PATH>' requires a value but none was supplied",
            );
            options_conf = Some(conf.to_string());
        } else if arg.starts_with("--config=") {
            options_conf = Some(arg.split('=').collect::<Vec<&str>>()[1].to_string());
        }
    }

    if let Some(options_conf) = options_conf {
        let path = std::path::Path::new(&options_conf);
        combine_conf(cmd_args, path).expect("Error processing --config")
    } else {
        cmd_args
    }
}

fn main() {
    let version = VersionInfo {
        name: "ChainX",
        commit: env!("VERGEN_SHA_SHORT"),
        version: env!("CARGO_PKG_VERSION"),
        executable_name: "ChainX",
        author: "ChainX community",
        description: "Fully Decentralized Interchain Crypto Asset Management on Polkadot",
        support_url: "https://github.com/chainx-org/ChainX",
    };

    let args = try_combine_options_config(::std::env::args().collect::<Vec<String>>());

    if let Err(e) = chainx_cli::run(args, Exit, version) {
        eprintln!("Error starting the node: {}\n\n{:?}", e, e);
        std::process::exit(1)
    }
}
