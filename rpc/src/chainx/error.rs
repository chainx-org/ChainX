use client;
use errors;
use rpc;

error_chain! {
    links {
        Client(client::error::Error, client::error::ErrorKind) #[doc = "Client error"];
    }
    errors {
        /// Not implemented yet
        Unimplemented {
            description("not yet implemented"),
            display("Method Not Implemented"),
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
        /// Execution error.
        Execution(e: Box<state_machine::Error>) {
            description("state execution error"),
            display("Execution: {}", e),
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
