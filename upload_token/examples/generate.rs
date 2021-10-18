use qiniu_credential::{Credential, StaticCredentialProvider};
use qiniu_upload_token::{UploadPolicyBuilder, UploadTokenProvider};
use std::{error::Error, result::Result, time::Duration};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "generate-upload-token")]
struct Opt {
    /// Qiniu Access Key
    #[structopt(short, long)]
    access_key: String,
    /// Qiniu Secret Key
    #[structopt(short, long)]
    secret_key: String,
    /// Qiniu Bucket Name
    #[structopt(short, long)]
    bucket_name: String,
    /// Upload Token Lifetime
    #[structopt(short, long, default_value = "86400")]
    lifetime: u64,
}

fn main() -> Result<(), Box<dyn Error>> {
    let opt: Opt = Opt::from_args();

    let upload_policy = UploadPolicyBuilder::new_policy_for_bucket(
        &opt.bucket_name,
        Duration::from_secs(opt.lifetime),
    )
    .build();
    let upload_token = upload_policy.into_upload_token_provider(StaticCredentialProvider::new(
        Credential::new(opt.access_key, opt.secret_key),
    ));
    println!("{}", upload_token.to_token_string(&Default::default())?);
    Ok(())
}
