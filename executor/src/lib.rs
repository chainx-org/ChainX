// Copyright 2019-2021 ChainX Project Authors. Licensed under GPL-3.0.

pub use sc_executor::NativeElseWasmExecutor;

pub struct ChainXExecutor;
impl sc_executor::NativeExecutionDispatch for ChainXExecutor {
    type ExtendHostFunctions = (
        frame_benchmarking::benchmarking::HostFunctions,
        xp_io::ss_58_codec::HostFunctions,
    );

    fn dispatch(method: &str, data: &[u8]) -> Option<Vec<u8>> {
        chainx_runtime::api::dispatch(method, data)
    }

    fn native_version() -> sc_executor::NativeVersion {
        chainx_runtime::native_version()
    }
}

pub struct DevExecutor;
impl sc_executor::NativeExecutionDispatch for DevExecutor {
    type ExtendHostFunctions = (
        frame_benchmarking::benchmarking::HostFunctions,
        xp_io::ss_58_codec::HostFunctions,
    );

    fn dispatch(method: &str, data: &[u8]) -> Option<Vec<u8>> {
        dev_runtime::api::dispatch(method, data)
    }

    fn native_version() -> sc_executor::NativeVersion {
        dev_runtime::native_version()
    }
}

pub struct MalanExecutor;
impl sc_executor::NativeExecutionDispatch for MalanExecutor {
    type ExtendHostFunctions = (
        frame_benchmarking::benchmarking::HostFunctions,
        xp_io::ss_58_codec::HostFunctions,
    );

    fn dispatch(method: &str, data: &[u8]) -> Option<Vec<u8>> {
        malan_runtime::api::dispatch(method, data)
    }

    fn native_version() -> sc_executor::NativeVersion {
        malan_runtime::native_version()
    }
}
