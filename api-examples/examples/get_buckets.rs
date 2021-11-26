use anyhow::Result;
use qiniu_apis::{
    credential::Credential,
    http_client::{CachedRegionsProvider, RegionProvider},
    Client,
};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "get-buckets")]
struct Opt {
    /// Qiniu Access Key
    #[structopt(short, long)]
    access_key: String,
    /// Qiniu Secret Key
    #[structopt(short, long)]
    secret_key: String,
}

#[async_std::main]
async fn main() -> Result<()> {
    env_logger::init();

    let opt: Opt = Opt::from_args();

    let credential = Box::new(Credential::new(opt.access_key, opt.secret_key));
    let region = CachedRegionsProvider::new(credential.to_owned())
        .async_get(&Default::default())
        .await?
        .into_region();
    let response = Client::default()
        .storage()
        .get_buckets()
        .new_async_request(&region, credential)
        .call()
        .await?;
    for bucket_name in response.body().to_str_vec().into_iter() {
        println!("{}", bucket_name);
    }

    Ok(())
}
