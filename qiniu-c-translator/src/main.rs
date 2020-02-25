use clang::{Clang, Index};
use clap::{App, Arg, SubCommand};
use regex::Regex;
use std::{
    fs::OpenOptions,
    io::{stdout, Result, Write},
};
use tap::TapOps;

mod ast;
mod classifier;
mod dump_entity;
mod ruby;
mod utils;
use ast::{dump_ast, SourceFile};
use classifier::{dump_classifier, Class, Classifier};
use dump_entity::dump_entity;
use ruby::GenerateBindings as GenerateRubyBindings;

fn main() -> Result<()> {
    let matches = App::new("Qiniu C Translator")
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS").split(':').last().unwrap())
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .arg(
            Arg::with_name("header-file")
                .long("header-file")
                .required(true)
                .value_name("FILE")
                .help("To generate bindings")
                .takes_value(true),
        )
        .subcommand(
            SubCommand::with_name("generate-ruby-bindings")
                .about("Generate Ruby bindings code")
                .arg(
                    Arg::with_name("output")
                        .long("output")
                        .value_name("FILE")
                        .help("Output ruby code to file")
                        .takes_value(true),
                ),
        )
        .subcommand(
            SubCommand::with_name("dump-entity")
                .about("Show clang entities, only for debug")
                .arg(
                    Arg::with_name("pretty-print")
                        .long("pretty")
                        .help("Pretty-printed output"),
                ),
        )
        .subcommand(SubCommand::with_name("dump-classifier").about("Show classifier, only for debug"))
        .subcommand(
            SubCommand::with_name("dump-ast").about("Show ast, only for debug").arg(
                Arg::with_name("pretty-print")
                    .long("pretty")
                    .help("Pretty-printed output"),
            ),
        )
        .get_matches();
    let cl = Clang::new().unwrap();
    let idx = Index::new(&cl, true, false);
    let tu = {
        let header_file_path = matches.value_of_os("header-file").unwrap();
        if let Err(err) = OpenOptions::new().read(true).open(&header_file_path) {
            panic!("Failed to open header file: {}", err);
        }
        idx.parser(header_file_path).parse().unwrap()
    };
    let entity = tu.get_entity();
    match matches.subcommand() {
        ("generate-ruby-bindings", args) => {
            let source_file = SourceFile::parse(&entity);
            GenerateRubyBindings::default()
                // 这里目前是写死的模块路径，如果有需要可以改为参数配置
                .module_names(["QiniuNg".into(), "Bindings".into()])
                .version_constant("QiniuNg::VERSION")
                .build(
                    &source_file,
                    make_classifier(&source_file),
                    &mut args
                        .and_then(|args| args.value_of_os("output"))
                        .and_then(|path| if path == "-" { None } else { Some(path) })
                        .map(|file_path| {
                            let output: Box<dyn Write> = Box::new(
                                OpenOptions::new()
                                    .write(true)
                                    .truncate(true)
                                    .create(true)
                                    .open(file_path)?,
                            );
                            Ok(output) as Result<Box<dyn Write>>
                        })
                        .unwrap_or_else(|| Ok(Box::new(stdout())))?,
                )?;
        }
        ("dump-entity", args) => dump_entity(
            &entity,
            args.map(|args| args.is_present("pretty-print")).unwrap_or(false),
        )?,

        ("dump-ast", args) => dump_ast(
            &entity,
            args.map(|args| args.is_present("pretty-print")).unwrap_or(false),
        ),
        ("dump-classifier", _) => dump_classifier(&make_classifier(&SourceFile::parse(&entity)))?,

        ("", _) => {}
        (subcommand, _) => panic!("Unrecognized subcommand: {}", subcommand),
    }

    Ok(())
}

