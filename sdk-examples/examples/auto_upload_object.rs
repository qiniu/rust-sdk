use anyhow::Result;
use qiniu_sdk::upload::{
    apis::{credential::Credential, upload_token::ObjectUploadTokenProvider},
    AutoUploader, AutoUploaderObjectParams, FixedConcurrencyProvider, FixedDataPartitionProvider,
    MultiPartsUploaderPrefer, MultiPartsUploaderSchedulerPrefer, UploadManager, UploaderWithCallbacks,
};
use std::{
    num::{NonZeroU64, NonZeroUsize},
    path::PathBuf,
    time::Duration,
};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "auto-upload-object")]
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
    /// Upload File Path
    #[structopt(long)]
    file: PathBuf,
    /// Upload Workers
    #[structopt(long)]
    workers: Option<usize>,
    /// Upload Part Size
    #[structopt(long)]
    part_size: Option<u64>,
    /// Multi Parts Uploader Version
    #[structopt(long)]
    version: Option<usize>,
}

#[async_std::main]
async fn main() -> Result<()> {
    env_logger::init();

    let opt: Opt = Opt::from_args();
    let credential = Credential::new(opt.access_key, opt.secret_key);
    let upload_manager = UploadManager::new(ObjectUploadTokenProvider::new(
        &opt.bucket_name,
        &opt.object_name,
        Duration::from_secs(3600),
        credential,
    ));
    let mut auto_uploader: AutoUploader = {
        let mut builder = AutoUploader::builder(upload_manager);
        if let Some(workers) = opt.workers.and_then(NonZeroUsize::new) {
            builder.concurrency_provider(FixedConcurrencyProvider::new_with_non_zero_concurrency(workers));
        }
        if let Some(part_size) = opt.part_size.and_then(NonZeroU64::new) {
            builder.data_partition_provider(FixedDataPartitionProvider::new_with_non_zero_part_size(part_size));
        }
        builder.build()
    };
    auto_uploader.on_upload_progress(|transfer| {
        let transferred_bytes = transfer.transferred_bytes();
        if let Some(total_bytes) = transfer.total_bytes() {
            println!(
                "{} / {} => {}%",
                transferred_bytes,
                total_bytes,
                transferred_bytes as f64 * 100f64 / total_bytes as f64
            );
        } else {
            println!("{}", transferred_bytes);
        }
        Ok(())
    });
    let params = {
        let mut builder = AutoUploaderObjectParams::builder();
        let scheduler = match opt.workers {
            Some(workers) if workers >= 2 => MultiPartsUploaderSchedulerPrefer::Concurrent,
            _ => MultiPartsUploaderSchedulerPrefer::Serial,
        };
        match opt.version {
            Some(1) => {
                builder.multi_parts_uploader_prefer(MultiPartsUploaderPrefer::V1);
            }
            Some(2) => {
                builder.multi_parts_uploader_prefer(MultiPartsUploaderPrefer::V2);
            }
            Some(version) => panic!("Unrecognized version: {}", version),
            None => {}
        };
        builder
            .multi_parts_uploader_scheduler_prefer(scheduler)
            .object_name(&opt.object_name);
        builder.build()
    };
    let value = auto_uploader.async_upload_path(&opt.file, params).await?;
    println!("{:?}", value);

    Ok(())
}
