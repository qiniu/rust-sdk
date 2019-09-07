use error_chain::error_chain;
use serde::{Deserialize, Serialize};

error_chain! {
    errors {
        NoHostAvailable {
            description("no host is available"),
            display("no host is available"),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub(super) struct ErrorResponse {
    pub(super) error: Option<String>,
}
