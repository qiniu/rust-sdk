use anyhow::Result;
use qiniu_credential::Credential;
use qiniu_upload_token::UploadPolicy;
use std::time::Duration;
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

fn main() -> Result<()> {
    let opt: Opt = Opt::from_args();

    let upload_token = UploadPolicy::new_for_bucket(&opt.bucket_name, Duration::from_secs(opt.lifetime))
        .build_token(Credential::new(opt.access_key, opt.secret_key), Default::default());
    println!("{}", upload_token);
    Ok(())
}
