use jsonrpc_core::{Error, ErrorCode};

pub enum ChainXRpcErr {}

const BASE_ERROR: i64 = 5000;

// impl From<ChainXRpcErr> for Error {
//     fn from(e: ChainXRpcErr) -> Self {
//         match e {
//         }
//     }
// }
