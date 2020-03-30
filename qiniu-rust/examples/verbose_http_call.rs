use clap::{App, Arg};
use qiniu_http::{Request as HTTPRequest, Response as HTTPResponse, Result as HTTPResult};
use qiniu_ng::{
    http::{HTTPAfterAction, HTTPBeforeAction},
    storage::uploader::{BatchUploadJobBuilder, UploadManager, UploadPolicyBuilder},
    ConfigBuilder, Credential,
};
use rand::{thread_rng, Rng};
use std::{error::Error, ffi::c_void, mem::transmute, result::Result};

struct HTTPLogger {}

impl HTTPBeforeAction for HTTPLogger {
    fn before_call(&self, request: &mut HTTPRequest) -> HTTPResult<()> {
        let request_id: usize = thread_rng().gen();
        println!("[{}] {} {}", request_id, request.method().as_str(), request.url());
        for (header_name, header_value) in request.headers().iter() {
            println!("[{}]   {}: {}", request_id, header_name, header_value);
        }
        *request.custom_data_mut() = request_id as *mut c_void;
        Ok(())
    }
}

impl HTTPAfterAction for HTTPLogger {
    fn after_call(&self, request: &mut HTTPRequest, response: &mut HTTPResponse) -> HTTPResult<()> {
        let request_id: usize = unsafe { transmute(request.custom_data()) };
        println!("[{}] {}", request_id, response.status_code());
        for (header_name, header_value) in response.headers().iter() {
            println!("[{}]   {}: {}", request_id, header_name, header_value);
        }
        Ok(())
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = App::new("Verbose HTTP Call")
        .about("How to write callback functions for http request")
        .arg(
            Arg::with_name("access_key")
                .long("access-key")
                .help("Qiniu Access Key")
                .required(true)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("secret_key")
                .long("secret-key")
                .help("Qiniu Secret Key")
                .required(true)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("bucket")
                .long("bucket")
                .help("Qiniu Bucket name")
                .required(true)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("FILES")
                .help("Files to upload")
                .required(true)
                .multiple(true),
        )
        .get_matches();
    let credential = Credential::new(
        args.value_of("access_key").unwrap().to_owned(),
        args.value_of("secret_key").unwrap().to_owned(),
    );
    let config = ConfigBuilder::default()
        .append_http_request_before_action_handler(HTTPLogger {})
        .append_http_request_after_action_handler(HTTPLogger {})
        .build();
    let upload_policy = UploadPolicyBuilder::new_policy_for_bucket(args.value_of("bucket").unwrap(), &config).build();
    let mut batch_uploader = UploadManager::new(config).batch_uploader_for_upload_policy(upload_policy, credential)?;
    for file_path in args.values_of_os("FILES").unwrap() {
        let file_path_str = file_path.to_string_lossy().into_owned();
        batch_uploader.push_job(
            BatchUploadJobBuilder::default()
                .on_completed(move |result| {
                    if let Err(err) = result {
                        eprintln!("Upload {} failed: {:?}", file_path_str, err);
                    } else {
                        println!("Upload {} succeed", file_path_str);
                    }
                })
                .upload_file(file_path, "", None)?,
        );
    }
    batch_uploader.start();
    Ok(())
}
