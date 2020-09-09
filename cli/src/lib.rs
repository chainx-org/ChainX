//! ChainX CLI library.

#![warn(missing_docs)]
#![warn(unused_extern_crates)]

mod chain_spec;
#[macro_use]
mod service;
mod cli;
mod command;
mod config;
mod genesis;
mod logger;
mod res;

pub use command::*;
pub use sc_cli::Result;
