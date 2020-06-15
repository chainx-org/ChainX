use jsonrpc_core::{Error, ErrorCode};

pub enum XRpcErr {}

const BASE_ERROR: i64 = 5000;

// impl From<CiRpcErr> for Error {
//     fn from(e: CiRpcErr) -> Self {
//         match e {
//         }
//     }
// }
