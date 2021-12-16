use anyhow::Result;
use qiniu_objects_manager::{apis::credential::Credential, ObjectsManager};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "move-object")]
struct Opt {
    /// Qiniu Access Key
    #[structopt(long)]
    access_key: String,
    /// Qiniu Secret Key
    #[structopt(long)]
    secret_key: String,
    /// From Qiniu Bucket Name
    #[structopt(long)]
    from_bucket_name: String,
    /// From Qiniu Object Name
    #[structopt(long)]
    from_object_name: String,
    /// To Qiniu Bucket Name
    #[structopt(long)]
    to_bucket_name: String,
    /// To Qiniu Object Name
    #[structopt(long)]
    to_object_name: String,
}

#[async_std::main]
async fn main() -> Result<()> {
    env_logger::init();

    let opt: Opt = Opt::from_args();

    let credential = Credential::new(&opt.access_key, &opt.secret_key);
    let object_manager = ObjectsManager::builder(credential).build();
    let bucket = object_manager.bucket(opt.from_bucket_name);

    bucket
        .move_object_to(
            &opt.from_object_name,
            &opt.to_bucket_name,
            &opt.to_object_name,
        )
        .async_call()
        .await?;

    Ok(())
}
