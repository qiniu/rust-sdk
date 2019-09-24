use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub(super) struct ErrorResponse {
    pub(super) error: Option<String>,
}
