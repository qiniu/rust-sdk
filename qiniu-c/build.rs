use cbindgen::{Config, ItemType, Language};
use std::env;

fn main() {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();

    let mut config: Config = Default::default();
    config.documentation = true;
    config.sys_includes.push("curl/curl.h".to_string());
    config.export.item_types = vec![
        ItemType::Constants,
        ItemType::Globals,
        ItemType::Enums,
        ItemType::Structs,
        ItemType::Unions,
        ItemType::OpaqueItems,
        ItemType::Functions,
    ];
    config.include_guard = Some("__QINIU_NG_H".to_string());
    config.language = Language::C;
    config.cpp_compat = true;
    cbindgen::generate_with_config(&crate_dir, config)
        .expect("Unable to generate bindings")
        .write_to_file("libqiniu_ng.h");
}
