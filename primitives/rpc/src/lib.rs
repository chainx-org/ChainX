use std::fmt::Debug;

pub use jsonrpc_core::{Error, ErrorCode, Result};

/// The call to runtime failed.
pub const RUNTIME_ERROR: i64 = 1;

/// The call related to trustee to runtime failed.
pub const RUNTIME_TRUSTEE_ERROR: i64 = RUNTIME_ERROR + 100;

/// Decode the generic trustee info failed.
pub const RUNTIME_TRUSTEE_DECODE_ERROR: i64 = RUNTIME_TRUSTEE_ERROR + 1;

/// The trustees are inexistent.
pub const RUNTIME_TRUSTEE_INEXISTENT_ERROR: i64 = RUNTIME_TRUSTEE_ERROR + 2;

/// The transaction was not decodable.
pub const DECODE_ERROR: i64 = 1000;

/// The bytes failed to be decoded as hex.
pub const DECODE_HEX_ERROR: i64 = DECODE_ERROR + 1;

/// Converts a runtime trap into an RPC error.
pub fn runtime_error_into_rpc_err(err: impl Debug) -> Error {
    Error {
        code: ErrorCode::ServerError(RUNTIME_ERROR),
        message: "Runtime trapped".into(),
        data: Some(format!("{:?}", err).into()),
    }
}

/// Converts a trustee runtime trap into an RPC error.
pub fn trustee_decode_error_into_rpc_err(err: impl Debug) -> Error {
    Error {
        code: ErrorCode::ServerError(RUNTIME_TRUSTEE_DECODE_ERROR),
        message: "Can not decode generic trustee session info".into(),
        data: Some(format!("{:?}", err).into()),
    }
}

/// Converts a trustee runtime trap into an RPC error.
pub fn trustee_inexistent_error_into_rpc_err() -> Error {
    Error {
        code: ErrorCode::ServerError(RUNTIME_TRUSTEE_INEXISTENT_ERROR),
        message: "Trustee does not exist".into(),
        data: None,
    }
}

/// Converts a hex decode error into an RPC error.
pub fn hex_decode_error_into_rpc_err(err: impl Debug) -> Error {
    Error {
        code: ErrorCode::ServerError(DECODE_HEX_ERROR),
        message: "Failed to decode hex".into(),
        data: Some(format!("{:?}", err).into()),
    }
}
