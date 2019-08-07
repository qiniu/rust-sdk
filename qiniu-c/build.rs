use cbindgen::{Config, Language};
use std::env;

fn main() {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let mut config: Config = Default::default();
    config.language = Language::C;
    config.documentation = true;

    cbindgen::generate_with_config(&crate_dir, config)
        .expect("Unable to generate bindings")
        .write_to_file("libqiniu_ng.h");

    let mut config: Config = Default::default();
    config.language = Language::Cxx;
    config.documentation = true;
    config.namespace = Some("qiniu_ng".to_owned());
    cbindgen::generate_with_config(&crate_dir, config)
        .expect("Unable to generate bindings")
        .write_to_file("libqiniu_ng.hpp");
}
