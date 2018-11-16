// Copyright 2018 Chainpool.

//! Errors that can occur during the consensus process.

use primitives::AuthorityId;

error_chain! {
    links {
        ChainXApi(::chainx_api::Error, ::chainx_api::ErrorKind);
        Bft(::bft::Error, ::bft::ErrorKind);
    }

    errors {
        InvalidDutyRosterLength(expected: usize, got: usize) {
            description("Duty Roster had invalid length"),
            display("Invalid duty roster length: expected {}, got {}", expected, got),
        }
        NotValidator(id: AuthorityId) {
            description("Local account ID not a validator at this block."),
            display("Local account ID ({:?}) not a validator at this block.", id),
        }
        PrematureDestruction {
            description("Proposer destroyed before finishing proposing or evaluating"),
            display("Proposer destroyed before finishing proposing or evaluating"),
        }
        Timer(e: ::tokio::timer::Error) {
            description("Failed to register or resolve async timer."),
            display("Timer failed: {}", e),
        }
        Executor(e: ::futures::future::ExecuteErrorKind) {
            description("Unable to dispatch agreement future"),
            display("Unable to dispatch agreement future: {:?}", e),
        }
    }
}

impl From<::bft::InputStreamConcluded> for Error {
    fn from(err: ::bft::InputStreamConcluded) -> Self {
        ::bft::Error::from(err).into()
    }
}
