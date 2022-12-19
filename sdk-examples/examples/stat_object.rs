use anyhow::Result;
use qiniu_sdk::objects::{apis::credential::Credential, ObjectsManager};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "stat-object")]
struct Opt {
    /// Qiniu Access Key
    #[structopt(long)]
    access_key: String,
    /// Qiniu Secret Key
    #[structopt(long)]
    secret_key: String,
    /// Qiniu Bucket Name
    #[structopt(long)]
    bucket_name: String,
    /// Qiniu Object Name
    #[structopt(long)]
    object_name: String,
}

#[async_std::main]
async fn main() -> Result<()> {
    env_logger::init();

    let opt: Opt = Opt::from_args();

    let credential = Credential::new(&opt.access_key, &opt.secret_key);
    let object_manager = ObjectsManager::new(credential);
    let bucket = object_manager.bucket(opt.bucket_name);

    let response = bucket.stat_object(&opt.object_name).async_call().await?;
    let entry = response.into_body();
    println!("{entry:#?}");

    Ok(())
}
