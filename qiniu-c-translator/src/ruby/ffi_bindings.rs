use super::ast::{Module, RawCode};

pub(super) fn insert_ffi_bindings(version_constant: &str, module: &mut Module) {
    module
        .sub_nodes_mut()
        .push(Box::new(RawCode::new("extend FFI::Library")));
    module.sub_nodes_mut().push(Box::new(RawCode::new(
        "DEFAULT_TARGET_DIR = ".to_owned() + "File.expand_path(File.join('..', '..', '..', 'target'), __dir__)",
    )));
    module
        .sub_nodes_mut()
        .push(Box::new(RawCode::new("private_constant :DEFAULT_TARGET_DIR")));
    module.sub_nodes_mut().push(Box::new(RawCode::new(
        "ffi_lib [".to_owned()
            + &format!("\"qiniu_ng_c-#{{{}}}\", ", version_constant)
            + "'qiniu_ng_c', "
            + &format!("File.expand_path(File.join(DEFAULT_TARGET_DIR, 'release', \"#{{FFI::Platform::LIBPREFIX}}qiniu_ng_c-#{{{}}}.#{{FFI::Platform::LIBSUFFIX}}\"), __dir__), ", version_constant)
            + "File.expand_path(File.join(DEFAULT_TARGET_DIR, 'release', \"#{FFI::Platform::LIBPREFIX}qiniu_ng_c.#{FFI::Platform::LIBSUFFIX}\"), __dir__), "
            + &format!("File.expand_path(File.join(DEFAULT_TARGET_DIR, 'debug', \"#{{FFI::Platform::LIBPREFIX}}qiniu_ng_c-#{{{}}}.#{{FFI::Platform::LIBSUFFIX}}\"), __dir__), ", version_constant)
            + "File.expand_path(File.join(DEFAULT_TARGET_DIR, 'debug', \"#{FFI::Platform::LIBPREFIX}qiniu_ng_c.#{FFI::Platform::LIBSUFFIX}\"), __dir__), "
            + &format!("File.expand_path(File.join('..', '..', 'ext', \"#{{FFI::Platform::LIBPREFIX}}qiniu_ng_c-#{{{}}}.#{{FFI::Platform::LIBSUFFIX}}\"), __dir__), ", version_constant)
            + "File.expand_path(File.join('..', '..', 'ext', \"#{FFI::Platform::LIBPREFIX}qiniu_ng_c.#{FFI::Platform::LIBSUFFIX}\"), __dir__), "
            + "]"
    )));
}
