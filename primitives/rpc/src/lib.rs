use std::fmt::Debug;

pub use jsonrpc_core::{Error, ErrorCode, Result};

/// The call to runtime failed.
pub const RUNTIME_ERROR: i64 = 1;

/// The transaction was not decodable.
pub const DECODE_ERROR: i64 = 100;

/// The bytes failed to be decoded as hex.
pub const HEX_DECODE_ERROR: i64 = DECODE_ERROR + 1;

/// Converts a runtime trap into an RPC error.
pub fn runtime_error_into_rpc_err(err: impl Debug) -> Error {
    Error {
        code: ErrorCode::ServerError(RUNTIME_ERROR),
        message: "Runtime trapped".into(),
        data: Some(format!("{:?}", err).into()),
    }
}

pub fn hex_decode_error_into_rpc_err(err: impl Debug) -> Error {
    Error {
        code: ErrorCode::ServerError(HEX_DECODE_ERROR),
        message: "Failed to decode hex".into(),
        data: Some(format!("{:?}", err).into()),
    }
}

pub fn new_runtime_error(message: String, err: impl Debug) -> Error {
    Error {
        code: ErrorCode::ServerError(RUNTIME_ERROR),
        message,
        data: Some(format!("{:?}", err).into()),
    }
}
