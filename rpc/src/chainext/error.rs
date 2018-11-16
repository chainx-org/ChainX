use client;
use jsonrpc_core as rpccore;

pub fn unimplemented() -> rpccore::Error {
    rpccore::Error {
        code: rpccore::ErrorCode::ServerError(1),
        message: "Not implemented yet".into(),
        data: None,
    }
}

pub fn internal<E: ::std::fmt::Debug>(e: E) -> rpccore::Error {
    warn!("Unknown error: {:?}", e);
    rpccore::Error {
        code: rpccore::ErrorCode::InternalError,
        message: "Unknown error occured".into(),
        data: Some(format!("{:?}", e).into()),
    }
}

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
    }
}

impl From<Error> for rpccore::Error {
    fn from(e: Error) -> Self {
        match e {
            Error(ErrorKind::Unimplemented, _) => unimplemented(),
            e => internal(e),
        }
    }
}
