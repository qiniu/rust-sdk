use anyhow::Result;
use async_std::io::stdin;
use qiniu_apis::credential::Credential;
use qiniu_upload_manager::{
    AlwaysMultiParts, AlwaysSinglePart, AutoUploader, AutoUploaderObjectParams, MultiPartsUploaderPrefer,
    MultiPartsUploaderWithCallbacks, UploadManager, UploadTokenSigner, UploadedPart, UploaderWithCallbacks,
    UploadingProgressInfo,
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
    upload_method: Option<UploadMethod>,
}

#[derive(Clone, Copy, Debug)]
enum UploadMethod {
    Default,
    Form,
    ResumableV1,
    ResumableV2,
}

impl FromStr for UploadMethod {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "default" => Ok(Self::Default),
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
    .build();

    let mut object_params_builder = {
        let mut builder = AutoUploaderObjectParams::builder();
        if let Some(object_name) = opt.object_name.take() {
            builder.object_name(object_name);
        }
        if let Some(file_name) = opt.file_name.take() {
            builder.file_name(file_name);
        }
        builder
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
        Ok(())
    };
    let part_uploaded = |part: &dyn UploadedPart| {
        println!("Uploaded Part: {}, is resumed: {}", part.offset(), part.resumed());
        Ok(())
    };

    let body = match opt.upload_method {
        None | Some(UploadMethod::Default) => {
            let mut uploader: AutoUploader = upload_manager.auto_uploader();
            let object_params = object_params_builder.build();
            uploader
                .on_upload_progress(upload_progress)
                .on_part_uploaded(part_uploaded);
            if let Some(local_file) = opt.local_file.as_ref() {
                uploader.async_upload_path(local_file, object_params).await?
            } else {
                uploader.async_upload_reader(stdin(), object_params).await?
            }
        }
        Some(UploadMethod::Form) => {
            let mut uploader: AutoUploader = upload_manager
                .auto_uploader_builder()
                .resumable_policy_provider(AlwaysSinglePart)
                .build();
            let object_params = object_params_builder.build();
            uploader.on_upload_progress(upload_progress);
            if let Some(local_file) = opt.local_file.as_ref() {
                uploader.async_upload_path(local_file, object_params).await?
            } else {
                uploader.async_upload_reader(stdin(), object_params).await?
            }
        }
        Some(UploadMethod::ResumableV1) => {
            let mut uploader: AutoUploader = upload_manager
                .auto_uploader_builder()
                .resumable_policy_provider(AlwaysMultiParts)
                .build();
            uploader
                .on_upload_progress(upload_progress)
                .on_part_uploaded(part_uploaded);
            let object_params = object_params_builder
                .multi_parts_uploader_prefer(MultiPartsUploaderPrefer::V1)
                .build();
            if let Some(local_file) = opt.local_file.as_ref() {
                uploader.async_upload_path(local_file, object_params).await?
            } else {
                uploader.async_upload_reader(stdin(), object_params).await?
            }
        }
        Some(UploadMethod::ResumableV2) => {
            let mut uploader: AutoUploader = upload_manager
                .auto_uploader_builder()
                .resumable_policy_provider(AlwaysMultiParts)
                .build();
            uploader
                .on_upload_progress(upload_progress)
                .on_part_uploaded(part_uploaded);
            let object_params = object_params_builder
                .multi_parts_uploader_prefer(MultiPartsUploaderPrefer::V2)
                .build();
            if let Some(local_file) = opt.local_file.as_ref() {
                uploader.async_upload_path(local_file, object_params).await?
            } else {
                uploader.async_upload_reader(stdin(), object_params).await?
            }
        }
    };
    println!("{:?}", body);
    Ok(())
}
