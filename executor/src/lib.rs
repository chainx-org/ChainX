// Copyright 2018 Chainpool.

//! A `CodeExecutor` specialisation which uses natively compiled runtime when the wasm to be
//! executed is equivalent to the natively compiled code.

pub use substrate_executor::NativeExecutor;

use substrate_executor::native_executor_instance;

native_executor_instance!(pub Executor, chainx_runtime::api::dispatch, chainx_runtime::native_version,
  include_bytes!("../../runtime/wasm/target/wasm32-unknown-unknown/release/chainx_runtime.compact.wasm"));
