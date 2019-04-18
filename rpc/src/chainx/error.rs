// Copyright 2018-2019 Chainpool.

use error_chain::*;

use std::str;

use crate::errors;
use crate::rpc;

error_chain! {
    foreign_links {
        Client(client::error::Error) #[doc = "Client error"];
    }
    errors {
        /// Not implemented yet
        Unimplemented {
            description("not yet implemented"),
            display("Method Not Implemented"),
        }
        ChainErr {
            description("Not has this chain id"),
            display("Not has this chain id"),
        }
        /// Get certlist failed
        CertNameErr {
            description("Get cert name list failed"),
            display("Get cert name list failed"),
        }
        /// Get Properties failed
        CertPropErr {
            description("Get cert Properties failed"),
            display("Get cert Properties failed"),
        }
        /// Get Owner failed
        CertOwnerErr {
            description("Get cert Owner failed"),
            display("Get cert Owner failed"),
        }
        /// Get Remaining Shares failed
        CertRemainingSharesErr {
            description("Get cert Remaining Shares failed"),
            display("Get cert Remaining Shares failed"),
        }
        TradingPairIndexErr{
            description("TradingPair Index Error"),
            display("TradingPair Index Error"),
        }
        QuotationsPieceErr{
            description("Quotations Piece Err"),
            display("Quotations Piece Err"),
        }
        PageSizeErr{
            description("Page Size Must Between 0~100"),
            display("Page Size Must Between 0~100"),
        }
        PageIndexErr{
            description("Page Index Error"),
            display("Page Index Error"),
        }
        DecodeErr {
            description("Decode Data Error"),
            display("Decode Data Error"),
        }
        BinanryStartErr {
            description("Start With 0x"),
            display("Start With 0x"),
        }
        HexDecodeErr {
            description("Decode Hex Err"),
            display("Decode Hex Err"),
        }
        /// Execution error.
        Execution(e: Box<state_machine::Error>) {
            description("state execution error"),
            display("Execution: {}", e),
        }
        RuntimeErr(e: Vec<u8>) {
            description("runtime error"),
            display("error: {:}", str::from_utf8(&e).unwrap_or_default()),
        }
    }
}

const ERROR: i64 = 1600;

impl From<Box<state_machine::Error>> for Error {
    fn from(e: Box<state_machine::Error>) -> Self {
        ErrorKind::Execution(e).into()
    }
}

impl From<Error> for rpc::Error {
    fn from(e: Error) -> Self {
        match e {
            Error(ErrorKind::Unimplemented, _) => errors::unimplemented(),
            Error(ErrorKind::CertNameErr, _) => rpc::Error {
                code: rpc::ErrorCode::ServerError(ERROR + 1),
                message: "Get cert name list failed.".into(),
                data: None,
            },
            Error(ErrorKind::CertPropErr, _) => rpc::Error {
                code: rpc::ErrorCode::ServerError(ERROR + 2),
                message: "Get cert Properties failed.".into(),
                data: None,
            },
            Error(ErrorKind::CertOwnerErr, _) => rpc::Error {
                code: rpc::ErrorCode::ServerError(ERROR + 3),
                message: "Get cert Owner failed.".into(),
                data: None,
            },
            Error(ErrorKind::CertRemainingSharesErr, _) => rpc::Error {
                code: rpc::ErrorCode::ServerError(ERROR + 4),
                message: "Get cert Remaining Shares failed.".into(),
                data: None,
            },
            Error(ErrorKind::QuotationsPieceErr, _) => rpc::Error {
                code: rpc::ErrorCode::ServerError(ERROR + 5),
                message: "Quotations Piece Err.".into(),
                data: None,
            },
            Error(ErrorKind::TradingPairIndexErr, _) => rpc::Error {
                code: rpc::ErrorCode::ServerError(ERROR + 6),
                message: "TradingPair Index Error.".into(),
                data: None,
            },
            Error(ErrorKind::PageSizeErr, _) => rpc::Error {
                code: rpc::ErrorCode::ServerError(ERROR + 7),
                message: "Page Size Must Between 0~100.".into(),
                data: None,
            },
            Error(ErrorKind::PageIndexErr, _) => rpc::Error {
                code: rpc::ErrorCode::ServerError(ERROR + 8),
                message: "Page Index Error.".into(),
                data: None,
            },
            Error(ErrorKind::DecodeErr, _) => rpc::Error {
                code: rpc::ErrorCode::ServerError(ERROR + 9),
                message: "Decode data error.".into(),
                data: None,
            },
            Error(ErrorKind::BinanryStartErr, _) => rpc::Error {
                code: rpc::ErrorCode::ServerError(ERROR + 10),
                message: "Start With 0x.".into(),
                data: None,
            },
            Error(ErrorKind::HexDecodeErr, _) => rpc::Error {
                code: rpc::ErrorCode::ServerError(ERROR + 11),
                message: "Decode Hex Err.".into(),
                data: None,
            },
            Error(ErrorKind::Execution(e), _) => rpc::Error {
                code: rpc::ErrorCode::ServerError(ERROR + 12),
                message: format!("Execution: {}", e),
                data: None,
            },
            Error(ErrorKind::RuntimeErr(e), _) => rpc::Error {
                code: rpc::ErrorCode::ServerError(ERROR + 13),
                message: format!(
                    "Runtime execute error: {:}",
                    str::from_utf8(&e).unwrap_or_default()
                ),
                data: None,
            },
            e => errors::internal(e),
        }
    }
}

impl Error {
    /// Chain a state error.
    pub fn from_state(e: Box<state_machine::Error + Send>) -> Self {
        ErrorKind::Execution(e).into()
    }
}
