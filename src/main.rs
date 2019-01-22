extern crate chainx_cli as cli;
extern crate ctrlc;
extern crate futures;

#[macro_use]
extern crate error_chain;

use cli::VersionInfo;
use futures::sync::oneshot;
use futures::{future, Future};

use std::cell::RefCell;

// handles ctrl-c
struct Exit;
impl cli::IntoExit for Exit {
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

quick_main!(run);

fn run() -> cli::error::Result<()> {
    let version = VersionInfo {
        name: "ChainX",
        commit: "VERGEN_SHA_SHORT",
        version: "CARGO_PKG_VERSION",
        executable_name: "ChainX",
        author: "ChainX community",
        description: "Fully Decentralized Interchain Crypto Asset Management on Polkadot",
    };
    cli::run(::std::env::args(), Exit, version)
}
