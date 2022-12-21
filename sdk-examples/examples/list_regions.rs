use anyhow::Result;
use qiniu_sdk::apis::{
    credential::Credential,
    http_client::{AllRegionsProvider, RegionsProvider},
};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "list-regions")]
struct Opt {
    /// Qiniu Access Key
    #[structopt(long)]
    access_key: String,
    /// Qiniu Secret Key
    #[structopt(long)]
    secret_key: String,
}

#[async_std::main]
async fn main() -> Result<()> {
    env_logger::init();

    let opt: Opt = Opt::from_args();
    let credential = Credential::new(opt.access_key, opt.secret_key);

    let regions = AllRegionsProvider::new(credential)
        .async_get_all(Default::default())
        .await?;
    println!("{regions:#?}");

    Ok(())
}
