use anyhow::Result;
use futures::TryStreamExt;
use qiniu_sdk::objects::{apis::credential::Credential, ListVersion, ObjectsManager};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "get-objects")]
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
    /// Qiniu List API Version
    #[structopt(long, default_value = "2")]
    version: u8,
    /// Qiniu Object Name Prefix
    #[structopt(long)]
    prefix: String,
}

#[async_std::main]
async fn main() -> Result<()> {
    env_logger::init();

    let opt: Opt = Opt::from_args();

    let version = match opt.version {
        1 => ListVersion::V1,
        2 => ListVersion::V2,
        _ => panic!("Unrecognized version"),
    };

    let credential = Credential::new(&opt.access_key, &opt.secret_key);
    let object_manager = ObjectsManager::new(credential);
    let bucket = object_manager.bucket(opt.bucket_name);
    let mut stream = bucket.list().version(version).prefix(opt.prefix).stream();

    while let Some(entry) = stream.try_next().await? {
        println!(
            "{}\n  hash: {}\n  size: {}\n  mime type: {}",
            entry.get_key_as_str(),
            entry.get_hash_as_str(),
            entry.get_size_as_u64(),
            entry.get_mime_type_as_str(),
        );
    }
    Ok(())
}
