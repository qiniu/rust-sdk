use anyhow::Result;
use async_std::io::stdin;
use qiniu_apis::{
    credential::Credential,
    http_client::{CallbackResult, HttpClient},
};
use qiniu_upload_manager::{
    ConcurrentMultiPartsUploaderScheduler, FileSystemResumableRecorder, MultiPartsUploaderScheduler,
    MultiPartsUploaderSchedulerExt, ObjectParams, SinglePartUploader, UploadManager, UploadTokenSigner,
    UploaderWithCallbacks, UploadingProgressInfo,
};
use std::{path::PathBuf, str::FromStr, time::Duration};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "upload")]
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
    /// Qiniu Object Name
    #[structopt(short, long)]
    object_name: Option<String>,
    /// Qiniu File Name
    #[structopt(short, long)]
    file_name: Option<String>,
    /// Upload File
    #[structopt(short, long, parse(from_os_str))]
    local_file: Option<PathBuf>,
    /// Upload method
    #[structopt(short, long)]
    upload_method: UploadMethod,
}

#[derive(Clone, Copy, Debug)]
enum UploadMethod {
    Form,
    ResumableV1,
    ResumableV2,
}

impl FromStr for UploadMethod {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "form" => Ok(Self::Form),
            "v1" | "resumable_v1" => Ok(Self::ResumableV1),
            "v2" | "resumable_v2" => Ok(Self::ResumableV2),
            s => Err(s.to_owned()),
        }
    }
}

#[async_std::main]
async fn main() -> Result<()> {
    let mut opt: Opt = Opt::from_args();

    let upload_manager = UploadManager::builder(UploadTokenSigner::new_credential_provider(
        Credential::new(opt.access_key, opt.secret_key),
        opt.bucket_name,
        Duration::from_secs(3600),
    ))
    .http_client({
        let mut builder = HttpClient::build_isahc()?;
        builder.use_https(false);
        builder.build()
    })
    .build();

    let object_params = {
        let mut builder = ObjectParams::builder();
        if let Some(object_name) = opt.object_name.take() {
            builder.object_name(object_name);
        }
        if let Some(file_name) = opt.file_name.take() {
            builder.file_name(file_name);
        }
        builder.build()
    };

    let upload_progress = |transfer: &UploadingProgressInfo| {
        if let Some(total_size) = transfer.total_bytes() {
            println!(
                "Progress: {} / {} = {}%",
                transfer.transferred_bytes(),
                total_size,
                transfer.transferred_bytes() * 100 / total_size
            );
        } else {
            println!("Progress: {}", transfer.transferred_bytes());
        }
        CallbackResult::Continue
    };

    let body = match opt.upload_method {
        UploadMethod::Form => {
            let mut uploader = upload_manager.form_uploader();
            uploader.on_upload_progress(upload_progress);
            if let Some(local_file) = opt.local_file.as_ref() {
                uploader.async_upload_path(local_file, object_params).await?
            } else {
                uploader.async_upload_reader(stdin(), object_params).await?
            }
        }
        UploadMethod::ResumableV1 => {
            let mut uploader = upload_manager.multi_parts_v1_uploader(FileSystemResumableRecorder::default());
            uploader.on_upload_progress(upload_progress);
            let scheduler = ConcurrentMultiPartsUploaderScheduler::new(uploader);
            if let Some(local_file) = opt.local_file.as_ref() {
                scheduler.async_upload_path(local_file, object_params).await?
            } else {
                scheduler.async_upload_reader(stdin(), object_params).await?
            }
        }
        UploadMethod::ResumableV2 => {
            let mut uploader = upload_manager.multi_parts_v2_uploader(FileSystemResumableRecorder::default());
            uploader.on_upload_progress(upload_progress);
            let scheduler = ConcurrentMultiPartsUploaderScheduler::new(uploader);
            if let Some(local_file) = opt.local_file.as_ref() {
                scheduler.async_upload_path(local_file, object_params).await?
            } else {
                scheduler.async_upload_reader(stdin(), object_params).await?
            }
        }
    };
    println!("{:?}", body);
    Ok(())
}
