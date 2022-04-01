use anyhow::Result;
use qiniu_apis::{
    credential::Credential,
    http_client::{AllRegionsProvider, RegionsProvider, RegionsProviderEndpoints},
    storage::create_bucket::PathParams,
    Client,
};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "create-bucket")]
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
    /// Qiniu Region ID
    #[structopt(long)]
    region_id: String,
}

#[async_std::main]
async fn main() -> Result<()> {
    env_logger::init();

    let opt: Opt = Opt::from_args();

    let credential = Credential::new(opt.access_key, opt.secret_key);
    let region = AllRegionsProvider::new(credential.to_owned())
        .async_get(&Default::default())
        .await?;
    Client::default()
        .storage()
        .create_bucket()
        .new_async_request(
            RegionsProviderEndpoints::new(&region),
            PathParams::default()
                .set_bucket_as_str(opt.bucket_name)
                .set_region_as_str(opt.region_id),
            credential,
        )
        .call()
        .await?;
    Ok(())
}
