use super::bucket::Bucket;
use crate::{config::Config, http, utils::auth::Auth};

pub struct Uploader {
    auth: Auth,
    config: Config,
}
