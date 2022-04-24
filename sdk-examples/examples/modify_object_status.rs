use anyhow::Result;
use qiniu_sdk::objects::{apis::credential::Credential, ObjectsManager};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "modify-object-status")]
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
    /// Disable
    #[structopt(long)]
    disable: usize,
}

#[async_std::main]
async fn main() -> Result<()> {
    env_logger::init();

    let opt: Opt = Opt::from_args();

    let credential = Credential::new(&opt.access_key, &opt.secret_key);
    let object_manager = ObjectsManager::new(credential);
    let bucket = object_manager.bucket(opt.bucket_name);

    bucket
        .modify_object_status(&opt.object_name, opt.disable > 0)
        .async_call()
        .await?;

    Ok(())
}
