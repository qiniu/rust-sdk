extern crate clap;
extern crate qiniu_credential;
extern crate qiniu_upload_token;

use clap::Clap;
use qiniu_credential::StaticCredentialProvider;
use qiniu_upload_token::{UploadPolicyBuilder, UploadTokenProvider};
use std::{error::Error, result::Result, time::Duration};

#[derive(Debug, Clap)]
#[clap(version = "1.0", author = "Rong Zhou <zhourong@qiniu.com>")]
struct Params {
    /// Qiniu Access Key
    #[clap(short, long)]
    access_key: String,
    /// Qiniu Secret Key
    #[clap(short, long)]
    secret_key: String,
    /// Qiniu Bucket Name
    #[clap(short, long)]
    bucket_name: String,
}

fn main() -> Result<(), Box<dyn Error>> {
    let params: Params = Params::parse();

    let upload_policy = UploadPolicyBuilder::new_policy_for_bucket(
        &params.bucket_name,
        Duration::from_secs(24 * 3600),
    )
    .build();
    let upload_token = upload_policy.into_upload_token_provider(Box::new(
        StaticCredentialProvider::new(params.access_key, params.secret_key),
    ));
    println!("{}", upload_token.to_string()?);
    Ok(())
}
