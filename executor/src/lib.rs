// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use sc_executor::native_executor_instance;
pub use sc_executor::NativeExecutor;

// Declare an instance of the native executor named `Executor`. Include the wasm binary as the
// equivalent wasm code.
native_executor_instance!(
    pub ChainXExecutor,
    chainx_runtime::api::dispatch,
    chainx_runtime::native_version,
    (frame_benchmarking::benchmarking::HostFunctions, xp_io::ss_58_codec::HostFunctions),
);

native_executor_instance!(
    pub DevExecutor,
    dev_runtime::api::dispatch,
    dev_runtime::native_version,
    (frame_benchmarking::benchmarking::HostFunctions, xp_io::ss_58_codec::HostFunctions),
);

native_executor_instance!(
    pub MalanExecutor,
    malan_runtime::api::dispatch,
    malan_runtime::native_version,
    (frame_benchmarking::benchmarking::HostFunctions, xp_io::ss_58_codec::HostFunctions),
);
