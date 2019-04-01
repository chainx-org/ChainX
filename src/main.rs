// Copyright 2018-2019 Chainpool.

use std::cell::RefCell;

use chainx_cli::VersionInfo;
use error_chain::quick_main;
use futures::{future, sync::oneshot, Future};

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

fn run() -> chainx_cli::error::Result<()> {
    let version = VersionInfo {
        name: "ChainX",
        commit: env!("VERGEN_SHA_SHORT"),
        version: env!("CARGO_PKG_VERSION"),
        executable_name: "ChainX",
        author: "ChainX community",
        description: "Fully Decentralized Interchain Crypto Asset Management on Polkadot",
        support_url: "https://github.com/chainx-org/ChainX",
    };
    chainx_cli::run(::std::env::args(), Exit, version)
}

quick_main!(run);
