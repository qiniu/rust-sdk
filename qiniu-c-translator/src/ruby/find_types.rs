//! 提前找出所有即将声明的类型，这样在之后如果碰到一个类型，就可以判断是已经定义过的类型，还是未知类型
//!
//! 注意，随机标识符的类型不会在这里声明，因此需要在实际声明类型的时候重新声明一次

use super::utils::{insert_type_constants, normalize_constant, ConstantType};
use crate::ast::{SourceFile, TypeDeclaration};
use tap::TapOptionOps;

pub(super) fn find_all_type_constants(source_file: &SourceFile) {
    find_predefined_type_constants();

    source_file
        .type_declarations()
        .iter()
        .for_each(|type_declaration| match type_declaration {
            TypeDeclaration::Enum(e) => {
                e.typedef_name()
                    .as_ref()
                    .or_else(|| e.enum_name().as_ref())
                    .map(normalize_constant)
                    .tap_some(|constant| {
                        insert_type_constants(constant.as_str(), ConstantType::Enum);
                    });
            }
            TypeDeclaration::Struct(s) => {
                s.typedef_name()
                    .as_ref()
                    .or_else(|| s.struct_name().as_ref())
                    .map(normalize_constant)
                    .tap_some(|constant| {
                        insert_type_constants(constant.as_str(), ConstantType::Struct);
                    });
            }
        });
}

fn find_predefined_type_constants() {
    insert_type_constants("In6Addr", ConstantType::Struct);
}
