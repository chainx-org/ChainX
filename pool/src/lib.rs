// Copyright 2018 Chainpool.

extern crate substrate_runtime_primitives as runtime_primitives;
extern crate substrate_primitives as substrate_primitives;
extern crate substrate_extrinsic_pool as extrinsic_pool;
extern crate substrate_codec as codec;
extern crate substrate_client_db;
extern crate substrate_executor;
extern crate substrate_network;
extern crate substrate_client;
extern crate chainx_primitives;
extern crate chainx_executor;
extern crate chainx_runtime;
extern crate chainx_api;
extern crate ed25519;

#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate log;

mod pool;
mod error;

pub use pool::TransactionPool;
pub use pool::PoolApi;

