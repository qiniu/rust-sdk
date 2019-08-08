use cbindgen::{Config, ItemType, Language};
use std::env;

fn main() {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();

    let mut config = default_cbindgen_config();
    config.language = Language::C;
    cbindgen::generate_with_config(&crate_dir, config)
        .expect("Unable to generate bindings")
        .write_to_file("libqiniu_ng.h");

    let mut config = default_cbindgen_config();
    config.language = Language::Cxx;
    cbindgen::generate_with_config(&crate_dir, config)
        .expect("Unable to generate bindings")
        .write_to_file("libqiniu_ng.hpp");
}

fn default_cbindgen_config() -> Config {
    let mut config: Config = Default::default();
    config.documentation = true;
    config.namespace = Some("qiniu_ng".to_owned());
    config.export.item_types = vec![
        ItemType::Constants,
        ItemType::Globals,
        ItemType::Enums,
        ItemType::Structs,
        ItemType::Unions,
        ItemType::OpaqueItems,
        ItemType::Functions,
    ];
    config
}
