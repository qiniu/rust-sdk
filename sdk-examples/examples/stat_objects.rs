use anyhow::Result;
use async_std::io::stdin;
use futures::{io::BufReader, AsyncBufReadExt, TryStreamExt};
use qiniu_sdk::objects::{apis::credential::Credential, ObjectsManager, OperationProvider};
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
}

#[async_std::main]
async fn main() -> Result<()> {
    env_logger::init();

    let opt: Opt = Opt::from_args();

    let credential = Credential::new(&opt.access_key, &opt.secret_key);
    let object_manager = ObjectsManager::new(credential);
    let bucket = object_manager.bucket(opt.bucket_name);
    let object_names = BufReader::new(stdin()).lines().try_collect::<Vec<String>>().await?;
    let mut ops = bucket.batch_ops();
    ops.add_operations(
        object_names
            .iter()
            .map(|object_name| Box::new(bucket.stat_object(object_name.trim())) as Box<dyn OperationProvider>),
    );
    let mut stream = ops.async_call();
    loop {
        match stream.try_next().await {
            Ok(Some(object_info)) => {
                println!("{object_info:?}");
            }
            Err(err) => {
                println!("{err:?}");
            }
            Ok(None) => {
                break;
            }
        }
    }

    Ok(())
}
