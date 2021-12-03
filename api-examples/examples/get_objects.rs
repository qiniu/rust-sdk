use anyhow::Result;
use qiniu_apis::{
    credential::Credential, http_client::BucketRegionsQueryer, storage::get_objects::QueryParams,
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
    /// Qiniu Bucket Name
    #[structopt(short, long)]
    bucket_name: String,
}

#[async_std::main]
async fn main() -> Result<()> {
    env_logger::init();

    let opt: Opt = Opt::from_args();

    let bucket_queryer = BucketRegionsQueryer::default();
    let credential = Credential::new(&opt.access_key, &opt.secret_key);
    let client = Client::default();
    let mut prev_marker = String::new();
    loop {
        let mut query_params = QueryParams::default().set_bucket_as_str(&opt.bucket_name);
        if !prev_marker.is_empty() {
            query_params = query_params.set_marker_as_str(&prev_marker);
        }
        let response = client
            .storage()
            .get_objects()
            .new_async_request(
                bucket_queryer.query(&opt.access_key, &opt.bucket_name),
                credential.to_owned(),
            )
            .query_pairs(query_params)
            .call()
            .await?;
        for entry in response
            .body()
            .get_items()
            .to_listed_object_entry_vec()
            .into_iter()
        {
            println!(
                "{}\n  hash: {}\n  size: {}\n  mime type: {}",
                entry.get_key_as_str(),
                entry.get_hash_as_str(),
                entry.get_size_as_u64(),
                entry.get_mime_type_as_str(),
            );
        }

        if let Some(marker) = response.body().get_marker_as_str() {
            prev_marker = marker.to_owned();
        } else {
            break;
        }
    }

    Ok(())
}
