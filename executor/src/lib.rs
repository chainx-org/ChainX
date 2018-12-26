// Copyright 2018 Chainpool.

//! A `CodeExecutor` specialisation which uses natively compiled runtime when the wasm to be
//! executed is equivalent to the natively compiled code.

extern crate chainx_runtime;
#[macro_use]
extern crate substrate_executor;
#[cfg_attr(test, macro_use)]
extern crate substrate_primitives as primitives;

pub use substrate_executor::NativeExecutor;
native_executor_instance!(pub Executor, chainx_runtime::api::dispatch, chainx_runtime::native_version,
  include_bytes!("../../runtime/wasm/target/wasm32-unknown-unknown/release/chainx_runtime_wasm.compact.wasm"));
