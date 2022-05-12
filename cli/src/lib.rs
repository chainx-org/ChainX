// Copyright 2019-2022 ChainX Project Authors. Licensed under GPL-3.0.

//! ChainX CLI library.

#![warn(missing_docs)]
#![warn(unused_extern_crates)]

mod chain_spec;
mod cli;
mod command;
mod config;
mod genesis;
mod logger;

pub use sc_cli::Result;

pub use self::command::*;