fn make_classifier(source_file: &SourceFile) -> Classifier {
    Classifier::default().tap(|classifier| {
        classifier.add_class(Class::new(
            "Str",
            Some("qiniu_ng_str_t"),
            Regex::new("^qiniu_ng_str_(\\w+)").unwrap(),
            Some(Regex::new("^qiniu_ng_str_(list|map)_").unwrap()),
            source_file.function_declarations().iter(),
            None,
            vec![],
        ));
        classifier.add_class(Class::new(
            "StrList",
            Some("qiniu_ng_str_list_t"),
            Regex::new("^qiniu_ng_str_list_(\\w+)").unwrap(),
            None,
            source_file.function_declarations().iter(),
            None,
            vec![],
        ));
        classifier.add_class(Class::new(
            "StrMap",
            Some("qiniu_ng_str_map_t"),
            Regex::new("^qiniu_ng_str_map_(\\w+)").unwrap(),
            None,
            source_file.function_declarations().iter(),
            None,
            vec![],
        ));
        classifier.add_class(Class::new(
            "Etag",
            Some("qiniu_ng_etag_t"),
            Regex::new("^qiniu_ng_etag_(\\w+)").unwrap(),
            None,
            source_file.function_declarations().iter(),
            None,
            vec![],
        ));
        classifier.add_class(Class::new(
            "ConfigBuilder",
            Some("qiniu_ng_config_builder_t"),
            Regex::new("^qiniu_ng_config_builder_(\\w+)").unwrap(),
            None,
            source_file.function_declarations().iter(),
            None,
            vec![],
        ));
        classifier.add_class(Class::new(
            "Config",
            Some("qiniu_ng_config_t"),
            Regex::new("^qiniu_ng_config_(\\w+)").unwrap(),
            Some(Regex::new("^qiniu_ng_config_builder_(\\w+)").unwrap()),
            source_file.function_declarations().iter(),
            None,
            vec![("qiniu_ng_config_build", "builder_ptr")],
        ));
        classifier.add_class(Class::new(
            "Region",
            Some("qiniu_ng_region_t"),
            Regex::new("^qiniu_ng_region_(\\w+)").unwrap(),
            None,
            source_file.function_declarations().iter(),
            None,
            vec![],
        ));
        classifier.add_class(Class::new(
            "Regions",
            Some("qiniu_ng_regions_t"),
            Regex::new("^qiniu_ng_regions_(\\w+)").unwrap(),
            None,
            source_file.function_declarations().iter(),
            None,
            vec![],
        ));
        classifier.add_class(Class::new(
            "Client",
            Some("qiniu_ng_client_t"),
            Regex::new("^qiniu_ng_client_(\\w+)").unwrap(),
            None,
            source_file.function_declarations().iter(),
            None,
            vec![],
        ));
        classifier.add_class(Class::new(
            "Storage",
            None,
            Regex::new("^qiniu_ng_storage_(\\w+)").unwrap(),
            None,
            source_file.function_declarations().iter(),
            None,
            vec![],
        ));
        classifier.add_class(Class::new(
            "Bucket",
            Some("qiniu_ng_bucket_t"),
            Regex::new("^qiniu_ng_bucket_(\\w+)").unwrap(),
            Some(Regex::new("^qiniu_ng_bucket_(uploader|builder)_(\\w+)").unwrap()),
            source_file.function_declarations().iter(),
            None,
            vec![],
        ));
        classifier.add_class(Class::new(
            "BucketBuilder",
            Some("qiniu_ng_bucket_builder_t"),
            Regex::new("^qiniu_ng_bucket_builder_(\\w+)").unwrap(),
            None,
            source_file.function_declarations().iter(),
            None,
            vec![],
        ));
        classifier.add_class(Class::new(
            "UploadPolicyBuilder",
            Some("qiniu_ng_upload_policy_builder_t"),
            Regex::new("^qiniu_ng_upload_policy_builder_(\\w+)").unwrap(),
            None,
            source_file.function_declarations().iter(),
            None,
            vec![],
        ));
        classifier.add_class(Class::new(
            "UploadPolicy",
            Some("qiniu_ng_upload_policy_t"),
            Regex::new("^qiniu_ng_upload_policy_(\\w+)").unwrap(),
            Some(Regex::new("^qiniu_ng_upload_policy_builder_(\\w+)").unwrap()),
            source_file.function_declarations().iter(),
            None,
            vec![("qiniu_ng_upload_policy_build", "builder_ptr")],
        ));
        classifier.add_class(Class::new(
            "UploadToken",
            Some("qiniu_ng_upload_token_t"),
            Regex::new("^qiniu_ng_upload_token_(\\w+)").unwrap(),
            None,
            source_file.function_declarations().iter(),
            None,
            vec![("qiniu_ng_upload_token_new_from_policy_builder", "policy_builder_ptr")],
        ));
        classifier.add_class(Class::new(
            "UploadManager",
            Some("qiniu_ng_upload_manager_t"),
            Regex::new("^qiniu_ng_upload_manager_(\\w+)").unwrap(),
            None,
            source_file.function_declarations().iter(),
            None,
            vec![],
        ));
        classifier.add_class(Class::new(
            "BucketUploader",
            Some("qiniu_ng_bucket_uploader_t"),
            Regex::new("^qiniu_ng_bucket_uploader_(\\w+)").unwrap(),
            None,
            source_file.function_declarations().iter(),
            None,
            vec![
                ("qiniu_ng_bucket_uploader_upload_file", "file"),
                ("qiniu_ng_bucket_uploader_upload_file", "params"),
                ("qiniu_ng_bucket_uploader_upload_file_path", "params"),
                ("qiniu_ng_bucket_uploader_upload_reader", "params"),
            ],
        ));
        classifier.add_class(Class::new(
            "UploadResponse",
            Some("qiniu_ng_upload_response_t"),
            Regex::new("^qiniu_ng_upload_response_(\\w+)").unwrap(),
            None,
            source_file.function_declarations().iter(),
            None,
            vec![],
        ));
    })
}
