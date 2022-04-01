use anyhow::Result;
use qiniu_apis::{
    credential::Credential,
    http_client::{BucketRegionsQueryer, RegionsProviderEndpoints},
    storage::get_bucket_taggings::QueryParams,
    Client,
};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "get-taggings")]
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

    let credential = Credential::new(opt.access_key, opt.secret_key);
    let region = BucketRegionsQueryer::new().query(credential.access_key().to_owned(), opt.bucket_name.to_owned());
    let tags = Client::default()
        .storage()
        .get_bucket_taggings()
        .new_async_request(RegionsProviderEndpoints::new(&region), credential)
        .query_pairs(QueryParams::default().set_bucket_name_as_str(opt.bucket_name))
        .call()
        .await?
        .into_body()
        .get_tags()
        .to_tag_info_vec();
    for tag in tags {
        println!("{}: {}", tag.get_key_as_str(), tag.get_value_as_str());
    }

    Ok(())
}
